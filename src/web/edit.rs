use askama::Template;
use axum::extract::Query;
use axum::response::Redirect;
use axum::routing::get;
use axum::Form;
use axum::{extract::State, Router};
use axum_messages::Messages;
use serde::Deserialize;

use crate::archiver::ArchiverState;
use crate::contacts::*;
use crate::errors::*;
use crate::{get_time, AppStateType};

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditContactTemplate {
    pub errors_t: CreationErrorState,
    pub contact_t: Contact,
    pub archive_t: ArchiverState,
}
#[derive(Deserialize)]
pub struct EditContactIDParam {
    pub id_p: u32,
}

#[derive(Deserialize)]
pub struct EditContactParams {
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
    use askama_axum::IntoResponse;

    use super::*;

    pub async fn handler_get_editcontact(
        State(state): State<AppStateType>,
        Query(params): Query<EditContactIDParam>,
    ) -> Result<impl IntoResponse, AppError> {
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
            archive_t: state.read().await.archiver_state.clone(),
        };
        Ok(edit_contact_template.into_response())
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
        let contact = Contact {
            id: params_query.id_p as i64,
            first_name: params_form.first_p,
            last_name: params_form.last_p,
            phone: params_form.phone_p,
            email: params_form.email_p,
            birth_date: params_form.birth_p,
            time_creation: String::default(),
        };

        let pool = state.read().await.contacts_state.clone();

        let new_error = contact.check_errors(&pool).await?;

        match new_error {
            None => {
                let rows_affected = Contact::edit_contact(pool, contact).await?;
                match rows_affected {
                    1 => {
                        println!("Updated Successfully");
                        messages.info(
                            format!("Contact ID {} Updated Successfully!", params_query.id_p)
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
                    contact.id, contact.first_name, contact.last_name, contact.phone, contact.email,
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }
}
