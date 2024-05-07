use askama::Template;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::Form;
use axum::{extract::State, Router};
use axum_messages::Messages;
use serde::Deserialize;

use crate::contacts::*;
use crate::errors::*;
use crate::{get_time, AppStateType};

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewContactTemplate {
    pub errors_t: CreationErrorState,
    pub contact_t: Contact,
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
        let contact = Contact {
            id: 0,
            first_name: params.first_p.unwrap_or("".to_owned()),
            last_name: params.last_p.unwrap_or("".to_owned()),
            phone: params.phone_p.unwrap_or("".to_owned()),
            email: params.email_p.unwrap_or("".to_owned()),
            birth_date: params.birth_p.unwrap_or("".to_owned()),
            time_creation: "".to_owned(),
        };
        let new_contact_templ = NewContactTemplate {
            errors_t: errors_all,
            contact_t: contact,
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
        let contact = Contact {
            id: 0,
            first_name: params
                .first_p
                .expect("Obligatory field, missing validation"),
            last_name: params.last_p.expect("Obligatory field, missing validation"),
            phone: params
                .phone_p
                .expect("Obligatory field, missing validation"),
            email: params
                .email_p
                .expect("Obligatory field, missing validation"),
            birth_date: params
                .birth_p
                .expect("Obligatory field, missing validation"),
            time_creation: String::default(),
        };
        let pool = state.read().await.contacts_state.clone();

        let new_error = contact.check_errors(&pool).await?;
        match new_error {
            None => {
                let id_inserted = contact.create_contact(pool).await?;

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
                    contact.first_name, contact.last_name, contact.phone, contact.email
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }
}
