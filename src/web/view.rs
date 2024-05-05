use askama::Template;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::{extract::State, Router};
use axum::{http::HeaderMap, response::Response};
use axum_messages::Messages;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;

#[derive(Template)]
#[template(path = "view.html")]
pub struct ViewContactTemplate {
    pub contact_t: Contact,
}

#[derive(Deserialize)]
pub struct ViewContactParams {
    pub id_p: Option<u32>,
}

pub fn view_router() -> Router<AppStateType> {
    Router::new().route(
        "/contacts/view",
        get(self::get::handler_get_viewcontact).delete(self::delete::handler_delete_contact),
    )
}

mod get {
    use super::*;

    pub async fn handler_get_viewcontact(
        State(state): State<AppStateType>,
        Query(params): Query<ViewContactParams>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_viewcontact", get_time());
        let id_set = params.id_p.unwrap();

        let pool = state.read().unwrap().contacts_state.clone();
        let contact_set = sqlx::query_as!(
            Contact,
            r#"
            SELECT *
            FROM contacts_table
            WHERE id = ?
            "#,
            id_set
        )
        .fetch_one(&pool)
        .await?;
        let view_contact_template = ViewContactTemplate {
            contact_t: contact_set,
        };
        Ok(Html(view_contact_template.render()?))
    }
}

mod delete {
    use super::*;

    pub async fn handler_delete_contact(
        State(state): State<AppStateType>,
        Query(params_query): Query<ViewContactParams>,
        headers: HeaderMap,
        messages: Messages,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_delete_contact", get_time());
        let id_set = params_query.id_p.unwrap();
        let hx_header = headers.get("HX-trigger");

        let pool = state.read().unwrap().contacts_state.clone();
        let rows_affected = sqlx::query!(
            r#"
            DELETE FROM contacts_table
            WHERE id = ?1
            "#,
            id_set
        )
        .execute(&pool)
        .await?
        .rows_affected();

        match rows_affected {
            1 => {
                println!("Deleted Successfully");

                messages.info(format!("Contact ID {} Deleted Successfully!", id_set).to_string());
            }
            _ => println!("Deleted Unsuccessfully"),
        };
        match hx_header {
            Some(header_value) => match header_value.to_str().unwrap() {
                "delete_btn" => {
                    return Ok(TypeOr::Redir);
                }
                _ => {
                    return Ok(TypeOr::EmptyStr);
                }
            },
            None => {
                return Ok(TypeOr::EmptyStr);
            }
        };
    }

    enum TypeOr {
        Redir,
        EmptyStr,
    }
    impl IntoResponse for TypeOr {
        fn into_response(self) -> Response {
            match self {
                TypeOr::EmptyStr => {
                    return ([("HX-Trigger", "fire_reload")], "").into_response();
                }
                TypeOr::Redir => {
                    return Redirect::to("/contacts/show?page_p=1").into_response();
                }
            }
        }
    }
}