use anyhow::Result;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;

/// Generates a new RSA key pair for signing and verification
///
/// This function generates:
/// - A 2048-bit RSA private key in PKCS#8 PEM format
/// - A corresponding public key in PEM format
///
/// # Returns
/// * `Result<(Vec<u8>, Vec<u8>)>` - A tuple containing the public and private keys as byte vectors
///
/// # Examples
///
/// ```
/// use csync_misc::rsa::generate_rsa_keys;
///
/// // Generate a new RSA key pair
/// let (public_key, private_key) = generate_rsa_keys().unwrap();
///
/// // Keys are in PEM format and can be written to files
/// std::fs::write("testdata/public_key.pem", &public_key).unwrap();
/// std::fs::write("testdata/private_key.pem", &private_key).unwrap();
/// ```
pub fn generate_rsa_keys() -> Result<(Vec<u8>, Vec<u8>)> {
    let rsa = Rsa::generate(2048)?;
    let pkey = PKey::from_rsa(rsa)?;

    let private_key = pkey.private_key_to_pem_pkcs8()?;
    let public_key = pkey.public_key_to_pem()?;

    Ok((public_key, private_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::pkey::PKey;

    #[test]
    fn test_generate_rsa_keys() {
        // Generate key pair
        let (public_key, private_key) = generate_rsa_keys().unwrap();

        // Test 1: Verify keys are not empty
        assert!(!public_key.is_empty());
        assert!(!private_key.is_empty());

        // Test 2: Verify PEM format
        let pub_str = String::from_utf8_lossy(&public_key);
        let priv_str = String::from_utf8_lossy(&private_key);

        assert!(pub_str.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(pub_str.ends_with("-----END PUBLIC KEY-----\n"));
        assert!(priv_str.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(priv_str.ends_with("-----END PRIVATE KEY-----\n"));

        // Test 3: Verify keys can be parsed by OpenSSL
        let public_key = PKey::public_key_from_pem(&public_key).unwrap();
        let private_key = PKey::private_key_from_pem(&private_key).unwrap();

        // Test 4: Verify key size is 2048 bits
        assert_eq!(public_key.size(), 256); // 2048 bits = 256 bytes
        assert_eq!(private_key.size(), 256);

        // Test 5: Verify keys form a valid pair by signing and verifying data
        use openssl::hash::MessageDigest;
        use openssl::sign::{Signer, Verifier};

        // Create test data
        let data = b"test message for signing";

        // Sign with private key
        let mut signer = Signer::new(MessageDigest::sha256(), &private_key).unwrap();
        signer.update(data).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        // Verify with public key
        let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key).unwrap();
        verifier.update(data).unwrap();
        assert!(verifier.verify(&signature).unwrap());
    }

    #[test]
    fn test_multiple_key_pairs_are_unique() {
        let (pub1, priv1) = generate_rsa_keys().unwrap();
        let (pub2, priv2) = generate_rsa_keys().unwrap();

        // Verify keys are different
        assert_ne!(pub1, pub2);
        assert_ne!(priv1, priv2);
    }
}
