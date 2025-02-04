use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crdts::bft_reg::RecoverableSignature;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    VmMessage,
    FormnetMessages,
    DnsMessage,
    PackManagerMessage,
    StateMessage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMessage<T> {
    event_id: [u8; 32],
    event_type: MessageType,
    payload: T,
    timestamp: u64,
    source_node: String,
    signature: RecoverableSignature
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Binary(Vec<u8>),
    String(String),
    Json(serde_json::Value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attributes {
    pub id: String,
    pub ty: String,
    pub source: String,
    pub datacontenttype: Option<String>,
    pub dataschema: Option<Url>,
    pub subject: Option<String>,
    pub time: Option<DateTime<Utc>>
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtenstionValue {
    String(String),
    Boolean(bool),
    Integer(i64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub attributes: Attributes,
    pub data: Option<Data>,
    pub extensions: HashMap<String, ExtenstionValue>
}

impl<T: DeserializeOwned> TryFrom<Data> for NetworkMessage<T> {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: Data) -> Result<Self, Self::Error> {
        match value {
            Data::Binary(data) => Ok(bincode::deserialize(&data)?),
            Data::String(data) => Ok(serde_json::from_str(&data)?),
            Data::Json(data) => Ok(serde_json::from_value(data)?),
        }
    }
}

impl<T: Serialize> TryFrom<NetworkMessage<T>> for Data {
    type Error = Box<dyn std::error::Error>;
    fn try_from(value: NetworkMessage<T>) -> Result<Self, Self::Error> {
        Ok(Data::Binary(bincode::serialize(&value)?))
    }
}
