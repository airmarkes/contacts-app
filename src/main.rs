#![allow(unused)]
pub mod errors;
pub mod models;
pub mod params;
pub mod templates;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, head, post},
    Form, Json, Router,
};
use axum_extra::extract::Form as ExtraForm;
use axum_macros::debug_handler;
use chrono::prelude::*;
use core::num;
use dotenv::dotenv;
use rand::prelude::*;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Pool, Sqlite};
use sqlx::{sqlite::SqliteRow, Row};
use std::env;
use std::{
    fmt::write,
    sync::{Arc, RwLock, RwLockWriteGuard},
    thread::{self, yield_now},
    time::Duration,
};
use tokio::time::{sleep, Duration as TokioDuration};
use tokio::{fs::File, io::AsyncReadExt, net::TcpListener};
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;
use axum_messages::{Messages, MessagesManagerLayer};
use tower_sessions::{MemoryStore, SessionManagerLayer};

use errors::*;
use models::*;
use params::*;
use templates::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {    
    dotenv().ok();
    let db_url: String = std::env::var("DATABASE_URL")?;
    //let db_url = "sqlite:db/contacts.db";
    let pool: Pool<Sqlite> = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let app_state = AppState {
        contacts_state: pool,
        error_state: CreationErrorState::default(),
        archiver_state: ArchiverState::default(),
    };
    let app_state = Arc::new(RwLock::new(app_state));

    let app = Router::new()
        .route("/", get(handler_root))
        .route("/overview", get(handler_overview))
        .merge(contacts_management().with_state(app_state))
        .layer(MessagesManagerLayer)
        .layer(session_layer)
        .nest_service("/assets", ServeDir::new("assets"));

    //let socket = "127.0.0.1:8080";
    let socket = "0.0.0.0:8080";
    let listener = TcpListener::bind(socket).await?;
    println!("Listening on {}\n", socket);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handler_root() -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_root", get_time());
    let root_tmpl = RootTemplate { name: "Guest!" };
    Ok(Html(root_tmpl.render()?))
}

async fn handler_overview() -> Result<impl IntoResponse, AppError> {
    let overview_tmpl = OverviewTemplate {};
    Ok(Html(overview_tmpl.render()?))
}

fn contacts_management() -> Router<AppStateType> {

    #[debug_handler]
    async fn handler_get_showcontacts(
        State(state): State<AppStateType>,
        Query(params): Query<ShowParams>,
        headers: HeaderMap,
        messages: Messages
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_showcontacts", get_time());
        let search_bar = params.search_p.as_deref().unwrap_or("");
        let mut page_set = params.page_p.unwrap_or(1);

        let archiver = state.read().unwrap().archiver_state.clone();

        let messages = messages
            .into_iter()
            .map(|message| format!("{}: {}", message.level, message))
            .collect::<Vec<_>>()
            .join(", ");


        let pool = state.read().unwrap().contacts_state.clone();

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
        let mut writable_state = state.write().unwrap();
        writable_state.error_state = CreationErrorState::default();
        thread::sleep(Duration::from_millis(300));

        let header_hx = headers.get("HX-Trigger");
        match header_hx {
            Some(header_value) => {
                match header_value.to_str().unwrap() {
                    "search" => {
                        return (Ok((
                            [(header::VARY, "HX-Trigger")],
                            Html(rows_tmpl.render()?),
                        )));
                    }
                    _ => {
                        return (Ok((
                            [(header::VARY, "HX-Trigger")],
                            Html(contacts_tmpl.render()?),
                        )));
                    }
                };
            }
            None => {
                return (Ok((
                    [(header::VARY, "HX-Trigger")],
                    Html(contacts_tmpl.render()?),
                )));
            }
        };
    }

    async fn handler_get_newcontact(
        State(state): State<AppStateType>,
        Query(params): Query<NewContactParams>,
    ) -> Result<impl IntoResponse, AppError>  {
        println!("->> {} - HANDLER: handler_get_newcontact", get_time());
        let errors_all = state.read().unwrap().error_state.clone();
        let first_bar = params.first_p.as_deref().unwrap_or("");
        let last_bar = params.last_p.as_deref().unwrap_or("");
        let phone_bar = params.phone_p.as_deref().unwrap_or("");
        let email_bar = params.email_p.as_deref().unwrap_or("");
        let birth_bar = params.birth_p.as_deref().unwrap_or("");

        let mut writable_state: RwLockWriteGuard<'_, AppState> = state.write().unwrap();

        let new_contact_templ = NewContactTemplate {
            errors_t: errors_all,
            first_t: first_bar,
            last_t: last_bar,
            phone_t: phone_bar,
            email_t: email_bar,
            birth_t: birth_bar,
        };
        Ok(Html(new_contact_templ.render()?))
    }

    async fn handler_post_newcontact(
        State(state): State<AppStateType>,
        messages: Messages,
        Form(params): Form<NewContactParams>,        
    ) -> Result<Redirect, AppError> {
        println!("->> {} - HANDLER: handler_post_newcontact", get_time());
        let first_set = params.first_p.unwrap();
        let last_set = params.last_p.unwrap();
        let phone_set = params.phone_p.unwrap();
        let email_set = params.email_p.unwrap();
        let birth_set = params.birth_p.unwrap();

        let id_set: Option<u32> = None;

        let pool = state.read().unwrap().contacts_state.clone();

        let errors_all = CreationErrorState::default();
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
                
                let mut writable_state = state.write().unwrap();
                writable_state.error_state = CreationErrorState::default();
                Ok(Redirect::to("/contacts/show?page_p=1"))
            }
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();
                writable_state.error_state = new_error;
                let uri = format!(
                    "/contacts/new?first_p={}&last_p={}&phone_p={}&email_p={}",
                    first_set, last_set, phone_set, email_set
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }

    async fn handler_get_viewcontact(
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

    async fn handler_get_editcontact(
        State(state): State<AppStateType>,
        Query(params): Query<EditContactParams>,
    ) -> Result<Html<String>, AppError> {
        println!("->> {} - HANDLER: handler_get_editcontact", get_time());
        let errors_all = state.read().unwrap().error_state.clone();
        let id_set = params.id_p.unwrap();

        let pool = state.read().unwrap().contacts_state.clone();
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

    async fn handler_post_editcontact(
        State(state): State<AppStateType>,
        Query(params_query): Query<EditContactParams>,
        messages: Messages,
        Form(params_form): Form<EditContactParams>,
    ) -> Result<Redirect, AppError> {
        println!("->> {} - HANDLER: handler_post_editcontact", get_time());
        let id_set = params_query.id_p;
        let first_set = params_form.first_p.unwrap();
        let last_set = params_form.last_p.unwrap();
        let phone_set = params_form.phone_p.unwrap();
        let email_set = params_form.email_p.unwrap();
        let birth_set = params_form.birth_p.unwrap();

        let pool = state.read().unwrap().contacts_state.clone();

        let errors_all = CreationErrorState::default();
        let new_error = check_errors(
            &pool, &first_set, &last_set, &phone_set, &email_set, &birth_set, &id_set,
        )
        .await?;

        match new_error {
            None => {
                let (rows_affected) = edit_contact(
                    pool,
                    first_set,
                    last_set,
                    phone_set,
                    email_set,
                    birth_set,
                    id_set.unwrap(),
                )
                .await?;
                match rows_affected {
                    1 => {
                        println!("Updated Successfully");
                        messages
                        .info(format!("Contact ID {} Updated Successfully!", id_set.unwrap()).to_string());
        
                    }
                    _ => println!("Updated Unsuccessfully"),
                };
                Ok(Redirect::to("/contacts/show?page_p=1"))
            }
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();
                writable_state.error_state = new_error;
                let uri = format!(
                    "/contacts/edit?id_p={}&first_p={}&last_p={}&phone_p={}&email_p={}",
                    id_set.unwrap(),
                    first_set,
                    last_set,
                    phone_set,
                    email_set
                );
                Ok(Redirect::to(uri.as_str()))
            }
        }
    }

    async fn handler_delete_contact(
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
                let mut writable_state = state.write().unwrap();
                
                messages
                .info(format!("Contact ID {} Deleted Successfully!", id_set).to_string());

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

    async fn handler_delete_bulk(
        State(state): State<AppStateType>,
        ExtraForm(params_form): ExtraForm<DeleteBulkParams>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_delete_bulk", get_time());
        let ids_opt: Option<Vec<String>> = params_form.ids_p;
        let pool = state.read().unwrap().contacts_state.clone();
        let mut rows_affected_sum: u32 = 0;
        match (ids_opt) {
            Some(ids_set) => {
                let ids_u32 = ids_set
                    .into_iter()
                    .map(|u| u.parse::<u32>().unwrap())
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
                    rows_affected_sum += 1;
                }
                match rows_affected_sum {
                    0 => {
                        println!("Deleted UnSuccessfully");
                    }
                    _ => println!("Deleted Successfully {} Contacts", rows_affected_sum),
                };
                return Ok(Redirect::to("/contacts/show?page_p=1"));
            }
            None => return Ok(Redirect::to("/contacts/show?page_p=1")),
        }
    }

    async fn handler_get_validate_email(
        State(state): State<AppStateType>,
        Query(params_query): Query<ValidateEmailParams>,
    ) -> Result<String, AppError> {
        println!("->> {} - HANDLER: handler_get_validate_email", get_time());
        let email_set = params_query.email_p.unwrap();
        let id_set_opt = params_query.id_p;

        let pool = state.read().unwrap().contacts_state.clone();
        let email_validated = validate_email(&pool, &email_set, &id_set_opt).await?;
        Ok(email_validated)
    }

    async fn handler_contacts_count(
        State(state): State<AppStateType>, //State(state_contacts): State<ContactState>
    ) -> Result<String, AppError> {
        println!("->> {} - HANDLER: handler_contacts_count", get_time());
        let pool = state.read().unwrap().contacts_state.clone();

        let rec = sqlx::query!(
            r#"
            SELECT COUNT(*) as count 
            FROM contacts_table
            "#
        )
        .fetch_one(&pool)
        .await?;
        let contacts_count = rec.count;
        let span = format!("({} total contacts)", contacts_count);
        thread::sleep(Duration::from_millis(900));
        Ok(span)
    }

    async fn handler_post_archive(
        State(state): State<AppStateType>, //State(state_archive): State<ArchiverState>
    ) -> Result<impl IntoResponse, AppError>  {
        println!("->> {} - HANDLER: handler_post_archive", get_time());
        let archiver = state.read().unwrap().archiver_state.clone();
        if archiver.archive_status == "Waiting".to_owned() {
            let mut write = state.write().unwrap();
            write.archiver_state.archive_status = "Running".to_owned();
            write.archiver_state.archive_progress = 0.0;
            drop(write);
            let clone = state.clone();
            let handle = tokio::spawn(async move {
                run_thread(clone).await;
            });
        };
        let archiver_then = state.read().unwrap().archiver_state.clone();

        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver_then,
        };
        Ok(Html(archive_ui.render()?))
    }

    async fn handler_get_archive(
        State(state): State<AppStateType>)
     -> Result<impl IntoResponse, AppError>  {
        println!("->> {} - HANDLER: handler_get_archive", get_time());
        let archiver = state.read().unwrap().archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver,
        };
        Ok(Html(archive_ui.render()?))
    }

    async fn handler_get_archive_file(
        State(state): State<AppStateType>) 
        -> Result<impl IntoResponse, AppError>  {
        println!("->> {} - HANDLER: handler_get_archive_file", get_time());
        let archiver = state.read().unwrap().archiver_state.clone();
        let file = tokio::fs::File::open(archiver.archive_file()).await?;
        let stream = ReaderStream::new(file);
        let body = axum::body::Body::from_stream(stream);
        let headers = [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"contacts.db\"",
            ),
        ];
        Ok((headers, body))
    }

    async fn handler_delete_archive_file(
        State(state): State<AppStateType>) 
        -> Result<impl IntoResponse, AppError>  {
        println!("->> {} - HANDLER: handler_delete_archive_file", get_time());
        let mut write = state.write().unwrap();
        write.archiver_state.archive_status = "Waiting".to_owned();
        drop(write);
        let archiver = state.read().unwrap().archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver,
        };
        Ok(Html(archive_ui.render()?))
    }

    Router::new()
        .route(
            "/contacts/show",
            get(handler_get_showcontacts).delete(handler_delete_bulk),
        )
        .route(
            "/contacts/new",
            get(handler_get_newcontact).post(handler_post_newcontact),
        )
        .route(
            "/contacts/view",
            get(handler_get_viewcontact).delete(handler_delete_contact),
        )
        .route(
            "/contacts/edit",
            get(handler_get_editcontact).post(handler_post_editcontact),
        )
        .route("/contacts/validate_email", get(handler_get_validate_email))
        .route("/contacts/count", get(handler_contacts_count))
        .route(
            "/contacts/archive",
            post(handler_post_archive).get(handler_get_archive),
        )
        .route(
            "/contacts/archive/file",
            get(handler_get_archive_file).delete(handler_delete_archive_file),
        )
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
