use std::{fs::File, io::BufReader, path::{Path, PathBuf}};
use tokio_rustls::rustls::pki_types::PrivateKeyDer;

/// Load raw private key data from a PEM file
pub fn load_private_key(path: impl AsRef<Path>) -> std::io::Result<PrivateKeyDer<'static>> {
    let path: PathBuf = path.as_ref().into();
    println!("Attempting to read private key from {path:?}");
    let file = File::open(path.clone())?;
    let mut reader = BufReader::new(file);

    if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
        if let Some(key) = keys.first() {
            return Ok(PrivateKeyDer::Pkcs8(key.clone().into()));
        }
    }

    let file = File::open(path.clone())?;
    let mut reader = BufReader::new(file);
    if let Ok(keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
        if let Some(key) = keys.first() {
            return Ok(PrivateKeyDer::Pkcs8(key.clone().into()));
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid private key"))
}

