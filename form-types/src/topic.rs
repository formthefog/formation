use derive_more::Display;
use serde::{Serialize, Deserialize};
use form_traits::Topic;

#[derive(Clone, Debug, Serialize, Deserialize, Display)]
pub struct NetworkTopic;

impl Topic for NetworkTopic {}

#[derive(Clone, Debug, Serialize, Deserialize, Display)]
pub struct QuorumTopic;

impl Topic for QuorumTopic {}

#[derive(Clone, Debug, Serialize, Deserialize, Display)]
pub struct VmmTopic;

impl Topic for VmmTopic {}

#[derive(Clone, Debug, Serialize, Deserialize, Display)]
pub struct FormnetTopic;

impl Topic for FormnetTopic {}
