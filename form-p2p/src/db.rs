use std::fmt::Debug;
use std::sync::Arc;
use std::path::PathBuf;
use serde::{Serialize, de::DeserializeOwned};
use crdts::bft_topic_queue::TopicQueue; // Adjust to your actual crate path
use crdts::{merkle_reg::Sha3Hash, CmRDT, ResetRemove, vclock::VClock}; // Assuming this trait for T
use redb::{Database, TableDefinition, ReadableTable};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use std::hash::Hash;
use std::str::FromStr;

// Placeholder imports (adjust to your actual crate paths)
use crdts::map::{Map, Entry};

// Define our table for storing entries
const ENTRIES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("entries");

/// Opens a redb database at the specified path.
/// Creates the database if it doesn't exist.
pub fn open_db(path: PathBuf) -> Arc<Database> {
    let db = Database::create(path).expect("Failed to open redb database");
    
    // Create the tables if they don't exist
    let write_txn = db.begin_write().expect("Failed to begin write transaction");
    {
        let _ = write_txn.open_table(ENTRIES_TABLE).expect("Failed to open entries table");
    }
    write_txn.commit().expect("Failed to commit transaction");
    
    Arc::new(db)
}

/// Stores a Map<K, V, A> to redb.
pub fn store_map<K, V, A>(db: &Database, map_name: &str, map: &Map<K, V, A>)
where
    K: Serialize + Ord + ToString,
    V: Serialize + Clone + Default + CmRDT + ResetRemove<A>,
    A: Serialize + Ord + Hash + Clone,
{
    let write_txn = db.begin_write().expect("Failed to begin write transaction");
    {
        let mut table = write_txn.open_table(ENTRIES_TABLE).expect("Failed to open table");

        // Store the clock
        let clock_key = format!("{}/clock", map_name).into_bytes();
        let clock_bytes = bincode::serialize(&map.clock).expect("Failed to serialize clock");
        table.insert(&clock_key[..], &clock_bytes[..]).expect("Failed to insert clock");

        // Store the entries
        for (k, entry) in map.entries.iter() {
            let entry_key = format!("{}/entries/{}", map_name, k.to_string()).into_bytes();
            let entry_bytes = bincode::serialize(entry).expect("Failed to serialize entry");
            table.insert(&entry_key[..], &entry_bytes[..]).expect("Failed to insert entry");
        }

        // Store deferred operations (using a simple index for simplicity)
        for (idx, (vclock, keys)) in map.deferred.iter().enumerate() {
            let deferred_key = format!("{}/deferred/{}", map_name, idx).into_bytes();
            let cloned_clock = (*vclock).clone();
            let deferred_bytes = bincode::serialize(&(cloned_clock, keys)).expect("Failed to serialize deferred");
            table.insert(&deferred_key[..], &deferred_bytes[..]).expect("Failed to insert deferred");
        }
    }
    // Write all changes atomically
    write_txn.commit().expect("Failed to commit transaction");
}

/// Loads a Map<K, V, A> from redb.
pub fn load_map<K, V, A>(db: &Database, map_name: &str) -> Map<K, V, A>
where
    K: DeserializeOwned + Ord + FromStr, // FromStr needed to parse keys
    <K as FromStr>::Err: std::fmt::Debug, // Required for unwrap
    V: DeserializeOwned + Clone + Default + CmRDT + ResetRemove<A>,
    A: DeserializeOwned + Ord + Hash + Clone,
{
    let read_txn = db.begin_read().expect("Failed to begin read transaction");
    let table = read_txn.open_table(ENTRIES_TABLE).expect("Failed to open table");

    // Load the clock
    let clock_key = format!("{}/clock", map_name).into_bytes();
    let clock = match table.get(&clock_key[..]).expect("Failed to read clock") {
        Some(bytes) => bincode::deserialize(bytes.value()).expect("Failed to deserialize clock"),
        None => VClock::new(), // Default if not found (e.g., first run)
    };

    // Load the entries
    let mut entries = BTreeMap::new();
    let entry_prefix = format!("{}/entries/", map_name).into_bytes();
    
    // Scan all keys and filter those that match the prefix
    for entry in table.iter().expect("Failed to iterate table") {
        let (key, value) = entry.expect("Failed to get entry");
        let key_bytes = key.value();
        
        if !starts_with(key_bytes, &entry_prefix) {
            continue; // Not part of our prefix
        }
        
        let key_str = String::from_utf8(key_bytes.to_vec()).expect("Invalid UTF-8 key");
        let entry_prefix_str = String::from_utf8(entry_prefix.clone()).expect("Invalid UTF-8 prefix");
        
        if !key_str.starts_with(&entry_prefix_str) {
            continue;
        }
        
        let k_str = key_str.strip_prefix(&entry_prefix_str).expect("Invalid entry key");
        let k: K = k_str.parse().expect("Failed to parse key");
        let entry: Entry<V, A> = bincode::deserialize(value.value()).expect("Failed to deserialize entry");
        entries.insert(k, entry);
    }

    // Load deferred operations
    let mut deferred = HashMap::new();
    let deferred_prefix = format!("{}/deferred/", map_name).into_bytes();
    
    for entry in table.iter().expect("Failed to iterate table") {
        let (key, value) = entry.expect("Failed to get entry");
        let key_bytes = key.value();
        
        if !starts_with(key_bytes, &deferred_prefix) {
            continue; // Not part of our prefix
        }
        
        let key_str = String::from_utf8(key_bytes.to_vec()).expect("Invalid UTF-8 key");
        let deferred_prefix_str = String::from_utf8(deferred_prefix.clone()).expect("Invalid UTF-8 prefix");
        
        if !key_str.starts_with(&deferred_prefix_str) {
            continue;
        }
        
        let (vclock, keys): (VClock<A>, BTreeSet<K>) =
            bincode::deserialize(value.value()).expect("Failed to deserialize deferred");
        deferred.insert(vclock, keys);
    }

    Map {
        clock,
        entries,
        deferred,
    }
}

/// Helper function to check if a byte slice starts with another byte slice
fn starts_with(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

/// Stores a TopicQueue<T> to redb.
pub fn store_topic_queue<T>(db: &Database, mq_name: &str, queue: &TopicQueue<T>)
where
    T: Serialize + Sha3Hash + Default + Debug + Clone + Ord,
{
    store_map(db, &format!("{}/topics", mq_name), &queue.topics);
}

/// Loads a TopicQueue<T> from redb.
pub fn load_topic_queue<T>(db: &Database, mq_name: &str) -> TopicQueue<T>
where
    T: DeserializeOwned + Sha3Hash + Default + Debug + Clone + Ord,
{
    let topics = load_map(db, &format!("{}/topics", mq_name));
    TopicQueue { topics }
}
