use serde::Deserialize;
use sqlx::{prelude::FromRow, Pool, Sqlite};
//use std::sync::{Arc, RwLock};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub contacts_state: Pool<Sqlite>,
    pub error_state: CreationErrorState,
    pub archiver_state: ArchiverState,
}
pub type AppStateType = Arc<RwLock<AppState>>;

#[derive(Debug, Default, Clone, Deserialize, FromRow)]
pub struct Contact {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email: String,
    pub birth_date: String,
    pub time_creation: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CreationErrorState {
    pub first_error: String,
    pub last_error: String,
    pub phone_error: String,
    pub email_error: String,
    pub email_unique_error: String,
    pub birth_error: String,
}

#[derive(Clone, Deserialize)]
pub struct ArchiverState {
    pub archive_status: String,
    pub archive_progress: f64,
}
impl Default for ArchiverState {
    fn default() -> Self {
        ArchiverState {
            archive_status: "Waiting".to_owned(),
            archive_progress: 0.0,
        }
    }
}
impl ArchiverState {
    pub fn status(&self) -> String {
        self.archive_status.clone()
    }
    pub fn progress(&self) -> f64 {
        self.archive_progress
    }
    pub fn archive_file(&self) -> &str {
        //return "/db/contacts.db";
        return "D:/RustProjects/axum-3-htmx/db/contacts.db";
    }
}
