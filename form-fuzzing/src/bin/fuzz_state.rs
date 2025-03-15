//! State Fuzzer for Formation Network

use form_fuzzing::generators::state::{
    InstanceGenerator, NodeGenerator, AccountGenerator,
    PeerGenerator, CidrGenerator, AssociationGenerator,
    DnsRecordGenerator, CRDTDnsRecordGenerator
};
use form_fuzzing::generators::Generator;
use form_fuzzing::harness::state::StateFuzzHarness;
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::instrumentation::sanitizer;
use form_fuzzing::mutators::Mutator;
use form_fuzzing::mutators::state::{
    AccountMutator, InstanceMutator, NodeMutator,
    CrdtPeerMutator, CrdtCidrMutator, CrdtAssociationMutator,
    FormDnsRecordMutator, CrdtDnsRecordMutator
};

use std::time::{Instant, Duration};
use std::thread;
use std::fs;
use std::path::Path;
use rand::Rng;
use rand::seq::SliceRandom;
use uuid::Uuid;

fn main() {
    // Initialize logging
    env_logger::init();
    
    // Create harness
    let harness = StateFuzzHarness::new();
    
    // Set up generators
    let instance_gen = InstanceGenerator::new();
    let node_gen = NodeGenerator::new();
    let account_gen = AccountGenerator::new();
    let crdt_peer_gen = PeerGenerator::new();
    let crdt_cidr_gen = CidrGenerator::new();
    let crdt_association_gen = AssociationGenerator::new();
    let form_dns_record_gen = DnsRecordGenerator::new();
    let crdt_dns_record_gen = CRDTDnsRecordGenerator::new();
    
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