use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{Context, Result};
use log::{info, warn};

use crate::dirs;
use crate::server::authn::rsa;

use super::config::TokenConfig;
use super::jwt::JwtTokenizer;
use super::Tokenizer;

/// Factory for creating token authentication implementations
#[derive(Copy, Clone)]
pub struct TokenizerFactory;

impl TokenizerFactory {
    /// Creates a new TokenizerFactory instance
    pub fn new() -> Self {
        Self {}
    }

    /// Creates a simple tokenizer for testing purposes
    /// This should only be used in tests, not in production
    #[cfg(test)]
    pub fn build_simple_tokenizer(&self) -> Box<dyn Tokenizer> {
        use super::simple::SimpleTokenizer;
        Box::new(SimpleTokenizer::new())
    }

    /// Builds a production-ready JWT tokenizer based on the provided configuration
    ///
    /// RSA keys handling:
    /// 1. First tries to read existing RSA key pair from the PKI directory
    /// 2. If keys don't exist and auto-generation is enabled (default):
    ///   - Generates new RSA key pair
    ///   - Saves keys to PKI directory for persistence
    /// 3. If keys don't exist and auto-generation is disabled:
    ///   - Falls back to default test keys (UNSAFE for production)
    ///
    /// # Arguments
    /// * `cfg` - Token configuration containing PKI path and key generation settings
    ///
    /// # Returns
    /// * `Result<Box<dyn Tokenizer>>` - The configured tokenizer on success, or an error
    pub fn build_tokenizer(&self, cfg: &TokenConfig) -> Result<Box<dyn Tokenizer>> {
        let (pubkey, privkey) = Self::get_keys(cfg)?;
        Ok(Box::new(JwtTokenizer::new(&privkey, &pubkey)?))
    }

    fn get_keys(cfg: &TokenConfig) -> Result<(Vec<u8>, Vec<u8>)> {
        let keys_dir = PathBuf::from(&cfg.pki_path).join("tokens");
        let keys = Self::read_keys_from_file(&keys_dir)?;
        if let Some((pubkey, privkey)) = keys {
            return Ok((pubkey, privkey));
        }

        if cfg.no_generate_keys {
            warn!("No RSA keys provided for token authentication and auto-generation is disabled. Using default RSA keys - this is dangerous and tokens can be easily compromised. For testing only, DO NOT use in production!");
            let pubkey = include_bytes!("public_key.pem").to_vec();
            let privkey = include_bytes!("private_key.pem").to_vec();
            return Ok((pubkey, privkey));
        }

        // JWT algorithm requires keys for encryption. If user does not provide keys,
        // we need to generate a new key pair. The generated keys will be persisted
        // to the PKI directory so that token authentication remains valid after
        // server restart.
        info!(
            "No RSA keys provided for token authentication, generating new RSA keys to '{}'",
            keys_dir.display()
        );
        let (pubkey, privkey) = rsa::generate_rsa_keys().context("generate rsa keys for tokens")?;

        dirs::ensure_dir_exists(&keys_dir).context("ensure tokens pki dir")?;
        fs::write(keys_dir.join("public_key.pem"), &pubkey).context("write token public key")?;
        fs::write(keys_dir.join("private_key.pem"), &privkey).context("write token private key")?;

        Ok((pubkey, privkey))
    }

    fn read_keys_from_file(dir: &Path) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let pubkey_path = dir.join("public_key.pem");
        let privkey_path = dir.join("private_key.pem");

        let pubkey = match fs::read(pubkey_path) {
            Ok(key) => key,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).context("failed to read token public key file"),
        };

        let privkey = match fs::read(privkey_path) {
            Ok(key) => key,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).context("failed to read token public key file"),
        };

        Ok(Some((pubkey, privkey)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_tokenizer() {
        let cfg = TokenConfig {
            pki_path: "testdata/config/pki".into(),
            no_generate_keys: false,
            expiry: 3600,
        };

        // Create tokenizer using factory
        let tokenizer = TokenizerFactory::new().build_tokenizer(&cfg).unwrap();

        // Generate a test token
        let test_user = "test_user".to_string();
        let token = tokenizer.generate_token(test_user.clone(), 1).unwrap();

        // Validate the token
        let user = tokenizer.validate_token(&token).unwrap();
        assert_eq!(user, test_user);
    }

    #[test]
    fn test_get_keys() {
        // Clean up test directories before starting
        let _ = fs::remove_dir_all("_test_pki_empty");
        let _ = fs::remove_dir_all("_test_pki_no_generate");

        // Test case 1: Using existing keys from testdata/pki
        {
            let cfg = TokenConfig {
                pki_path: "testdata/config/pki".into(),
                no_generate_keys: false,
                expiry: 0,
            };
            let (pubkey, privkey) = TokenizerFactory::get_keys(&cfg).unwrap();
            assert!(!pubkey.is_empty());
            assert!(!privkey.is_empty());
        }

        // Test case 2: Generate new keys in empty directory
        {
            let cfg = TokenConfig {
                pki_path: "_test_pki_empty".into(),
                no_generate_keys: false,
                expiry: 0,
            };
            let (pubkey, privkey) = TokenizerFactory::get_keys(&cfg).unwrap();
            assert!(!pubkey.is_empty());
            assert!(!privkey.is_empty());

            // Verify keys were written to disk
            assert!(Path::new("_test_pki_empty/tokens/public_key.pem").exists());
            assert!(Path::new("_test_pki_empty/tokens/private_key.pem").exists());
        }

        // Test case 3: No keys with generation disabled
        {
            let cfg = TokenConfig {
                pki_path: "_test_pki_no_generate".into(),
                no_generate_keys: true,
                expiry: 0,
            };
            let (pubkey, privkey) = TokenizerFactory::get_keys(&cfg).unwrap();
            assert!(!pubkey.is_empty());
            assert!(!privkey.is_empty());

            // Verify no keys were written
            assert!(!Path::new("_test_pki_no_generate/tokens/public_key.pem").exists());
            assert!(!Path::new("_test_pki_no_generate/tokens/private_key.pem").exists());
        }

        // Clean up test directories after tests
        fs::remove_dir_all("_test_pki_empty").unwrap();

        // Verify _test_pki_no_generate is still empty/non-existent
        assert!(!Path::new("_test_pki_no_generate/tokens").exists());
    }
}
