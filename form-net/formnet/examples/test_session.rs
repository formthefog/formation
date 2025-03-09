use formnet::relay::{RelaySession, RelayPacket, RelayHeader};
use std::time::{Duration, Instant, SystemTime};
use std::thread::sleep;

fn main() {
    println!("Testing relay session management");
    
    // Create a new session
    let session_id = 12345;
    let initiator_pubkey = [1u8; 32];
    let target_pubkey = [2u8; 32];
    
    let session = RelaySession::new(session_id, initiator_pubkey, target_pubkey);
    
    // Print session details
    println!("Created session with ID: {}", session.id);
    println!("Session will expire at: {:?}", session.expires_at);
    
    // Test authentication
    let header = RelayHeader::new(target_pubkey, session_id);
    let payload = vec![1, 2, 3, 4];
    let packet = RelayPacket {
        header,
        payload: payload.clone(),
    };
    
    let auth_result = session.authenticate_packet(&packet);
    println!("Packet authentication result: {}", auth_result);
    
    // Test token generation
    let token = session.generate_auth_token();
    println!("Generated auth token with length: {}", token.len());
    
    let token_valid = session.verify_auth_token(&token);
    println!("Auth token verification result: {}", token_valid);
    
    // Test expiration
    let is_expired = session.is_expired();
    println!("Session is expired: {}", is_expired);
    
    // Test inactivity
    let is_inactive = session.is_inactive(Duration::from_secs(60));
    println!("Session is inactive (60s threshold): {}", is_inactive);
    
    // Update activity
    let mut session = session;
    println!("Last activity: {:?}", session.last_activity);
    
    // Sleep a bit
    sleep(Duration::from_millis(100));
    
    // Update activity
    session.update_activity();
    println!("Updated activity: {:?}", session.last_activity);
    
    // Record some packets
    session.record_initiator_to_target(1024);
    session.record_target_to_initiator(2048);
    
    // Print stats
    println!("Packets initiator->target: {}", session.packets_forwarded_initiator_to_target);
    println!("Bytes initiator->target: {}", session.bytes_forwarded_initiator_to_target);
    println!("Packets target->initiator: {}", session.packets_forwarded_target_to_initiator);
    println!("Bytes target->initiator: {}", session.bytes_forwarded_target_to_initiator);
    
    println!("Session test completed successfully!");
} 