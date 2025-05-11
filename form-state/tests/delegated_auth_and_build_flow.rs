#[cfg(test)]
mod delegated_auth_and_build_flow_tests {
    use k256::ecdsa::{SigningKey, signature::Signer, Signature as K256Signature, RecoveryId};
    use sha2::{Sha256, Digest};
    use rand::rngs::OsRng;
    use hex;
    use alloy_primitives::Address;
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    use reqwest::{
        Client,
        header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
        multipart,
    };
    use serde_json::{json, Value};
    use tiny_keccak::{Hasher, Keccak, Sha3};
    use std::fs::File;
    use std::io::Write;
    use tar::Builder as TarBuilder;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tempfile::{tempdir, TempDir};
    use log::{info, error, LevelFilter}; // For logging, added LevelFilter
    use std::sync::Once;
    use std::path::PathBuf;

    // --- Configuration ---
    const FORM_STATE_URL: &str = "http://localhost:3004";
    const FORM_PACK_URL: &str = "http://localhost:3003";
    const TEST_AGENT_NAME_IN_FORMFILE: &str = "test-delegated-agent-rust";

    // --- Helper Functions ---
    static INIT: Once = Once::new();

    fn init_logger() {
        // Initialize logger only once
        INIT.call_once(|| {
            simple_logger::SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
            info!("Logger initialized for delegated_auth_and_build_flow_tests");
        });
    }

    // Generate a keypair and Ethereum address
    fn generate_keypair() -> (SigningKey, Address) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        // Ethereum address derivation:
        // Keccak-256 hash of the uncompressed public key (excluding the 0x04 prefix)
        // Then take the last 20 bytes of this hash.
        let mut hasher = Keccak::v256();
        let mut output = [0u8; 32];
        // Get the uncompressed public key bytes. 
        // .to_encoded_point(false) gives X9.62 uncompressed form (0x04 + X + Y)
        hasher.update(&verifying_key.to_encoded_point(false).as_bytes()[1..]); 
        hasher.finalize(&mut output);
        let address = Address::from_slice(&output[12..]); // Last 20 bytes of the Keccak-256 hash
        
        (signing_key, address)
    }

    // Sign a message and create the Authorization header value
    // Message format for our services: "<HTTP_METHOD>:<REQUEST_PATH>:<TIMESTAMP_SECONDS>"
    // Header format: "Signature <signature_hex>.<recovery_id>.<original_message_hex>"
    fn create_auth_header(signing_key: &SigningKey, message: &str) -> Result<String, String> {
        // 1. Hash the message string with SHA-256 (as per ecdsa.rs auth logic in form-state/form-pack)
        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        let message_hash = hasher.finalize();

        // 2. Sign the hash to get recoverable signature
        let (signature, recovery_id): (K256Signature, RecoveryId) = signing_key
            .sign_recoverable(message_hash.as_slice())
            .map_err(|e| format!("Failed to sign message: {}", e))?;

        // 3. Hex encode the original (unhashed) message string for the header
        let original_message_hex = hex::encode(message.as_bytes());

        // 4. Format the Authorization header value
        let auth_value = format!(
            "Signature {}.{}.{}",
            hex::encode(signature.to_bytes()),
            recovery_id.to_byte(),
            original_message_hex
        );
        Ok(auth_value)
    }

    // Create a dummy tar.gz artifact containing nginx.conf and start.sh
    fn create_dummy_artifacts(dir: &TempDir) -> PathBuf {
        let nginx_conf_path = dir.path().join("nginx.conf");
        let start_sh_path = dir.path().join("start.sh");

        let mut nginx_file = File::create(&nginx_conf_path).expect("Failed to create nginx.conf for artifacts");
        nginx_file.write_all(b"server { listen 80; location / { return 200 'Nginx OK from dummy artifact'; } }").expect("Failed to write nginx.conf");

        let mut start_sh_file = File::create(&start_sh_path).expect("Failed to create start.sh for artifacts");
        start_sh_file.write_all(b"#!/bin/bash\necho 'Dummy start.sh executed'\nexit 0").expect("Failed to write start.sh");
        // In a real scenario, you might need to set execute permissions here if form-pack doesn't handle it.

        let tar_gz_path = dir.path().join("artifacts.tar.gz");
        let tar_gz_file = File::create(&tar_gz_path).expect("Failed to create artifacts.tar.gz");
        let enc = GzEncoder::new(tar_gz_file, Compression::default());
        let mut tar_builder = TarBuilder::new(enc);

        // Add files to the tar archive. The path inside the tar will be just the filename.
        tar_builder.append_path_with_name(&nginx_conf_path, "nginx.conf").expect("Failed to add nginx.conf to tar");
        tar_builder.append_path_with_name(&start_sh_path, "start.sh").expect("Failed to add start.sh to tar");
        
        // Finish writing the tar archive and close the Gzip encoder
        let encoder = tar_builder.into_inner().expect("Failed to get tar encoder from builder");
        encoder.finish().expect("Failed to finish gzip encoding");
        
        info!("Created dummy artifacts at: {}", tar_gz_path.display());
        tar_gz_path
    }

    // --- Test Function ---
    #[tokio::test]
    async fn delegated_auth_and_build_flow_test() {
        init_logger();
        let client = Client::new();
        let (signing_key, wallet_address) = generate_keypair();
        // wallet_address_str is for logging and for parts of messages that might expect "0x"
        let wallet_address_str = format!("0x{}", hex::encode(wallet_address)); 
        info!("Using Wallet Address for logging/display: {}", wallet_address_str);

        // --- Step 1: Create Account ---
        info!("\nSTEP 1: Creating Account in form-state...");
        let timestamp_create_acc = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let msg_create_acc = format!("POST:/account/create:{}", timestamp_create_acc);
        let auth_header_create_acc = create_auth_header(&signing_key, &msg_create_acc)
            .expect("Failed to create auth header for account creation");

        // For the payload, the server-side `create_account` handler now normalizes the address from the payload.
        // So, we can send it with or without "0x", but using the `wallet_address_str` (with "0x") is fine.
        let account_data_for_payload = json!({
            "address": wallet_address_str, 
            "name": "Test Delegated Acc",   
            "owned_instances": [],
            "owned_agents": [],
            "owned_models": [],
            "authorized_instances": {},
            "subscription": null,
            "hired_agents": []
        });

        let account_payload = json!({
            "Create": account_data_for_payload
        });

        let res_create_acc = client.post(format!("{}/account/create", FORM_STATE_URL))
            .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_create_acc).unwrap())
            .header(CONTENT_TYPE, "application/json")
            .json(&account_payload)
            .send()
            .await
            .expect("Failed to send account create request");

        let status_create_acc = res_create_acc.status();
        let response_text_create_acc = res_create_acc.text().await.expect("Failed to get text from account create response");
        info!("Create Account Response Status: {}", status_create_acc);
        info!("Create Account Response Body: {}", response_text_create_acc);

        let body_create_acc: Value = serde_json::from_str(&response_text_create_acc)
            .expect(&format!("Failed to parse account create response as JSON. Raw text: {}", response_text_create_acc));
        
        assert!(status_create_acc.is_success(), "Account creation HTTP request failed. Status: {}. Body: {}", status_create_acc, serde_json::to_string_pretty(&body_create_acc).unwrap_or_default());
        assert_eq!(body_create_acc["success"].as_bool().unwrap_or(false), true, "Account creation was not successful in API response. Body: {}", serde_json::to_string_pretty(&body_create_acc).unwrap_or_default());
        info!("Account created successfully.");

        // --- Step 2: Prepare for Image Build ---
        info!("\nSTEP 2: Preparing for Image Build...");
        let temp_artifacts_dir = tempdir().expect("Failed to create temp dir for artifacts");
        let artifacts_tar_gz_path = create_dummy_artifacts(&temp_artifacts_dir);

        let formfile_json_content = json!({
            "name": TEST_AGENT_NAME_IN_FORMFILE,
            "description": "A test agent built via Rust integration test for delegated auth.",
            "model_id": null, // Using Option<String> for model_id, so null is appropriate if not set
            "model_required": false,
            "build_instructions": [
                { "Copy": ["nginx.conf", "/app/nginx.conf"] },
                { "Copy": ["start.sh", "/app/start.sh"] },
                { "Run": "chmod +x /app/start.sh" },
                { "Entrypoint": { "command": "/app/start.sh", "args": [] } }
            ],
            "system_config": [
                { "Cpu": 1 }, { "Memory": 256 }, { "Disk": 5 }
            ],
            "users": [],
            "workdir": "/app"
        }).to_string();

        // --- Step 3: Build Image (form-pack-manager) ---
        info!("\nSTEP 3: Building Image via form-pack-manager...");
        let timestamp_build = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let msg_build = format!("POST:/build:{}", timestamp_build);
        let auth_header_build = create_auth_header(&signing_key, &msg_build)
            .expect("Failed to create auth header for build request");

        // Explicitly read file content for the part
        let artifact_bytes = std::fs::read(&artifacts_tar_gz_path)
            .expect(&format!("Failed to read artifact file: {:?}", artifacts_tar_gz_path));
        
        let artifacts_part = multipart::Part::bytes(artifact_bytes)
            .file_name("artifacts.tar.gz") // It's good practice to set a filename for the part
            .mime_str("application/gzip") 
            .expect("Failed to create multipart Part for artifacts");

        let form = multipart::Form::new()
            .text("metadata", formfile_json_content.clone()) 
            .part("artifacts", artifacts_part); // Use the created Part

        let build_res = client.post(format!("{}/build", FORM_PACK_URL))
            .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_build).unwrap())
            .multipart(form)
            .send()
            .await
            .expect("Failed to send build request");

        let build_status = build_res.status();
        let response_text_build = build_res.text().await.expect("Failed to get text from build response");
        info!("Build Response Status: {}", build_status);
        info!("Build Response Body: {}", response_text_build);
        let build_body: Value = serde_json::from_str(&response_text_build)
            .expect(&format!("Failed to parse build response as JSON. Raw text: {}", response_text_build));

        assert!(build_status.is_success(), "Build request HTTP call failed. Status: {}. Body: {}", build_status, serde_json::to_string_pretty(&build_body).unwrap_or_default());

        let mut hasher = Sha3::v256();
        let mut hash_output = [0u8; 32];
        hasher.update(hex::decode(wallet_address_str.strip_prefix("0x").unwrap_or(&wallet_address_str)).unwrap().as_slice());
        hasher.update(TEST_AGENT_NAME_IN_FORMFILE.as_bytes());
        hasher.finalize(&mut hash_output);
        let actual_build_id = hex::encode(hash_output);
        info!("Derived actual_build_id for verification: {}", actual_build_id);

        // Allow time for processing
        info!("Waiting for build/state updates (15 seconds)...");
        tokio::time::sleep(Duration::from_secs(15)).await;

        // --- Step 4: Get Account from form-state (Verify Linkage) ---
        info!("\nSTEP 4: Verifying Account Update (Linkage)...");
        let timestamp_get_acc = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let msg_get_acc = format!("GET:/account/{}/get:{}", wallet_address_str, timestamp_get_acc); 
        let auth_header_get_acc = create_auth_header(&signing_key, &msg_get_acc)
            .expect("Failed to create auth header for get account request");

        let get_acc_res = client.get(format!("{}/account/{}/get", FORM_STATE_URL, wallet_address_str)) 
            .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_get_acc).unwrap())
            .send()
            .await
            .expect("Failed to send get account request");

        let status_get_acc = get_acc_res.status();
        let response_text_get_acc = get_acc_res.text().await.expect("Failed to get text from get account response");
        info!("Get Account (after build) Response Status: {}", status_get_acc);
        info!("Get Account (after build) Response Body: {}", response_text_get_acc);
        let body_get_acc: Value = serde_json::from_str(&response_text_get_acc)
            .expect(&format!("Failed to parse get account response as JSON. Raw text: {}", response_text_get_acc));

        assert!(status_get_acc.is_success(), "Failed to get account after build. Status: {}. Body: {}", status_get_acc, serde_json::to_string_pretty(&body_get_acc).unwrap_or_default());

        let account_data_after_build = &body_get_acc["account"];
        let owned_agents = account_data_after_build["owned_agents"].as_array().expect("owned_agents is not an array or missing");
        assert!(owned_agents.iter().any(|id| id.as_str().unwrap_or("") == actual_build_id),
                "Agent ID {} not found in account's owned_agents. Owned agents: {:?}", actual_build_id, owned_agents);
        info!("Verified agent ID {} is linked to account {}.", actual_build_id, wallet_address_str);
        
        // --- Step 5: Get Agent from form-state ---
        info!("\nSTEP 5: Verifying Agent Creation...");
        let get_agent_res = client.get(format!("{}/agents/{}", FORM_STATE_URL, actual_build_id))
            .send()
            .await
            .expect("Failed to send get agent request");

        let status_get_agent = get_agent_res.status();
        let response_text_get_agent = get_agent_res.text().await.expect("Failed to get text from get agent response");
        info!("Get Agent Response Status: {}", status_get_agent);
        info!("Get Agent Response Body: {}", response_text_get_agent);
        let body_get_agent: Value = serde_json::from_str(&response_text_get_agent)
            .expect(&format!("Failed to parse get agent response as JSON. Raw text: {}", response_text_get_agent));

        assert!(status_get_agent.is_success(), "Get Agent HTTP request failed. Status: {}. Body: {}", status_get_agent, serde_json::to_string_pretty(&body_get_agent).unwrap_or_default());
        assert_eq!(body_get_agent["success"].as_bool().unwrap_or(false), true, "Get Agent API call was not successful. Body: {}", serde_json::to_string_pretty(&body_get_agent).unwrap_or_default());
        assert_eq!(
            body_get_agent["agent"]["agent_id"].as_str().unwrap_or("").strip_prefix("0x").unwrap_or_else(|| body_get_agent["agent"]["agent_id"].as_str().unwrap_or("")).to_lowercase(),
            actual_build_id.strip_prefix("0x").unwrap_or(&actual_build_id).to_lowercase(), 
            "Agent ID mismatch in Get Agent response."
        );
        assert_eq!(
            body_get_agent["agent"]["owner_id"].as_str().unwrap_or("").strip_prefix("0x").unwrap_or_else(|| body_get_agent["agent"]["owner_id"].as_str().unwrap_or("")).to_lowercase(),
            wallet_address_str.strip_prefix("0x").unwrap_or(&wallet_address_str).to_lowercase(),
            "Agent owner_id mismatch in Get Agent response."
        );

        // --- Step 6: Get Instance from form-state ---
        info!("\nSTEP 6: Verifying Instance Creation...");
        let timestamp_get_instance = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let msg_get_instance = format!("GET:/instance/{}/get_by_build_id:{}", actual_build_id, timestamp_get_instance);
        let auth_header_get_instance = create_auth_header(&signing_key, &msg_get_instance)
            .expect("Failed to create auth header for get instance request");

        let get_instance_res = client.get(format!("{}/instance/{}/get_by_build_id", FORM_STATE_URL, actual_build_id))
            .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_get_instance).unwrap())
            .send()
            .await
            .expect("Failed to send get instance request");

        let status_get_instance = get_instance_res.status();
        let response_text_get_instance = get_instance_res.text().await.expect("Failed to get text from get instance response");
        info!("Get Instance Response Status: {}", status_get_instance);
        info!("Get Instance Response Body: {}", response_text_get_instance);
        let body_get_instance: Value = serde_json::from_str(&response_text_get_instance)
            .expect(&format!("Failed to parse get instance response as JSON. Raw text: {}", response_text_get_instance));

        assert!(status_get_instance.is_success(), "Get Instance HTTP request failed. Status: {}. Body: {}", status_get_instance, serde_json::to_string_pretty(&body_get_instance).unwrap_or_default());
        
        // Expecting {"Success":{"List":[...]}}
        let instance_list = body_get_instance.get("Success")
            .and_then(|s| s.get("List"))
            .and_then(|l| l.as_array())
            .expect("Response format from /get_by_build_id is not {'Success':{'List':[...]}}. Body: {}");
        
        assert_eq!(instance_list.len(), 1, "Expected 1 instance for build_id {}, but found {}. Body: {}", actual_build_id, instance_list.len(), response_text_get_instance);
        
        let instance_data = &instance_list[0];

        // No longer assert body_get_instance["success"], instead we got the instance_data from the list
        // assert_eq!(body_get_instance["success"].as_bool().unwrap_or(false), true, "Get Instance API call was not successful. Body: {}", serde_json::to_string_pretty(&body_get_instance).unwrap_or_default());
        assert_eq!(
            instance_data["build_id"].as_str().unwrap_or("").strip_prefix("0x").unwrap_or_else(|| instance_data["build_id"].as_str().unwrap_or("")).to_lowercase(),
            actual_build_id.strip_prefix("0x").unwrap_or(&actual_build_id).to_lowercase(), 
            "Instance build_id mismatch in Get Instance response."
        );
        assert_eq!(
            instance_data["instance_owner"].as_str().unwrap_or("").strip_prefix("0x").unwrap_or_else(|| instance_data["instance_owner"].as_str().unwrap_or("")).to_lowercase(),
            wallet_address_str.strip_prefix("0x").unwrap_or(&wallet_address_str).to_lowercase(), 
            "Instance owner_id mismatch in Get Instance response."
        );

        let instance_id_from_get = instance_data["instance_id"].as_str().expect("Instance ID not found in Get Instance response").to_string();
        info!("Instance details retrieved successfully. Instance ID: {}", instance_id_from_get);

        let owned_instances = account_data_after_build["owned_instances"].as_array().expect("owned_instances is not an array or missing in account data");
        assert!(owned_instances.iter().any(|id| id.as_str().unwrap_or("") == instance_id_from_get),
                "Instance ID {} not found in account's owned_instances. Owned instances: {:?}", instance_id_from_get, owned_instances);
        info!("Verified instance ID {} is linked to account {}.", instance_id_from_get, wallet_address_str);

        // --- Phase 4: Finalization ---
        info!("\n✅✅✅ Delegated Authentication and Build Flow Test Passed! ✅✅✅");
    }
} 