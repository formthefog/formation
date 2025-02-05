use serde::{Serialize, Deserialize};
use shared::Cidr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Success<T> {
    Some(T),
    List(Vec<T>),
    Relationships(Vec<(Cidr<String>, Cidr<String>)>),
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response<T> {
    Success(Success<T>),
    Failure { reason: Option<String> }
}

