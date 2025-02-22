use rocksdb::{DB, WriteBatch, Options};
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
pub type DbHandle = Arc<DB>;

/// Opens a RocksDB database at the specified path.
/// Creates the database if it doesn’t exist.
pub fn open_db(path: PathBuf) -> DbHandle {
    let mut options = Options::default();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("Failed to create db directory");
        }
    }
    options.create_if_missing(true); // Create the DB if it doesn’t exist
    let db = DB::open(&options, path).expect("Failed to open RocksDB");
    Arc::new(db)
}

/// Stores a Map<K, V, A> to RocksDB.
pub fn store_map<K, V, A>(db: &DB, map_name: &str, map: &Map<K, V, A>) -> Result<(), Box<dyn std::error::Error>>
where
    K: Serialize + Ord + ToString,
    V: Serialize + CmRDT + ResetRemove<A> + Clone + Default,
    A: Serialize + Ord + Hash + Clone,
{
    let mut batch = WriteBatch::default();

    // Store the clock
    let clock_key = format!("{}/clock", map_name);
    let clock_bytes = serialize(&map.clock)?;
    batch.put(clock_key, clock_bytes);

    // Store the entries
    for (k, entry) in map.entries.iter() {
        let entry_key = format!("{}/entries/{}", map_name, k.to_string());
        let entry_bytes = serialize(entry)?;
        batch.put(entry_key, entry_bytes);
    }

    // Store deferred operations (using a simple index for simplicity)
    for (idx, (vclock, keys)) in map.deferred.iter().enumerate() {
        let deferred_key = format!("{}/deferred/{}", map_name, idx);
        let cloned_clock = (*vclock).clone();
        let deferred_bytes = serialize(&(cloned_clock, keys))?;
        batch.put(deferred_key, deferred_bytes);
    }

    // Write all changes atomically
    db.write(batch)?;
    Ok(())
}

/// Loads a Map<K, V, A> from RocksDB.
pub fn load_map<K, V, A>(db: &DB, map_name: &str) -> Result<Map<K, V, A>, Box<dyn std::error::Error>>
where
    K: DeserializeOwned + Ord + FromStr, // FromStr needed to parse keys
    <K as FromStr>::Err: std::fmt::Debug + std::error::Error + 'static, // Required for unwrap
    V: DeserializeOwned + CmRDT + ResetRemove<A> + Clone + Default,
    A: DeserializeOwned + Ord + Hash,
{
    // Load the clock
    let clock_key = format!("{}/clock", map_name);
    let clock = match db.get(&clock_key)? {
        Some(bytes) => deserialize(&bytes)?,
        None => VClock::new(), // Default if not found (e.g., first run)
    };

    // Load the entries
    let mut entries = BTreeMap::new();
    let entry_prefix = format!("{}/entries/", map_name);
    for result in db.prefix_iterator(entry_prefix.as_bytes()) {
        let (key, value) = result?;
        let key_str = String::from_utf8(key.to_vec())?;
        if !key_str.starts_with(&entry_prefix) {
            break; // End of prefix
        }
        let k_str = key_str.strip_prefix(&entry_prefix).ok_or(
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unable to strip prefix"))
        )?;
        let k: K = k_str.parse()?;
        let entry: Entry<V, A> = deserialize(&value)?;
        entries.insert(k, entry);
    }

    // Load deferred operations
    let mut deferred = HashMap::new();
    let deferred_prefix = format!("{}/deferred/", map_name);
    for result in db.prefix_iterator(deferred_prefix.as_bytes()) {
        let (key, value) = result?;
        let key_str = String::from_utf8(key.to_vec())?;
        if !key_str.starts_with(&deferred_prefix) {
            break; // End of prefix
        }
        let (vclock, keys): (VClock<A>, BTreeSet<K>) =
            deserialize(&value)?;
        deferred.insert(vclock, keys);
    }

    Ok(Map {
        clock,
        entries,
        deferred,
    })
}

pub fn write_datastore(db: &DB, datastore: &DataStore) -> Result<(), Box<dyn std::error::Error>> {
    store_map(db, "network_state/peers", &datastore.network_state.peers)?;
    store_map(db, "network_state/cidrs", &datastore.network_state.cidrs)?;
    store_map(db, "network_state/assocs", &datastore.network_state.associations)?;
    store_map(db, "network_state/dns", &datastore.network_state.dns_state.zones)?;
    store_map(db, "instance_state/instances", &datastore.instance_state.map)?;
    store_map(db, "node_state/nodes", &datastore.node_state.map)?;

    Ok(())
}
