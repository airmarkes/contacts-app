use askama::Template;
use axum::extract::Query;
use axum::response::{Html, Redirect};
use axum::routing::get;
use axum::Form;
use axum::{extract::State, Router};
use axum_messages::Messages;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::functions::*;

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditContactTemplate {
    pub errors_t: CreationErrorState,
    pub contact_t: Contact,
}
#[derive(Deserialize)]
pub struct EditContactIDParam {
    pub id_p: u32,
}

#[derive(Deserialize)]
pub struct EditContactParams {
    pub id_p: u32,
    pub first_p: String,
    pub last_p: String,
    pub phone_p: String,
    pub email_p: String,
    pub birth_p: String,
}

pub fn edit_router() -> Router<AppStateType> {
    Router::new().route(
        "/contacts/edit",
        get(self::get::handler_get_editcontact).post(self::post::handler_post_editcontact),
    )
}

mod get {
    use super::*;

    pub async fn handler_get_editcontact(
        State(state): State<AppStateType>,
        Query(params): Query<EditContactIDParam>,
    ) -> Result<Html<String>, AppError> {
        println!("->> {} - HANDLER: handler_get_editcontact", get_time());
        let errors_all = state.read().await.error_state.clone();
        let id_set = params.id_p;

        let pool = state.read().await.contacts_state.clone();
        let contact_set = sqlx::query_as!(
            Contact,
            r#"
            SELECT *
            FROM contacts_table
            WHERE id = ?1
            "#,
            id_set
        )
        .fetch_one(&pool)
        .await?;
        let edit_contact_template = EditContactTemplate {
            errors_t: errors_all,
            contact_t: contact_set,
        };
        Ok(Html(edit_contact_template.render()?))
    }
}

mod post {
    use super::*;

    pub async fn handler_post_editcontact(
        State(state): State<AppStateType>,
        Query(params_query): Query<EditContactIDParam>,
        messages: Messages,
        Form(params_form): Form<EditContactParams>,
    ) -> Result<Redirect, AppError> {
        println!("->> {} - HANDLER: handler_post_editcontact", get_time());
        let id_set = params_query.id_p;
        let first_set = params_form.first_p;
        let last_set = params_form.last_p;
        let phone_set = params_form.phone_p;
        let email_set = params_form.email_p;
        let birth_set = params_form.birth_p;

        let pool = state.read().await.contacts_state.clone();

        let new_error = check_errors(
            &pool, &first_set, &last_set, &phone_set, &email_set, &birth_set, &Some(id_set),
        )
        .await?;

        match new_error {
            None => {
                let rows_affected = edit_contact(
                    pool,
                    first_set,
                    last_set,
                    phone_set,
                    email_set,
                    birth_set,
                    id_set,
                )
                .await?;
                match rows_affected {
                    1 => {
                        println!("Updated Successfully");
                        messages.info(
                            format!("Contact ID {} Updated Successfully!", id_set)
                                .to_string(),
                        );
                    }
                    _ => println!("Updated Unsuccessfully"),
                };
                Ok(Redirect::to("/contacts/show?page_p=1"))
            }
            Some(new_error) => {
                let mut writable_state = state.write().await;
                writable_state.error_state = new_error;
                let uri = format!(
                    "/contacts/edit?id_p={}&first_p={}&last_p={}&phone_p={}&email_p={}",
                    id_set,
                    first_set,
                    last_set,
                    phone_set,
                    email_set
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }
}

