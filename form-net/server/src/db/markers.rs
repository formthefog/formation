use serde::{Serialize, Deserialize};

pub trait DatastoreType {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Sqlite;

impl DatastoreType for Sqlite {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CrdtMap;

impl DatastoreType for CrdtMap {}
