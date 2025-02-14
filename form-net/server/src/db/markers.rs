use serde::{Serialize, Deserialize};

pub trait DatastoreType {}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Sqlite;

impl DatastoreType for Sqlite {}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CrdtMap;

impl DatastoreType for CrdtMap {}
