use std::{collections::BTreeMap, fmt::Debug, sync::{Arc, Mutex}};
use tokio::task::JoinHandle;
use tokio_rustls_acme::{caches::DirCache, tokio_rustls::rustls::ServerConfig, AcmeAcceptor, AcmeConfig, AcmeState, EventOk, ResolvesServerCertAcme};
use tokio_stream::StreamExt;

pub type InnerResolver = Mutex<BTreeMap<String, Arc<ResolvesServerCertAcme>>>;

pub struct TlsManager {
    pub state_handle: JoinHandle<()>, 
    pub resolver: Arc<ResolvesServerCertAcme>,
    pub acceptor: Arc<AcmeAcceptor>,
    pub domains: BTreeMap<String, String>,
    pub config: Arc<ServerConfig>,
}

impl Debug for TlsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "TlsManager") 
    }
}

impl TlsManager {
    pub fn new(domains: Vec<String>) -> Self {
        println!("Splitting domain");
        let split_domains: Vec<Vec<String>> = domains.iter().cloned().map(|v| {
            let split = v.clone().split(".").into_iter().map(|s| s.to_string()).collect();
            split
        }).collect();

        println!("Building emails");
        let emails: Vec<String> = split_domains.iter().map(|split| {
            format!("mailto:admin@{}", split[(split.len() - 2)..].join(".")) 
        }).collect();

        let domains_emails: BTreeMap<String, String> = domains.iter().clone()
            .zip(emails.clone())
            .map(|(d, e)| (d.clone(), e.clone())).collect();

        println!("Building config");
        let config = AcmeConfig::new(domains.clone())
            .contact(emails)
            .cache(DirCache::new("./rustls_acme_cache"))
            .directory_lets_encrypt(true);
        
        let state = config.state();
        let resolver = state.resolver();
        println!("Created resolver for domains: {:?}", domains);
        let acceptor = Arc::new(state.acceptor());
        
        let config = Arc::new(ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver.clone())
        );

        let state_handle = tokio::spawn(async move {
            let _ = Self::start_state_event_handler(state).await;
        });

        Self {
            state_handle,
            acceptor,
            domains: domains_emails,
            resolver,
            config
        }
    }

    pub async fn start_state_event_handler(mut state: AcmeState<std::io::Error>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            match state.next().await {
                Some(Ok(event)) => {
                    match event {
                        EventOk::CertCacheStore => {
                            println!("Certificate stored in cache");
                        }
                        EventOk::DeployedNewCert => {
                            println!("New certificate deployed successfully");
                        }
                        EventOk::DeployedCachedCert => {
                            println!("Cached certificate deployed successfully");
                        }
                        EventOk::AccountCacheStore => {
                            println!("ACME account stored in cache");
                        }
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Certificate error occurred: {:?}", e);
                }
                None => {
                    println!("Certificate state stream ended");
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn add_domain(&mut self, domain: String, prod: bool) -> Result<(), Box<dyn std::error::Error>> {
        println!("Attempting to add domain");
        println!("Gathering existing domains");
        let mut domains = self.domains.keys().cloned().collect::<Vec<_>>().clone();
        println!("Building emails");
        let mut emails = self.domains.values().cloned().collect::<Vec<_>>().clone();
        println!("splitting domain to build new email");
        let split_domain: Vec<&str> = domain.split(".").collect();
        let email = format!("mailto:admin@{}", split_domain[(split_domain.len() - 2)..].join(".")); 
        println!("built new email");
        domains.push(domain.clone());
        emails.push(email);
        println!("building new state");
        let state = AcmeConfig::new(domains)
            .contact(emails)
            .cache(DirCache::new("./rustls_acme_cache"))
            .directory_lets_encrypt(prod)
            .state();

        println!("built new state, updating resolver");
        self.resolver = state.resolver();
        println!("updating acceptor");
        self.acceptor = Arc::new(state.acceptor());
        println!("updating config");
        let config = Arc::new(ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(self.resolver.clone())
        );
        self.config = config;

        let handle = self.state_handle.abort_handle();
        self.state_handle = tokio::spawn(async move {
            let _ = Self::start_state_event_handler(state).await;
        });

        handle.abort();

        println!("Added new domain {domain}");
        Ok(())
    }

    pub fn remove_domain(&mut self, domain: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.domains.remove(domain);
        let domains = self.domains.keys().cloned().collect::<Vec<_>>().clone();
        let emails = self.domains.values().cloned().collect::<Vec<_>>().clone();
        let state = AcmeConfig::new(domains)
            .contact(emails)
            .cache(DirCache::new("./rustls_acme_cache"))
            .directory_lets_encrypt(true)
            .state();

        self.resolver = state.resolver();
        self.acceptor = Arc::new(state.acceptor());
        let handle = self.state_handle.abort_handle();
        self.state_handle = tokio::spawn(async move {
            let _ = Self::start_state_event_handler(state).await;
        });

        handle.abort();

        Ok(())
    }
}
