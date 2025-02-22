use std::fmt::Debug;
use std::sync::Arc;
use std::path::PathBuf;
use serde::{Serialize, de::DeserializeOwned};
use crdts::bft_topic_queue::TopicQueue; // Adjust to your actual crate path
use crdts::{merkle_reg::Sha3Hash, CmRDT, ResetRemove, vclock::VClock}; // Assuming this trait for T
use rocksdb::{DB, WriteBatch, Options};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use std::hash::Hash;
use std::str::FromStr;

// Placeholder imports (adjust to your actual crate paths)
use crdts::map::{Map, Entry};

/// Opens a RocksDB database at the specified path.
/// Creates the database if it doesn’t exist.
pub fn open_db(path: PathBuf) -> Arc<DB> {
    let mut options = Options::default();
    options.create_if_missing(true); // Create the DB if it doesn’t exist
    let db = DB::open(&options, path).expect("Failed to open RocksDB");
    Arc::new(db)
}

/// Stores a Map<K, V, A> to RocksDB.
pub fn store_map<K, V, A>(db: &DB, map_name: &str, map: &Map<K, V, A>)
where
    K: Serialize + Ord + ToString,
    V: Serialize + Clone + Default + CmRDT + ResetRemove<A>,
    A: Serialize + Ord + Hash + Clone,
{
    let mut batch = WriteBatch::default();

    // Store the clock
    let clock_key = format!("{}/clock", map_name);
    let clock_bytes = bincode::serialize(&map.clock).expect("Failed to serialize clock");
    batch.put(clock_key, clock_bytes);

    // Store the entries
    for (k, entry) in map.entries.iter() {
        let entry_key = format!("{}/entries/{}", map_name, k.to_string());
        let entry_bytes = bincode::serialize(entry).expect("Failed to serialize entry");
        batch.put(entry_key, entry_bytes);
    }

    // Store deferred operations (using a simple index for simplicity)
    for (idx, (vclock, keys)) in map.deferred.iter().enumerate() {
        let deferred_key = format!("{}/deferred/{}", map_name, idx);
        let cloned_clock = (*vclock).clone();
        let deferred_bytes = bincode::serialize(&(cloned_clock, keys)).expect("Failed to serialize deferred");
        batch.put(deferred_key, deferred_bytes);
    }

    // Write all changes atomically
    db.write(batch).expect("Failed to write map to RocksDB");
}

/// Loads a Map<K, V, A> from RocksDB.
pub fn load_map<K, V, A>(db: &DB, map_name: &str) -> Map<K, V, A>
where
    K: DeserializeOwned + Ord + FromStr, // FromStr needed to parse keys
    <K as FromStr>::Err: std::fmt::Debug, // Required for unwrap
    V: DeserializeOwned + Clone + Default + CmRDT + ResetRemove<A>,
    A: DeserializeOwned + Ord + Hash + Clone,
{
    // Load the clock
    let clock_key = format!("{}/clock", map_name);
    let clock = match db.get(&clock_key).expect("Failed to read clock") {
        Some(bytes) => bincode::deserialize(&bytes).expect("Failed to deserialize clock"),
        None => VClock::new(), // Default if not found (e.g., first run)
    };

    // Load the entries
    let mut entries = BTreeMap::new();
    let entry_prefix = format!("{}/entries/", map_name);
    for result in db.prefix_iterator(entry_prefix.as_bytes()) {
        let (key, value) = result.expect("Failed to iterate entries");
        let key_str = String::from_utf8(key.to_vec()).expect("Invalid UTF-8 key");
        if !key_str.starts_with(&entry_prefix) {
            break; // End of prefix
        }
        let k_str = key_str.strip_prefix(&entry_prefix).expect("Invalid entry key");
        let k: K = k_str.parse().expect("Failed to parse key");
        let entry: Entry<V, A> = bincode::deserialize(&value).expect("Failed to deserialize entry");
        entries.insert(k, entry);
    }

    // Load deferred operations
    let mut deferred = HashMap::new();
    let deferred_prefix = format!("{}/deferred/", map_name);
    for result in db.prefix_iterator(deferred_prefix.as_bytes()) {
        let (key, value) = result.expect("Failed to iterate deferred");
        let key_str = String::from_utf8(key.to_vec()).expect("Invalid UTF-8 key");
        if !key_str.starts_with(&deferred_prefix) {
            break; // End of prefix
        }
        let (vclock, keys): (VClock<A>, BTreeSet<K>) =
            bincode::deserialize(&value).expect("Failed to deserialize deferred");
        deferred.insert(vclock, keys);
    }

    Map {
        clock,
        entries,
        deferred,
    }
}

/// Stores a TopicQueue<T> to RocksDB.
pub fn store_topic_queue<T>(db: &DB, mq_name: &str, queue: &TopicQueue<T>)
where
    T: Serialize + Sha3Hash + Default + Debug + Clone + Ord,
{
    store_map(db, &format!("{}/topics", mq_name), &queue.topics);
}

/// Loads a TopicQueue<T> from RocksDB.
pub fn load_topic_queue<T>(db: &DB, mq_name: &str) -> TopicQueue<T>
where
    T: DeserializeOwned + Sha3Hash + Default + Debug + Clone + Ord,
{
    let topics = load_map(db, &format!("{}/topics", mq_name));
    TopicQueue { topics }
}
