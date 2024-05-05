use askama::Template;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::Form;
use axum::{extract::State, Router};
use axum_messages::Messages;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::functions::*;

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewContactTemplate<'a> {
    pub errors_t: CreationErrorState,
    pub first_t: &'a str,
    pub last_t: &'a str,
    pub phone_t: &'a str,
    pub email_t: &'a str,
    pub birth_t: &'a str,
}

#[derive(Deserialize)]
pub struct NewContactParams {
    pub first_p: Option<String>,
    pub last_p: Option<String>,
    pub phone_p: Option<String>,
    pub email_p: Option<String>,
    pub birth_p: Option<String>,
}

pub fn new_router() -> Router<AppStateType> {
    Router::new().route(
        "/contacts/new",
        get(self::get::handler_get_newcontact).post(self::post::handler_post_newcontact),
    )
}

mod get {
    use super::*;

    pub async fn handler_get_newcontact(
        State(state): State<AppStateType>,
        Query(params): Query<NewContactParams>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_newcontact", get_time());
        let errors_all = state.read().await.error_state.clone();

        let new_contact_templ = NewContactTemplate {
            errors_t: errors_all,
            first_t: params.first_p.as_deref().unwrap_or(""),
            last_t: params.last_p.as_deref().unwrap_or(""),
            phone_t: params.phone_p.as_deref().unwrap_or(""),
            email_t: params.email_p.as_deref().unwrap_or(""),
            birth_t: params.birth_p.as_deref().unwrap_or(""),
        };
        Ok(Html(new_contact_templ.render()?))
    }
}

mod post {
    use super::*;

    pub async fn handler_post_newcontact(
        State(state): State<AppStateType>,
        messages: Messages,
        Form(params): Form<NewContactParams>,
    ) -> Result<Redirect, AppError> {
        println!("->> {} - HANDLER: handler_post_newcontact", get_time());
        let first_set = params.first_p.expect("Obligatory field, missing validation");
        let last_set = params.last_p.expect("Obligatory field, missing validation");
        let phone_set = params.phone_p.expect("Obligatory field, missing validation");
        let email_set = params.email_p.expect("Obligatory field, missing validation");
        let birth_set = params.birth_p.expect("Obligatory field, missing validation");
        let id_set: Option<u32> = None;

        let pool = state.read().await.contacts_state.clone();

        let new_error = check_errors(
            &pool, &first_set, &last_set, &phone_set, &email_set, &birth_set, &id_set,
        )
        .await?;
        match new_error {
            None => {
                let id_inserted =
                    create_contact(pool, first_set, last_set, phone_set, email_set, birth_set)
                        .await?;

                messages
                    .info(format!("Contact ID {} Created Successfully!", id_inserted).to_string());

                let mut writable_state = state.write().await;
                writable_state.error_state = CreationErrorState::default();
                Ok(Redirect::to("/contacts/show?page_p=1"))
            }
            Some(new_error) => {
                let mut writable_state = state.write().await;
                writable_state.error_state = new_error;
                let uri = format!(
                    "/contacts/new?first_p={}&last_p={}&phone_p={}&email_p={}",
                    first_set, last_set, phone_set, email_set
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }
}
