use super::*;
use axum::http::StatusCode;
use thiserror::Error;

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
        flash_state: FlashState::default()
    };
    let app_state = Ok(app_state);
    app_state
}