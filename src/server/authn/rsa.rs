use anyhow::Result;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;

/// Generates a new RSA key pair for token signing and verification
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
/// use crate::server::authn::rsa::generate_rsa_keys;
///
/// // Generate a new RSA key pair
/// let (public_key, private_key) = generate_rsa_keys().unwrap();
///
/// // Keys are in PEM format and can be written to files
/// std::fs::write("public_key.pem", &public_key).unwrap();
/// std::fs::write("private_key.pem", &private_key).unwrap();
/// ```
pub fn generate_rsa_keys() -> Result<(Vec<u8>, Vec<u8>)> {
    let rsa = Rsa::generate(2048)?;
    let pkey = PKey::from_rsa(rsa)?;

    let private_key = pkey.private_key_to_pem_pkcs8()?;
    let public_key = pkey.public_key_to_pem()?;

    Ok((public_key, private_key))
}
