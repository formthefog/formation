use form_fuzzing::{
    generators::state::{
        AccountGenerator, InstanceGenerator, NodeGenerator, 
        CrdtPeerGenerator, CrdtCidrGenerator, CrdtAssociationGenerator,
        FormDnsRecordGenerator, CrdtDnsRecordGenerator
    },
    mutators::state::{
        AccountMutator, InstanceMutator, NodeMutator,
        CrdtPeerMutator, CrdtCidrMutator, CrdtAssociationMutator,
        FormDnsRecordMutator, CrdtDnsRecordMutator
    },
    harness::state::StateFuzzHarness,
};
use form_fuzzing::{generators::Generator, mutators::Mutator};
use std::time::Duration;

fn main() {
    // Initialize logging
    env_logger::init();
    
    // Create harness
    let harness = StateFuzzHarness::new();
    
    // Set up generators
    let instance_gen = InstanceGenerator::new();
    let node_gen = NodeGenerator::new();
    let account_gen = AccountGenerator::new();
    let crdt_peer_gen = CrdtPeerGenerator::new();
    let crdt_cidr_gen = CrdtCidrGenerator::new();
    let crdt_association_gen = CrdtAssociationGenerator::new();
    let form_dns_record_gen = FormDnsRecordGenerator::new();
    let crdt_dns_record_gen = CrdtDnsRecordGenerator::new();
    
    // Set up mutators
    let instance_mut = InstanceMutator::new();
    let node_mut = NodeMutator::new();
    let account_mut = AccountMutator::new();
    let crdt_peer_mut = CrdtPeerMutator::new();
    let crdt_cidr_mut = CrdtCidrMutator::new();
    let crdt_association_mut = CrdtAssociationMutator::new();
    let form_dns_record_mut = FormDnsRecordMutator::new();
    let crdt_dns_record_mut = CrdtDnsRecordMutator::new();
    
    // Fuzz with instances
    for _ in 0..100 {
        let mut instance = instance_gen.generate();
        harness.fuzz_instance(&instance);
        
        // Mutate and fuzz again
        instance_mut.mutate(&mut instance);
        harness.fuzz_instance(&instance);
    }
    
    // Fuzz with nodes
    for _ in 0..100 {
        let mut node = node_gen.generate();
        harness.fuzz_node(&node);
        
        // Mutate and fuzz again
        node_mut.mutate(&mut node);
        harness.fuzz_node(&node);
    }
    
    // Fuzz with accounts
    for _ in 0..100 {
        let mut account = account_gen.generate();
        harness.fuzz_account(&account);
        
        // Mutate and fuzz again
        account_mut.mutate(&mut account);
        harness.fuzz_account(&account);
    }
    
    // Fuzz with CRDT peers
    for _ in 0..100 {
        let mut peer = crdt_peer_gen.generate();
        harness.fuzz_crdt_peer(&peer);
        
        // Mutate and fuzz again
        crdt_peer_mut.mutate(&mut peer);
        harness.fuzz_crdt_peer(&peer);
    }
    
    // Fuzz with CRDT CIDRs
    for _ in 0..100 {
        let mut cidr = crdt_cidr_gen.generate();
        harness.fuzz_crdt_cidr(&cidr);
        
        // Mutate and fuzz again
        crdt_cidr_mut.mutate(&mut cidr);
        harness.fuzz_crdt_cidr(&cidr);
    }
    
    // Fuzz with CRDT associations
    for _ in 0..100 {
        let mut association = crdt_association_gen.generate();
        harness.fuzz_crdt_association(&association);
        
        // Mutate and fuzz again
        crdt_association_mut.mutate(&mut association);
        harness.fuzz_crdt_association(&association);
    }
    
    // Fuzz with Form DNS records
    for _ in 0..100 {
        let mut record = form_dns_record_gen.generate();
        harness.fuzz_form_dns_record(&record);
        
        // Mutate and fuzz again
        form_dns_record_mut.mutate(&mut record);
        harness.fuzz_form_dns_record(&record);
    }
    
    // Fuzz with CRDT DNS records
    for _ in 0..100 {
        let mut record = crdt_dns_record_gen.generate();
        harness.fuzz_crdt_dns_record(&record);
        
        // Mutate and fuzz again
        crdt_dns_record_mut.mutate(&mut record);
        harness.fuzz_crdt_dns_record(&record);
    }
    
    println!("State fuzzing completed successfully");
} 