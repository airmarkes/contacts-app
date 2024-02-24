use super::*;
use sqlx::sqlite::SqlitePool;
use std::env;

//use rusqlite::{Connection, Result};
/* 
use axum::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError{
    #[error("error accessing DB")]
    ENVAccess(#[from] std::env::VarError),
    #[error("error parsing json")]
    DBAccess(#[from] sqlx::Error),
}
impl IntoResponse for DBError{
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            DBError::ENVAccess(ioe) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while accessing file: {ioe}"))
            },
            DBError::DBAccess(je) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while parsing json file: {je}"))
            },
        };
        let body = Json(error_message);
        (status, body).into_response()
    }
}

*/
/*
#[derive(Error, Debug)]
pub enum ContactError{
    #[error("error accessing file")]
    FileAccess(#[from] tokio::io::Error),
    #[error("error parsing json")]
    JsonParse(#[from] serde_json::Error),
}
impl IntoResponse for ContactError{
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ContactError::FileAccess(ioe) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while accessing file: {ioe}"))
            },
            ContactError::JsonParse(je) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while parsing json file: {je}"))
            },
        };
        let body = Json(error_message);
        (status, body).into_response()
    }
}

pub async fn connect_db() -> Result<AppState, ContactError> {
    let mut contents = String::new();
    File::open("D:/RustProjects/axum-3-htmx/assets/db.json").await?
        .read_to_string(&mut contents).await?;
    let contact_vec = serde_json::from_str::<ContactState>(&contents)?;
    let app_state = AppState {
        contacts_state: contact_vec,
        error_state: CreationErrorState::default(),
        flash_state: FlashState::default(),
        //archiver_state: Arc::new(RwLock::new(ArchiverState::default())),
        archiver_state: ArchiverState::default(),
    };
    let app_state = Ok(app_state);
    app_state 
}*/
/*pub async fn connect_db() -> Result<AppState> {
    let conn = Connection::open("my_db.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contacts_table (
            id INTEGER PRIMARY KEY,
            first TEXT NOT NULL,
            last TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT NOT NULL,
            time_creation TEXT NOT NULL
        )", 
        (),
    )?;    
    let mut contact_vec: ContactState = Vec::new();
    let me: Contact = Contact {
        id: 0,
        first: "Caio".to_string(),
        last: "Martins".to_string(),
        phone: "123".to_string(),
        email: "caio@hotmail.com".to_string(),
        time_creation: "old".to_string(),
    };
    contact_vec.push(me);
    let app_state = AppState {
        contacts_state: contact_vec,
        error_state: CreationErrorState::default(),
        flash_state: FlashState::default(),
        //archiver_state: Arc::new(RwLock::new(ArchiverState::default())),
        archiver_state: ArchiverState::default(),
    };
    
    Ok((app_state))
}*/
pub async fn connect_db() -> anyhow::Result<AppState> {
    let path: &'static str = env!("DATABASE_URL");
    let pool = SqlitePool::connect(path).await?;
   
    let app_state = AppState {
        contacts_state: pool,
        error_state: CreationErrorState::default(),
        flash_state: FlashState::default(),
        //archiver_state: Arc::new(RwLock::new(ArchiverState::default())),
        archiver_state: ArchiverState::default(),
    };
    
    Ok((app_state))
}