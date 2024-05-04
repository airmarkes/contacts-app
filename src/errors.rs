use axum::{http::StatusCode, response::IntoResponse};

use crate::get_time;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    //#[error("error accessing file")]
    //FileAccess(#[from] tokio::io::Error),
    //#[error("error parsing json")]
    //JsonParse(#[from] serde_json::Error),
    #[error("from anyhow")]
    AnyHow(#[from] anyhow::Error),
    #[error("from sqlx")]
    Sqlx(#[from] sqlx::Error),
    #[error("from askama")]
    Askama(#[from] askama::Error),
    #[error("from std io")]
    StdIo(#[from] std::io::Error),
}
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            //AppError::FileAccess(ioe) => {
            //    (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while accessing file: {ioe}"))
            //},
            //AppError::JsonParse(je) => {
            //    (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while parsing json file: {je}"))
            //},
            AppError::AnyHow(e) => {
                println!("->> {} - ERROR: {}", get_time(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Sqlx(e) => {
                println!("->> {} - ERROR: {}", get_time(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Askama(e) => {
                println!("->> {} - ERROR: {}", get_time(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::StdIo(e) => {
                println!("->> {} - ERROR: {}", get_time(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        //let body = Json(error_message);
        status.into_response()
    }
}

/*

#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
 */
