use axum::{http::StatusCode, response::IntoResponse, Json};

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    //#[error("error accessing file")]
    //FileAccess(#[from] tokio::io::Error),
    //#[error("error parsing json")]
    //JsonParse(#[from] serde_json::Error),
    #[error("error anyhow")]
    AnyHow(#[from] anyhow::Error),
    #[error("error connecting db")]
    DB(#[from] sqlx::Error),
}
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            //AppError::FileAccess(ioe) => {
            //    (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while accessing file: {ioe}"))
            //},
            //AppError::JsonParse(je) => {
            //    (StatusCode::INTERNAL_SERVER_ERROR, format!("Error while parsing json file: {je}"))
            //},
            AppError::AnyHow(ahe) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error from anyhow: {ahe}"),
            ),
            AppError::DB(dbe) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error from sqlx: {dbe}"),
            ),
        };
        let body = Json(error_message);
        (status, body).into_response()
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
