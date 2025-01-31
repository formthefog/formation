use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs::File, path::PathBuf};
use std::io::BufReader;
use acme_lib::persist::FilePersist;
use acme_lib::{create_p384_key, Directory, DirectoryUrl, Error};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, server::Server, StatusCode};
use rustls_pemfile::certs;
use tokio_rustls::rustls::crypto::ring::sign::any_supported_type;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::server::{ClientHello, ResolvesServerCert};
use tokio_rustls::rustls::sign::CertifiedKey;

use crate::keys::load_private_key;

pub const CERT_OUTPUT_DIR: &str = ".config/formation/certs";

#[derive(Default)]
pub struct ChallengeMap {
    tokens: Mutex<HashMap<String, String>>
}

impl ChallengeMap {
    fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new())
        }
    }

    fn insert(&self, token: String, proof: String) {
        if let Ok(mut map) = self.tokens.lock() {
            map.insert(token, proof);
        }
    }

    fn remove(&self, token: &str) {
        if let Ok(mut map) = self.tokens.lock() {
            map.remove(token);
        }
    }

    fn get(&self, token: &str) -> Option<String> {
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
    pub domain_map: HashMap<String, FormDomainCert>
}

impl FormSniResolver {
    //Insert
    //Remove
    //Refresh
    //Get
}

impl ResolvesServerCert for FormSniResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?;
        if let Some(ref domain_cert) = self.domain_map.get(sni) {
            Some(
                Arc::new(
                    CertifiedKey::new(
                        domain_cert.certificates().clone(),
                        any_supported_type(domain_cert.key()).ok()?
                    )
                )
            )
        } else {
            None
        }
    }
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
    std::fs::create_dir_all(&cert_output)?;
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

async fn handle_request(
    req: Request<Body>,
    challenge_map: Arc<ChallengeMap>
) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method();

    // Only handle GET for challenge tokens
    if method == Method::GET && path.starts_with("/.well-known/acme-challenge/") {
        let token = path.trim_start_matches("/.well-known/acme-challenge/");
        // Lookup the token in challenge_map
        if let Some(proof) = challenge_map.get(token) {
            // Return 200 with the challenge proof
            return Ok(Response::new(Body::from(proof)));
        } else {
            // Token not found => 404
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            return Ok(not_found);
        }
    }

    // For everything else, just say "Hello" or 404
    let mut resp = Response::new(Body::from("Not Found"));
    *resp.status_mut() = StatusCode::NOT_FOUND;
    Ok(resp)
}

fn obtain_cert_http_challenge(domain: &str, challenge_map: Arc<ChallengeMap>) -> Result<FormDomainCert, Box<dyn std::error::Error>> {
    println!("Starting ACME for domain: {}", domain);

    // 1) Create or load an existing account with Let’s Encrypt Staging
    //    Switch to DirectoryUrl::LetsEncrypt for production
    let dir_url = DirectoryUrl::LetsEncryptStaging;
    let persist = FilePersist::new("./acme-data");
    let dir = Directory::from_url(persist, dir_url)?;
    // Creates or loads account key from persistence
    let acc = dir.account("admin@example.com")?;

    // 2) Create a new certificate order
    let mut order = acc.new_order(domain, &[])?;

    // Attempt to finalize if domain is already authorized in a prior run
    let ord_csr = loop {
        if let Some(ord_csr) = order.confirm_validations() {
            // Already validated
            break ord_csr;
        }

        // 3) We must validate domain ownership
        let auths = order.authorizations()?;
        if auths.is_empty() {
            return Err(Box::new(Error::Other("No authorizations found".into())));
        }

        let auth = &auths[0]; // single domain => single auth
        let chall = auth.http_challenge();

        // The token is the filename
        let token = chall.http_token();
        // The proof is the content that must be served
        let proof = chall.http_proof();

        println!("Inserting challenge token => /{token}");
        challenge_map.insert(token.to_string(), proof);

        // 4) Now tell ACME that we are ready to validate
        chall.validate(5000)?;  // poll every 5000 ms
        // 5) Refresh the order status
        order.refresh()?;
    };

    // 6) We are authorized, so finalize the order with a new private key
    let pkey_pri = create_p384_key();
    let ord_cert = ord_csr.finalize_pkey(pkey_pri, 5000)?;

    // 7) Download the certificate
    let cert = ord_cert.download_and_save_cert()?;
    let cert_bytes: CertificateDer<'static> = cert.certificate_der().clone().into();
    let private_key_bytes = PrivateKeyDer::from_pem_slice(&cert.private_key_der())?;
    println!("Certificate successfully obtained! Files saved in ./acme-data.\n");

    //TODO: Get token and remove from challenge map in case we want a clean slate
    // challenge_map.remove(token);
    let domain_cert = FormDomainCert::new(vec![cert_bytes], private_key_bytes);

    Ok(domain_cert)
}

/// Spawns an HTTP server on `port` that serves the ACME http-01 challenge tokens
/// from the provided `challenge_map`. Returns immediately (spawning a background task).
pub async fn start_acme_challenge_server(challenge_map: Arc<ChallengeMap>, port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("ACME challenge server listening on http://{}", addr);

    let make_svc = make_service_fn(move |_conn| {
        let cm = challenge_map.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, cm.clone())
            }))
        }
    });

    // spawn the server in a background task
    tokio::spawn(async move {
        let server = Server::bind(&addr).serve(make_svc);
        if let Err(e) = server.await {
            eprintln!("ACME challenge server error: {}", e);
        }
    });
}
