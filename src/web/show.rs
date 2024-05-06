use askama::Template;
use axum::extract::Query;
use axum::http::{header, HeaderMap};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::{extract::State, Router};
use axum_extra::extract::Form as ExtraForm;
use axum_messages::Messages;
use serde::Deserialize;
use std::thread;
use std::time::Duration;

use crate::errors::AppError;
use crate::functions::*;
use crate::models::*;

#[derive(Template)]
#[template(path = "show.html")]
pub struct ShowTemplate<'a> {
    pub contacts_t: Vec<Contact>,
    pub search_t: &'a str,
    pub messages_t: String,
    pub length_t: u32,
    pub page_t: u32,
    pub max_page_t: u32,
    pub archive_t: ArchiverState,
    pub time_t: String,
}

#[derive(Template)]
#[template(path = "show_rows.html")]
pub struct RowsTemplate {
    pub contacts_t: Vec<Contact>,
    pub length_t: u32,
    pub page_t: u32,
    pub max_page_t: u32,
}

#[derive(Deserialize)]
pub struct ShowParams {
    pub search_p: Option<String>,
    pub page_p: u32,
}

#[derive(Deserialize)]
pub struct DeleteBulkParams {
    #[serde(rename = "ids_p")]
    pub ids_p: Option<Vec<String>>,
}

pub fn show_router() -> Router<AppStateType> {
    Router::new().route(
        "/contacts/show",
        get(self::get::handler_get_showcontacts).delete(self::delete::handler_delete_bulk),
    )
}

mod get {
    use super::*;

    pub async fn handler_get_showcontacts(
        State(state): State<AppStateType>,
        Query(params): Query<ShowParams>,
        headers: HeaderMap,
        messages: Messages,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_showcontacts", get_time());

        let search_bar = params.search_p.as_deref().unwrap_or("");
        let page_set = params.page_p;

        let archiver = state.read().await.archiver_state.clone();

        let messages = messages
            .into_iter()
            .map(|message| format!("{}: {}", message.level, message))
            .collect::<Vec<_>>()
            .join(", ");

        let pool = state.read().await.contacts_state.clone();

        let (contacts_set, length, page_set, max_page) =
            match_contacts(pool, search_bar, page_set).await?;

        let time_now = get_time();

        let rows_tmpl = RowsTemplate {
            contacts_t: contacts_set.clone(),
            length_t: length,
            page_t: page_set,
            max_page_t: max_page,
        };
        let contacts_tmpl = ShowTemplate {
            messages_t: messages,
            search_t: search_bar,
            contacts_t: contacts_set,
            length_t: length,
            page_t: page_set,
            max_page_t: max_page,
            archive_t: archiver,
            time_t: time_now,
        };
        let mut writable_state = state.write().await;
        writable_state.error_state = CreationErrorState::default();
        thread::sleep(Duration::from_millis(300));

        let header_hx = headers.get("HX-Trigger");
        match header_hx {
            Some(header_value) => match header_value.to_str()? {
                "search" => Ok(([(header::VARY, "HX-Trigger")], Html(rows_tmpl.render()?))),
                _ => Ok((
                    [(header::VARY, "HX-Trigger")],
                    Html(contacts_tmpl.render()?),
                )),
            },
            None => Ok((
                [(header::VARY, "HX-Trigger")],
                Html(contacts_tmpl.render()?),
            )),
        }
    }
}

mod delete {
    use super::*;

    pub async fn handler_delete_bulk(
        State(state): State<AppStateType>,
        ExtraForm(params_form): ExtraForm<DeleteBulkParams>,
    ) -> anyhow::Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_delete_bulk", get_time());
        let ids_opt: Option<Vec<String>> = params_form.ids_p;
        let pool = state.read().await.contacts_state.clone();
        let mut rows_affected_sum: u32 = 0;
        match ids_opt {
            Some(ids_set) => {
                let ids_u32 = ids_set
                    .into_iter()
                    .map(|u| u.parse::<u32>().expect("failed to parse ids"))
                    .collect::<Vec<u32>>();
                for id_set in ids_u32 {
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
                    rows_affected_sum += rows_affected as u32;
                }
                match rows_affected_sum {
                    0 => {
                        println!("Deleted UnSuccessfully");
                    }
                    _ => println!("Deleted Successfully {} Contacts", rows_affected_sum),
                };
                Ok(Redirect::to("/contacts/show?page_p=1"))
            }
            None => Ok(Redirect::to("/contacts/show?page_p=1")),
        }
    }
}
