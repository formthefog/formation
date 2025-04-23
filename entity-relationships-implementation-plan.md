# Entity Relationships Implementation Plan

## Overview

This document outlines the implementation plan for establishing proper relationships between accounts, agents, instances, models, and DNS entries in the Formation platform. The goal is to ensure that all entities are properly connected to deliver a high-quality user experience.

## Current State

- Entities (accounts, agents, instances, models, DNS entries) exist but have tenuous connections
- When form-pack builds an image, it creates an instance but doesn't properly link it to agents or accounts
- DNS records aren't automatically created or linked to instances
- Authentication is required for entity creation but lacks trusted node bypass for devnet

## Implementation Goals

1. Ensure account verification for all operations
2. Extend form-pack to extract agent metadata from Formfiles
3. Update agents with build IDs after image creation
4. Update agents and instances after deployment
5. Ensure proper authentication throughout the process
6. Add trusted node auth bypass for devnet environments

## Implementation Plan

### 1. Account Verification and Authentication

#### 1.1 Implement Auth Bypass for Localhost

- **Tasks:**
  - [ ] Add localhost detection in API key auth middleware
  - [ ] Create helper methods for dummy localhost auth objects
  - [ ] Update auth middleware to bypass authentication for localhost requests
  - [ ] Add detailed logging for security auditing

- **Implementation Details:**
  ```rust
  // Add to api_keys/middleware.rs
  
  // Either reuse existing function from api.rs or add this:
  fn is_localhost_request(req: &Request<Body>) -> bool {
      if let Some(addr) = req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>() {
          let ip = addr.ip();
          return ip.is_loopback();
      }
      
      // If we can't determine the address, check headers for proxy info
      if let Some(forwarded) = req.headers().get("x-forwarded-for") {
          if let Ok(addr) = forwarded.to_str() {
              return addr == "127.0.0.1" || addr == "::1";
          }
      }
      
      false
  }
  
  // Modify the API key auth middleware to add localhost bypass
  pub async fn api_key_auth_middleware(
      State(state): State<Arc<Mutex<DataStore>>>,
      mut request: Request<Body>,
      next: Next,
  ) -> Result<Response, StatusCode> {
      // Path and logging info...
      
      // Check if request is from localhost - bypass auth if it is
      if is_localhost_request(&request) {
          log::info!("Localhost detected, bypassing API key authentication for: {}", path);
          
          // Create a placeholder ApiKeyAuth for localhost requests
          let default_account = Account::default_for_localhost();
          let localhost_auth = ApiKeyAuth {
              api_key: ApiKey::localhost_key(),
              account: default_account,
          };
          
          // Set the auth object in request extensions
          request.extensions_mut().insert(localhost_auth);
          
          // Continue with the request
          return Ok(next.run(request).await);
      }
      
      // Rest of original authentication logic...
  }
  ```

#### 1.2 Enhance Account Checking in API Endpoints

- **Tasks:**
  - [ ] Add helper function to check account ownership
  - [ ] Implement function to verify account authorization for entity operations
  - [ ] Add account relationship validation

### 2. Extend form-pack for Agent Metadata

#### 2.1 Enhance Formfile Parser

- **Tasks:**
  - [ ] Add DESCRIPTION directive to Formfile parser
  - [ ] Add MODEL directive to Formfile parser
  - [ ] Add DOMAINS directive to Formfile parser
  - [ ] Update parser to extract existing metadata (NAME, resource requirements, etc.)

- **Implementation Details:**
  ```rust
  // Add to formfile.rs
  fn parse_description(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> {
      self.description = Some(args.trim().to_string());
      Ok(())
  }
  
  fn parse_model(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> {
      // Format: "required:model-id" or "preferred:model-id"
      let parts: Vec<&str> = args.split(':').collect();
      if parts.len() != 2 {
          return Err("Invalid MODEL format. Use 'required:model-id' or 'preferred:model-id'".into());
      }
      
      match parts[0] {
          "required" => {
              self.model_required = true;
              self.model_id = Some(parts[1].trim().to_string());
          },
          "preferred" => {
              self.model_required = false;
              self.model_id = Some(parts[1].trim().to_string());
          },
          _ => return Err("Invalid MODEL type. Use 'required' or 'preferred'".into())
      }
      
      Ok(())
  }
  
  fn parse_domains(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> {
      // Format: "internal:domain external:domain"
      let parts: Vec<&str> = self.split_preserving_quotes(args)?;
      
      for part in parts {
          let domain_parts: Vec<&str> = part.split(':').collect();
          if domain_parts.len() != 2 {
              return Err("Invalid DOMAINS format. Use 'internal:domain external:domain'".into());
          }
          
          match domain_parts[0] {
              "internal" => self.internal_domain = Some(domain_parts[1].trim().to_string()),
              "external" => self.external_domain = Some(domain_parts[1].trim().to_string()),
              _ => return Err("Invalid domain type. Use 'internal' or 'external'".into())
          }
      }
      
      Ok(())
  }
  ```

#### 2.2 Create Agent Metadata Extractor in form-pack

- **Tasks:**
  - [ ] Create function to convert Formfile data to AIAgent struct
  - [ ] Add reasonable defaults for missing fields
  - [ ] Implement validation for required fields

- **Implementation Details:**
  ```rust
  pub fn formfile_to_agent(
      formfile: &Formfile, 
      owner_id: &str
  ) -> Result<AIAgent, Box<dyn std::error::Error>> {
      // Generate a unique ID for the agent
      let agent_id = generate_unique_id();
      
      // Current timestamp
      let now = chrono::Utc::now().timestamp();
      
      // Extract resource requirements
      let resource_requirements = AgentResourceRequirements {
          min_vcpus: formfile.get_vcpus(),
          recommended_vcpus: formfile.get_vcpus(),
          min_memory_mb: (formfile.get_memory() * 1024) as u64,
          recommended_memory_mb: (formfile.get_memory() * 1024) as u64,
          min_disk_gb: formfile.get_storage().unwrap_or(5) as u64,
          recommended_disk_gb: formfile.get_storage().unwrap_or(5) as u64,
          requires_gpu: formfile.get_gpu_devices().is_some(),
      };
      
      // Create AIAgent from Formfile
      let agent = AIAgent {
          agent_id,
          name: formfile.name.clone(),
          owner_id: owner_id.to_string(),
          version: "0.1.0".to_string(),
          description: formfile.description.clone().unwrap_or_else(|| "".to_string()),
          documentation: None,
          license: ModelLicense::MIT,  // Default
          agent_type: AgentType::Assistant,  // Default
          framework: AgentFramework::FormationAgent,
          runtime: extract_runtime_from_formfile(formfile),
          compatible_model_types: vec![ModelType::LLM],  // Default
          preferred_models: Vec::new(),
          requires_specific_model: formfile.model_required,
          required_model_id: formfile.model_id.clone(),
          tags: Vec::new(),  // Default
          created_at: now,
          updated_at: now,
          formfile_template: base64::encode(serde_json::to_string(formfile)?),
          resource_requirements,
          capabilities: Vec::new(),  // Default
          tools: Vec::new(),  // Default
          has_memory: false,  // Default
          has_external_api_access: false,  // Default 
          has_internet_access: false,  // Default
          has_filesystem_access: true,  // Default for VM-based agents
          average_rating: None,
          deployment_count: 0,
          usage_count: 0,
          is_featured: false,
          is_private: false,
          metadata: BTreeMap::new(),
          repository_url: None,
          demo_url: None,
          price_per_request: None,
          usage_tracking: AgentUsageTracking::default(),
          config_schema: None,
      };
      
      Ok(agent)
  }
  ```

### 3. API Client for form-state Integration

#### 3.1 Implement API Client in form-pack

- **Tasks:**
  - [ ] Create HTTP client to communicate with form-state API
  - [ ] Implement agent creation function
  - [ ] Implement agent update function
  - [ ] Handle authentication and error cases

- **Implementation Details:**
  ```rust
  pub struct FormStateClient {
      base_url: String,
      auth_token: Option<String>,
      is_trusted_node: bool,
  }
  
  impl FormStateClient {
      pub fn new(base_url: &str, auth_token: Option<String>, is_trusted_node: bool) -> Self {
          Self {
              base_url: base_url.to_string(),
              auth_token,
              is_trusted_node,
          }
      }
      
      pub async fn create_agent(&self, agent: AIAgent) -> Result<AIAgent, Box<dyn std::error::Error>> {
          let client = reqwest::Client::new();
          let mut req = client.post(&format!("{}/api/v1/agent", self.base_url))
              .json(&AgentRequest::Create(agent));
          
          if let Some(token) = &self.auth_token {
              req = req.header("Authorization", format!("Bearer {}", token));
          }
          
          if self.is_trusted_node {
              req = req.header("X-Trusted-Node", "true");
          }
          
          let response = req.send().await?;
          
          if !response.status().is_success() {
              return Err(format!("Failed to create agent: {} - {}", 
                               response.status(), 
                               response.text().await?).into());
          }
          
          let result: Response<AIAgent> = response.json().await?;
          match result {
              Response::Success(Success::Some(agent)) => Ok(agent),
              Response::Success(Success::None) => Err("No agent returned".into()),
              Response::Failure { reason } => Err(reason.unwrap_or_else(|| "Unknown error".to_string()).into()),
          }
      }
      
      pub async fn update_agent(&self, agent: AIAgent) -> Result<AIAgent, Box<dyn std::error::Error>> {
          // Similar implementation as create_agent
      }
  }
  ```

### 4. Build Process Integration

#### 4.1 Enhance Build Process in form-pack

- **Tasks:**
  - [ ] Update build command to extract agent metadata
  - [ ] Add agent registration step after successful build
  - [ ] Update agent with build ID after image creation

- **Implementation Details:**
  ```rust
  pub async fn build_and_register(
      formfile_path: &Path,
      owner_id: &str,
      state_url: &str,
      auth_token: Option<String>
  ) -> Result<(String, String), Box<dyn std::error::Error>> {
      // 1. Parse Formfile
      let formfile_content = std::fs::read_to_string(formfile_path)?;
      let formfile = parse_formfile(&formfile_content)?;
      
      // 2. Create AIAgent from Formfile
      let agent = formfile_to_agent(&formfile, owner_id)?;
      
      // 3. Register agent with form-state
      let client = FormStateClient::new(state_url, auth_token, true);
      let registered_agent = client.create_agent(agent).await?;
      
      // 4. Build image
      let build_id = build_image(formfile_path)?;
      
      // 5. Update agent with build ID
      let mut updated_agent = registered_agent;
      updated_agent.metadata.insert("build_id".to_string(), build_id.clone());
      updated_agent.updated_at = chrono::Utc::now().timestamp();
      
      let _ = client.update_agent(updated_agent.clone()).await?;
      
      Ok((registered_agent.agent_id, build_id))
  }
  ```

### 5. Deployment Integration

#### 5.1 Update Deployment Process in VMM

- **Tasks:**
  - [ ] Modify deployment to accept agent ID
  - [ ] Update instance with agent ID during deployment
  - [ ] Update agent with instance ID after successful deployment

- **Implementation Details:**
  ```rust
  pub async fn deploy_instance(
      build_id: &str,
      agent_id: &str,
      node_id: &str,
      state_url: &str,
      auth_token: Option<String>
  ) -> Result<String, Box<dyn std::error::Error>> {
      // 1. Deploy instance
      let instance_id = deploy_image_to_vm(build_id)?;
      
      // 2. Update instance with agent ID
      update_instance_metadata(instance_id, "agent_id", agent_id)?;
      
      // 3. Update agent with instance ID
      let client = FormStateClient::new(state_url, auth_token, true);
      let agent = client.get_agent(agent_id).await?;
      
      let mut updated_agent = agent;
      let mut instances = match updated_agent.metadata.get("instances") {
          Some(instances_str) => {
              let mut instances: Vec<String> = serde_json::from_str(instances_str)?;
              instances.push(instance_id.to_string());
              instances
          },
          None => vec![instance_id.to_string()],
      };
      
      updated_agent.metadata.insert("instances".to_string(), serde_json::to_string(&instances)?);
      updated_agent.metadata.insert("node_id".to_string(), node_id.to_string());
      updated_agent.updated_at = chrono::Utc::now().timestamp();
      updated_agent.deployment_count += 1;
      
      let _ = client.update_agent(updated_agent).await?;
      
      Ok(instance_id.to_string())
  }
  ```

### 6. DNS Registration

#### 6.1 Implement DNS Record Creation

- **Tasks:**
  - [ ] Extract domain information during build
  - [ ] Create API client for form-dns
  - [ ] Register domains after successful deployment
  - [ ] Link domains to agent and instance

- **Implementation Details:**
  ```rust
  pub async fn register_domains(
      agent_id: &str,
      instance_id: &str,
      internal_domain: Option<String>,
      external_domain: Option<String>,
      dns_url: &str,
      state_url: &str,
      auth_token: Option<String>
  ) -> Result<(), Box<dyn std::error::Error>> {
      // 1. Get instance IP address
      let instance_ip = get_instance_ip(instance_id)?;
      
      // 2. Register internal domain if provided
      if let Some(domain) = internal_domain {
          register_dns_record(
              dns_url, 
              &domain, 
              &instance_ip, 
              true, 
              auth_token.clone()
          ).await?;
      }
      
      // 3. Register external domain if provided
      if let Some(domain) = external_domain {
          register_dns_record(
              dns_url, 
              &domain, 
              &instance_ip, 
              false, 
              auth_token.clone()
          ).await?;
      }
      
      // 4. Update agent with domain information
      let client = FormStateClient::new(state_url, auth_token, true);
      let agent = client.get_agent(agent_id).await?;
      
      let mut updated_agent = agent;
      if let Some(domain) = internal_domain {
          updated_agent.metadata.insert("internal_domain".to_string(), domain);
      }
      
      if let Some(domain) = external_domain {
          updated_agent.metadata.insert("external_domain".to_string(), domain);
      }
      
      updated_agent.updated_at = chrono::Utc::now().timestamp();
      let _ = client.update_agent(updated_agent).await?;
      
      Ok(())
  }
  
  async fn register_dns_record(
      dns_url: &str,
      domain: &str,
      ip: &str,
      is_internal: bool,
      auth_token: Option<String>
  ) -> Result<(), Box<dyn std::error::Error>> {
      let client = reqwest::Client::new();
      let mut req = client.post(&format!("{}/api/v1/dns/record", dns_url))
          .json(&json!({
              "domain": domain,
              "target": ip,
              "is_internal": is_internal,
              "record_type": "A"
          }));
      
      if let Some(token) = auth_token {
          req = req.header("Authorization", format!("Bearer {}", token));
      }
      
      let response = req.send().await?;
      
      if !response.status().is_success() {
          return Err(format!("Failed to register DNS: {} - {}", 
                           response.status(), 
                           response.text().await?).into());
      }
      
      Ok(())
  }
  ```

## Implementation Timeline

### Phase 1: Authentication and API Client (1 week)
- Implement trusted node auth bypass in form-state
- Create API client in form-pack for form-state communication
- Add helper functions for authentication

### Phase 2: Formfile Extensions (1 week)
- Add new directives to Formfile parser
- Create metadata extractor for agent creation
- Test parser with various Formfile formats

### Phase 3: Build Process Integration (1 week)
- Enhance build command to register agents
- Update build process to link entities
- Test build flow end-to-end

### Phase 4: Deployment and DNS Integration (1 week)
- Update deployment process to update entities
- Implement DNS registration
- Create end-to-end tests

## Success Criteria

1. Agents can be registered from Formfiles with correct metadata
2. Build IDs are properly linked to agents
3. Deployed instances are associated with their agents
4. DNS entries correctly point to agent instances
5. All operations maintain proper authentication
6. Devnet trusted nodes can perform operations without authentication 