use askama::Template;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum_messages::{Message, Messages};
use serde::Deserialize;

use crate::{archiver::ArchiverState, users::AuthSession};

use crate::AppStateType;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    messages: Vec<Message>,
    next: Option<String>,
    archive_t: ArchiverState,
}
// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrlParam {
    next: Option<String>,
}
// This allows us to extract the authentication fields from forms. We use this
// to authenticate requests with the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct CredentialsParam {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

pub fn login_router() -> Router<AppStateType> {
    Router::new()
        .route("/login", post(self::post::login))
        .route("/login", get(self::get::login))
        .route("/logout", get(self::get::logout))
}
mod post {
    use crate::get_time;

    use super::*;

    pub async fn login(
        mut auth_session: AuthSession,
        messages: Messages,
        Form(creds): Form<CredentialsParam>,
    ) -> impl IntoResponse {
        println!("->> {} - HANDLER: handler_login", get_time());

        let user = match auth_session.authenticate(creds.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                messages.error("Invalid credentials");

                let mut login_url = "/login".to_string();
                if let Some(next) = creds.next {
                    login_url = format!("{}?next={}", login_url, next);
                };

                return Redirect::to(&login_url).into_response();
            }
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        println!("->> first");
        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        messages.success(format!("Successfully logged in as {}", user.username));

        if let Some(ref next) = creds.next {
            Redirect::to(next)
        } else {
            Redirect::to("/")
        }
        .into_response()
    }
}

mod get {
    use axum::extract::State;

    use super::*;

    pub async fn login(
        messages: Messages,
        State(state): State<AppStateType>,
        Query(NextUrlParam { next }): Query<NextUrlParam>,
    ) -> LoginTemplate {
        LoginTemplate {
            messages: messages.into_iter().collect(),
            next,
            archive_t: state.read().await.archiver_state.clone(),
        }
    }

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/login").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
