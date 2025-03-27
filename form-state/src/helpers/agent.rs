use crate::datastore::{DataStore, DB_HANDLE, AccountRequest};
use crate::db::write_datastore;
use crate::agent::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};

pub async fn create_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn update_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn delete_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn get_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn list_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}
