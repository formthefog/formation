use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs::File, path::PathBuf};
use std::io::BufReader;
use rustls_pemfile::certs;
use tokio_rustls_acme::tokio_rustls::rustls::crypto::ring::sign::any_supported_type;
use tokio_rustls_acme::tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls_acme::tokio_rustls::rustls::server::{ClientHello, ResolvesServerCert};
use tokio_rustls_acme::tokio_rustls::rustls::sign::CertifiedKey;

use crate::keys::load_private_key;

pub const CERT_OUTPUT_DIR: &str = ".config/formation/certs";

#[derive(Default)]
pub struct ChallengeMap {
    tokens: Mutex<HashMap<String, String>>
}

impl ChallengeMap {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new())
        }
    }

    pub fn insert(&self, token: String, proof: String) {
        if let Ok(mut map) = self.tokens.lock() {
            map.insert(token, proof);
        }
    }

    pub fn remove(&self, token: &str) {
        if let Ok(mut map) = self.tokens.lock() {
            map.remove(token);
        }
    }

    pub fn get(&self, token: &str) -> Option<String> {
        if let Ok(map) = self.tokens.lock() {
            return map.get(token).cloned()
        }

        None
    }
}

#[derive(Debug)]
pub struct FormDomainCert {
    certificates: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static> 
}

impl FormDomainCert {
    pub fn new(cert: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> Self {
        Self { certificates: cert, key }
    }

    pub fn mkcert_dev(domain: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(mkcert(domain)?)
    }

    pub fn certificates(&self) -> &Vec<CertificateDer<'static>> {
        &self.certificates
    }
    
    pub fn key(&self) -> &PrivateKeyDer<'static> {
        &self.key
    }
}

#[derive(Debug)]
pub struct FormSniResolver {
    pub domain_map: Mutex<BTreeMap<String, FormDomainCert>>
}

impl FormSniResolver {
    pub fn new() -> Self {
        Self { domain_map: Mutex::new(BTreeMap::new()) }
    }
    //Insert
    //Remove
    //Refresh
    //Get
}

impl ResolvesServerCert for FormDomainCert {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(Arc::new(
            CertifiedKey::new(
                self.certificates().clone(),
                any_supported_type(self.key()).ok()?
            )
        ))
    }
}

impl ResolvesServerCert for FormSniResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?;
        if let Ok(guard) = self.domain_map.lock() {
            if let Some(domain_cert) = guard.get(sni) {
                return domain_cert.resolve(client_hello)
            }
        }
        None
    }
}

pub fn find_cert_file(acme_dir: &Path, domain: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let transformed_domain = domain.replace('.', "_");

    for entry in std::fs::read_dir(acme_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                if fname.contains("_crt_") && fname.contains(&transformed_domain) {
                    return Ok(path);
                }
            }
        }
    }

    Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Certificate not found for domain {domain}"))))
}

/// Load raw certificate data from a PEM file
pub fn load_certs(path: impl AsRef<Path>) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let path: PathBuf = path.as_ref().into();
    println!("Attempting to read cert from {path:?}");
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    certs(&mut reader)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid certificate"))
        .map(|certs| certs.into_iter().map(CertificateDer::from).collect())
}

pub fn mkcert(domain: &str) -> std::io::Result<FormDomainCert> {
    let home = std::env::var("HOME").unwrap_or(".".to_string());
    let cert_output = PathBuf::from(home).join(CERT_OUTPUT_DIR);
    log::info!("Attempting to create cert output dir: {}", cert_output.display());
    std::fs::create_dir_all(&cert_output)?;
    log::info!("created cert output dir: {}", cert_output.display());
    let output = std::process::Command::new("mkcert")
        .arg("-cert-file")
        .arg(cert_output.join(domain).with_extension("pem"))
        .arg("-key-file")
        .arg(cert_output.join(&format!("{domain}-key")).with_extension("pem"))
        .arg(domain)
        .output()?;


    if !output.status.success() {
        return Err(std::io::Error::last_os_error())
    }

    let certs = load_certs(cert_output.join(domain).with_extension("pem"))?;
    let key = load_private_key(cert_output.join(&format!("{domain}-key")).with_extension("pem"))?;
    Ok(FormDomainCert::new(certs, key))
}
