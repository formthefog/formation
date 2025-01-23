use serde::{Serialize, Deserialize};

pub trait DatastoreType {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sqlite;

impl DatastoreType for Sqlite {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtMap;

impl DatastoreType for CrdtMap {}
