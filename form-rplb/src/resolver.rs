use std::{collections::BTreeMap, fmt::Debug, sync::{Arc, Mutex}};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;
use tokio_rustls_acme::{
    caches::DirCache,
    tokio_rustls::rustls::{
        server::{ClientHello, ResolvesServerCert}, 
        sign::CertifiedKey, ServerConfig}, AcmeAcceptor, AcmeConfig, AcmeState, EventOk, ResolvesServerCertAcme};
use tokio_stream::StreamExt;

use crate::certs::{mkcert, FormSniResolver};

pub struct AcceptorManager {
    pub acceptors: Mutex<BTreeMap<String, AcmeAcceptor>>,
}

#[derive(Debug)]
pub struct ResolverManager {
    pub acme_resolvers: Mutex<BTreeMap<String, Arc<ResolvesServerCertAcme>>>,
    pub vanity_resolvers: Arc<FormSniResolver>
}

impl ResolvesServerCert for ResolverManager {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if let Some(domain) = client_hello.server_name() {
            if domain.starts_with(".fog") {
                return self.vanity_resolvers.resolve(client_hello);
            } else if let Ok(guard) = self.acme_resolvers.lock() {
                if let Some(resolver) = guard.get(domain) {
                    return resolver.resolve(client_hello)
                }
            }
        }

        None
    }
}

pub struct ConfigManager {
    pub configs: Mutex<BTreeMap<String, Arc<ServerConfig>>>
}

pub struct TlsManager {
    pub state_handles: Vec<(String, JoinHandle<()>)>, 
    pub resolvers: Arc<ResolverManager>,
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
    pub async fn new(domains: Vec<(String, bool)>) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Building TLS Manager");
        let mut states: BTreeMap<String, Arc<TokioMutex<AcmeState<std::io::Error>>>> = BTreeMap::new(); 
        let mut acme_resolvers: BTreeMap<String, Arc<ResolvesServerCertAcme>> = BTreeMap::new();
        let vanity_resolvers: Arc<FormSniResolver> = Arc::new(FormSniResolver::new());
        let mut domains_emails: BTreeMap<String, String> = BTreeMap::new();
        log::info!("Built states, acme resolvers and vanity resolvers...");
        for (domain, prod) in domains {
            log::info!("Adding provided domain {domain}...");
            if !domain.ends_with(".fog") {
                log::info!("Domain is not '.fog'");
                let split: Vec<String> = domain.clone().split(".").into_iter().map(|s| s.to_string()).collect();
                log::info!("Splitting domain...");
                let email: String = format!("mailto:admin@{}", split[(split.len() - 2)..].join(".")); 
                log::info!("Constructing email for acme server...");

                domains_emails.insert(domain.clone(), email.clone());
                log::info!("Inserting emails into domains_emails...");

                log::info!("Building ACME config");
                let config = AcmeConfig::new(vec![domain.clone()])
                    .contact_push(email)
                    .cache(DirCache::new("./rustls_acme_cache"))
                    .directory_lets_encrypt(prod);

                log::info!("Built acme config");
                let state = config.state();
                let resolver = state.resolver();
                states.insert(domain.clone(), Arc::new(TokioMutex::new(state)));
                acme_resolvers.insert(domain.clone(), resolver.clone());
                log::info!("Added state and resolver to TLS manager");
            } else {
                log::info!("Domain is '.fog', adding to vanity resolver domain map");
                vanity_resolvers.domain_map.lock().unwrap().insert(domain.clone(), mkcert(&domain.clone())?);
            }
        }

        log::info!("Building Resolver Manager for both acme and vanity resolvers...");
        let resolver_manager = Arc::new(ResolverManager {
            acme_resolvers: Mutex::new(acme_resolvers),
            vanity_resolvers,
        });

        log::info!("Building ServerConfig...");
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver_manager.clone());

        log::info!("Building AcmeAcceptor from server config...");
        let acceptor = Arc::new(AcmeAcceptor::from_config(config.clone()));

        log::info!("Starting state event handler...");
        let state_handles = if let Ok(handles) = Self::start_state_event_handler(states).await {
            handles
        } else {
            vec![]
        };

        log::info!("Returning TlsManager...");
        Ok(Self {
            state_handles,
            resolvers: resolver_manager,
            acceptor,
            domains: domains_emails,
            config: Arc::new(config),
        })
    }

    pub async fn new_state_handler(domain: String, state: Arc<TokioMutex<AcmeState<std::io::Error>>>) -> (String, JoinHandle<()>) {
        (domain.clone(), tokio::spawn(async move {
            loop {
                match state.lock().await.next().await {
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
        }))
    }

    pub async fn start_state_event_handler(
        states: BTreeMap<String, Arc<TokioMutex<AcmeState<std::io::Error>>>>
    ) -> Result<Vec<(String, JoinHandle<()>)>, Box<dyn std::error::Error>> {
        let mut handles = vec![];
        for (domain, state) in states { 
            handles.push((domain.clone(), tokio::spawn(async move {
                loop {
                    match state.lock().await.next().await {
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
            })));
        }

        Ok(handles)
    }

    pub async fn add_domain(&mut self, domain: String, prod: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !domain.ends_with(".fog") {
            let split_domain: Vec<&str> = domain.split(".").collect();
            let email = format!("mailto:admin@{}", split_domain[(split_domain.len() - 2)..].join(".")); 
            let state = AcmeConfig::new(vec![domain.clone()])
                .contact_push(email.clone())
                .cache(DirCache::new("./rustls_acme_cache"))
                .directory_lets_encrypt(prod)
                .state();

            let resolver = state.resolver();
            let handle = Self::new_state_handler(domain.clone(), Arc::new(TokioMutex::new(state))).await;
            if let Ok(mut resolvers_guard) = self.resolvers.acme_resolvers.lock() {
                resolvers_guard.insert(domain.clone(), resolver.clone());
            } else {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unable to acquire mutex guard")));
            }
            self.domains.insert(domain.clone(), email.clone());
            self.state_handles.push(handle);
        } else {
            if let Ok(mut resolvers_guard) = self.resolvers.vanity_resolvers.domain_map.lock() {
                resolvers_guard.insert(domain.clone(), mkcert(&domain.clone())?);
            }
        }

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(self.resolvers.clone());

        self.acceptor = Arc::new(AcmeAcceptor::from_config(config.clone()));
        self.config = Arc::new(config);

        println!("Added new domain {domain}");
        Ok(())
    }

    pub fn remove_domain(&mut self, domain: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.domains.remove(domain);
        if let Some(state_handle) = self.state_handles.iter().find_map(|(d, handle)| {
            if d == domain {
                Some(handle)
            } else {
                None
            }
        }) {
            state_handle.abort();
        }

        if !domain.ends_with(".fog") {
            if let Ok(mut resolvers_guard) = self.resolvers.acme_resolvers.lock() {
                resolvers_guard.remove(domain);
            }
        } else {
            if let Ok(mut resolvers_guard) = self.resolvers.vanity_resolvers.domain_map.lock() {
                resolvers_guard.remove(domain);
            }
        }

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(self.resolvers.clone());
        self.acceptor = Arc::new(AcmeAcceptor::from_config(config.clone()));
        self.config = Arc::new(config);
        Ok(())
    }
}
