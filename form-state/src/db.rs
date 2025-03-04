use redb::{Database, TableDefinition, ReadableTable};
use std::path::PathBuf;
use std::sync::Arc;
use bincode::{serialize, deserialize};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use std::hash::Hash;
use std::str::FromStr;
use crdts::map::{Map, Entry};
use crdts::VClock;
use crdts::{CmRDT, ResetRemove};

use crate::datastore::DataStore;

/// Database handle wrapped in Arc for sharing across threads.
pub type DbHandle = Arc<Database>;

// Define our table for storing entries
const ENTRIES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("entries");

/// Opens a redb database at the specified path.
/// Creates the database if it doesn't exist.
pub fn open_db(path: PathBuf) -> DbHandle {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("Failed to create db directory");
        }
    }
    
    let db = Database::create(&path).expect("Failed to open redb database");
    
    // Create the tables if they don't exist
    let write_txn = db.begin_write().expect("Failed to begin write transaction");
    {
        let _ = write_txn.open_table(ENTRIES_TABLE).expect("Failed to open entries table");
    }
    write_txn.commit().expect("Failed to commit transaction");
    
    Arc::new(db)
}

/// Stores a Map<K, V, A> to redb.
pub fn store_map<K, V, A>(db: &Database, map_name: &str, map: &Map<K, V, A>) -> Result<(), Box<dyn std::error::Error>>
where
    K: Serialize + Ord + ToString,
    V: Serialize + CmRDT + ResetRemove<A> + Clone + Default,
    A: Serialize + Ord + Hash + Clone,
{
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(ENTRIES_TABLE)?;

        // Store the clock
        let clock_key = format!("{}/clock", map_name).into_bytes();
        let clock_bytes = serialize(&map.clock)?;
        table.insert(&clock_key[..], &clock_bytes[..])?;

        // Store the entries
        for (k, entry) in map.entries.iter() {
            let entry_key = format!("{}/entries/{}", map_name, k.to_string()).into_bytes();
            let entry_bytes = serialize(entry)?;
            table.insert(&entry_key[..], &entry_bytes[..])?;
        }

        // Store deferred operations (using a simple index for simplicity)
        for (idx, (vclock, keys)) in map.deferred.iter().enumerate() {
            let deferred_key = format!("{}/deferred/{}", map_name, idx).into_bytes();
            let cloned_clock = (*vclock).clone();
            let deferred_bytes = serialize(&(cloned_clock, keys))?;
            table.insert(&deferred_key[..], &deferred_bytes[..])?;
        }
    }
    
    // Write all changes atomically
    write_txn.commit()?;
    Ok(())
}

/// Loads a Map<K, V, A> from redb.
pub fn load_map<K, V, A>(db: &Database, map_name: &str) -> Result<Map<K, V, A>, Box<dyn std::error::Error>>
where
    K: DeserializeOwned + Ord + FromStr, // FromStr needed to parse keys
    <K as FromStr>::Err: std::fmt::Debug + std::error::Error + 'static, // Required for unwrap
    V: DeserializeOwned + CmRDT + ResetRemove<A> + Clone + Default,
    A: DeserializeOwned + Ord + Hash,
{
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(ENTRIES_TABLE)?;

    // Load the clock
    let clock_key = format!("{}/clock", map_name).into_bytes();
    let clock = match table.get(&clock_key[..])? {
        Some(bytes) => deserialize(bytes.value())?,
        None => VClock::new(), // Default if not found (e.g., first run)
    };

    // Load the entries
    let mut entries = BTreeMap::new();
    let entry_prefix = format!("{}/entries/", map_name).into_bytes();
    
    // Scan all keys and filter those that match the prefix
    for entry in table.iter()? {
        let (key, value) = entry?;
        let key_bytes = key.value();
        
        if !starts_with(key_bytes, &entry_prefix) {
            continue; // Not part of our prefix
        }
        
        let key_str = String::from_utf8(key_bytes.to_vec())?;
        let entry_prefix_str = String::from_utf8(entry_prefix.clone())?;
        
        if !key_str.starts_with(&entry_prefix_str) {
            continue;
        }
        
        let k_str = key_str.strip_prefix(&entry_prefix_str).ok_or(
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unable to strip prefix"))
        )?;
        let k: K = k_str.parse()?;
        let entry: Entry<V, A> = deserialize(value.value())?;
        entries.insert(k, entry);
    }

    // Load deferred operations
    let mut deferred = HashMap::new();
    let deferred_prefix = format!("{}/deferred/", map_name).into_bytes();
    
    for entry in table.iter()? {
        let (key, value) = entry?;
        let key_bytes = key.value();
        
        if !starts_with(key_bytes, &deferred_prefix) {
            continue; // Not part of our prefix
        }
        
        let key_str = String::from_utf8(key_bytes.to_vec())?;
        let deferred_prefix_str = String::from_utf8(deferred_prefix.clone())?;
        
        if !key_str.starts_with(&deferred_prefix_str) {
            continue;
        }
        
        let (vclock, keys): (VClock<A>, BTreeSet<K>) =
            deserialize(value.value())?;
        deferred.insert(vclock, keys);
    }

    Ok(Map {
        clock,
        entries,
        deferred,
    })
}

/// Helper function to check if a byte slice starts with another byte slice
fn starts_with(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

pub fn write_datastore(db: &Database, datastore: &DataStore) -> Result<(), Box<dyn std::error::Error>> {
    store_map(db, "network_state/peers", &datastore.network_state.peers)?;
    store_map(db, "network_state/cidrs", &datastore.network_state.cidrs)?;
    store_map(db, "network_state/assocs", &datastore.network_state.associations)?;
    store_map(db, "network_state/dns", &datastore.network_state.dns_state.zones)?;
    store_map(db, "instance_state/instances", &datastore.instance_state.map)?;
    store_map(db, "node_state/nodes", &datastore.node_state.map)?;

    Ok(())
}
