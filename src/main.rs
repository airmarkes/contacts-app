#![allow(unused)]
pub mod contacts_app;
pub mod db;
pub mod templates;

use core::num;
use std::{sync::{Arc, RwLock, RwLockWriteGuard}, fmt::write, thread::{self, yield_now}, time::Duration};
use axum_macros::debug_handler;
use chrono::prelude::*;
use sqlx::{sqlite::SqliteRow, Row};
use tokio::{net::TcpListener, fs::File, io::AsyncReadExt};
//use tokio::sync::{RwLock, RwLockWriteGuard};
use axum::{extract::{Path, Query, State}, http::{header, HeaderMap, StatusCode}, response::{Html, IntoResponse, Redirect, Response}, routing::{get, head, post}, Form, Json, Router};
use tower_http::services::ServeDir;
use axum_extra::extract::Form as ExtraForm;
use askama::Template;
use tokio::time::{sleep, Duration as TokioDuration};
use tokio_util::io::ReaderStream;
//use tokio::sync::{RwLock as TokioRwLock, RwLockWriteGuard as TokioRwLockWriteGuard};
use rand::prelude::*;


use contacts_app::*;
use db::*;
use templates::*;

#[tokio::main]
async fn main() {
    //let contact_state = Arc::new(RwLock::new(ContactState::default()));
    let db: AppState = connect_db().await.unwrap();
    let app_state = Arc::new(RwLock::new(db));    
    let app = Router::new()
    .route("/", get(handler_root))    
    .route("/overview", get(handler_overview))   
    .merge(contacts_management().with_state(app_state))
    .nest_service("/assets", ServeDir::new("assets"));

    let socket = "127.0.0.1:8080";
    let listener = TcpListener::bind(socket).await.unwrap();
    println!("Listening on {}\n", socket);
    axum::serve(listener, app).await.unwrap();
}

async fn handler_root() -> impl IntoResponse {    
    let root_tmpl = RootTemplate { name: "Guest!"};
    Html(root_tmpl.render().unwrap())      
}

async fn handler_overview() -> impl IntoResponse {
    let overview_tmpl = OverviewTemplate{};
    Html(overview_tmpl.render().unwrap())
}

fn contacts_management() -> Router<AppStateType> {
    #[debug_handler]
    async fn handler_get_showcontacts(
        State(state): State<AppStateType>,
        Query(params): Query<ShowParams>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, AppError> { 
        let search_bar = params.search_p.as_deref().unwrap_or("");
        let mut page_set = params.page_p.unwrap();

        let archiver = state.read().unwrap().archiver_state.clone();
        let flash = state.read().unwrap().flash_state.clone();  
        let pool = state.read().unwrap().contacts_state.clone(); 

        let (contacts_set, length, page_set, max_page) = match_contacts(pool, search_bar, page_set).await?;
        
        let rows_tmpl = RowsTemplate {
            contacts_t: contacts_set.clone(),
            length_t: length,
            page_t: page_set,
            max_page_t: max_page,
        };  
        let contacts_tmpl = ShowTemplate {
            flash_t: flash, search_t: search_bar,
            contacts_t: contacts_set,
            length_t: length,
            page_t: page_set,
            max_page_t: max_page,
            archive_t: archiver,
        }; 
            let mut writable_state = state.write().unwrap(); 
            writable_state.flash_state = FlashState::default();
            writable_state.error_state = CreationErrorState::default();
            thread::sleep(Duration::from_millis(300));

        let header_hx = headers.get("HX-Trigger");
        match header_hx {
            Some(header_value) => {
                match header_value.to_str().unwrap() {
                    "search" => { return (Ok(([(header::VARY, "HX-Trigger")],Html(rows_tmpl.render().unwrap())))); },
                    _ => { return (Ok(([(header::VARY, "HX-Trigger")], Html(contacts_tmpl.render().unwrap())))); },
                };        
            }
            None => {                
                return (Ok(([(header::VARY, "HX-Trigger")], Html(contacts_tmpl.render().unwrap())))); 
            }
        };
                                      
        
    }

    async fn handler_get_newcontact(
        State(state): State<AppStateType>,    
        Query(params): Query<NewContactParams>
    ) -> impl IntoResponse {
        let errors_all = state.read().unwrap().error_state.clone();
        let first_bar = params.first_p.as_deref().unwrap_or("");
        let last_bar = params.last_p.as_deref().unwrap_or("");
        let phone_bar = params.phone_p.as_deref().unwrap_or("");
        let email_bar = params.email_p.as_deref().unwrap_or("");  

        let mut writable_state = state.write().unwrap(); 
        writable_state.flash_state = FlashState::default();      

        let new_contact_templ = NewContactTemplate{ 
            errors_t: errors_all, first_t: first_bar, last_t: last_bar, phone_t: phone_bar, email_t: email_bar};
        Html(new_contact_templ.render().unwrap())
    }

    async fn handler_post_newcontact(
        State(state): State<AppStateType>,
        Form(params): Form<NewContactParams>
    ) -> Result<Redirect, AppError> {  
        let first_set = params.first_p.unwrap();
        let last_set = params.last_p.unwrap();
        let phone_set = params.phone_p.unwrap();
        let email_set = params.email_p.unwrap();
        let id_set: Option<u32> = None;
        
        let pool = state.read().unwrap().contacts_state.clone(); 

        let errors_all = CreationErrorState::default();  
        let new_error = check_errors(&pool, &first_set, &last_set, &phone_set, &email_set, &id_set).await?;
        match new_error {
            None => { 
                let id_inserted = create_contact(pool, first_set, last_set, phone_set, email_set).await?;
                let mut writable_state = state.write().unwrap();
                writable_state.flash_state = FlashState { flash: format!("Contact ID {} Created Successfully!", id_inserted).to_string(), flash_count: 1};
                //writable_state.contacts_state.push(new_contact); 
                writable_state.error_state = CreationErrorState::default(); 
                Ok(Redirect::to("/contacts/show?page_p=1")) 
            }, 
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();            
                writable_state.error_state = new_error;
                writable_state.flash_state = FlashState::default();            
                let uri = format!("/contacts/new?first_p={}&last_p={}&phone_p={}&email_p={}",                                  
                first_set, last_set, phone_set, email_set);
                Ok(Redirect::to(uri.as_str())) 
            },    
        }
    }   
   
    async fn handler_get_viewcontact(
        State(state): State<AppStateType>,
        Query(params): Query<ViewContactParams>        
    ) -> Result<impl IntoResponse, AppError> {    
        let id_set = params.id_p.unwrap();
    
        let pool = state.read().unwrap().contacts_state.clone();
        let contact_set = sqlx::query_as!( Contact,
            r#"
            SELECT *
            FROM contacts_table
            WHERE id = ?
            "#, id_set
        ).fetch_one(&pool).await?; 
        //let contact_set = contacts_all.into_iter().filter(|x| x.id == id_set).collect::<ContactState>().swap_remove(0);              
/*         let contact_set = Contact{
            id: row.get("id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            phone: row.get("phone"),
            email: row.get("email"),
            time_creation: row.get("time_creation")
        };
 */        
        let view_contact_template = ViewContactTemplate { contact_t: contact_set };
        Ok(Html(view_contact_template.render().unwrap()))
    }
    
    async fn handler_get_editcontact (
        State(state): State<AppStateType>,
        Query(params): Query<EditContactParams>
    ) -> Result<Html<String>, AppError> {
        let errors_all = state.read().unwrap().error_state.clone();
        let id_set = params.id_p.unwrap();

        let pool = state.read().unwrap().contacts_state.clone();
        let contact_set = sqlx::query_as!(Contact,
            r#"
            SELECT *
            FROM contacts_table
            WHERE id = ?1
            "#, id_set
        )
        .fetch_one(&pool).await?; 
        //let contact_set = contacts_all.into_iter().filter(|x| x.id == id_set ).collect::<ContactState>().swap_remove(0);        
        let edit_contact_template = EditContactTemplate { errors_t: errors_all, contact_t: contact_set};
        Ok(Html(edit_contact_template.render().unwrap()))

    }
    
    async fn handler_post_editcontact (
        State(state): State<AppStateType>,
        Query(params_query): Query<EditContactParams>,
        Form(params_form): Form<EditContactParams>
    ) -> Result<Redirect, AppError> {
        let id_set = params_query.id_p;
        let first_set = params_form.first_p.unwrap();
        let last_set = params_form.last_p.unwrap();
        let phone_set = params_form.phone_p.unwrap();
        let email_set = params_form.email_p.unwrap();

        let pool = state.read().unwrap().contacts_state.clone();

        let errors_all = CreationErrorState::default();  
        let new_error = check_errors(&pool, &first_set, &last_set, &phone_set, &email_set, &id_set).await?;

        //let (edited_contact, contact_position) = contacts_all.edit_contact(id_set, first_set, last_set,phone_set, email_set);
        //let contacts_all_but = contacts_all.clone().into_iter().filter(|x| x.id != id_set).collect::<ContactState>();        
        //let new_error = contacts_all_but.check_errors(&edited_contact);
        
        match new_error {
            None => {
                let (rows_affected) = edit_contact(pool, first_set, last_set, phone_set, email_set, id_set.unwrap()).await?;
                match rows_affected {
                    1 => {
                        println!("Updated Successfully");
                        let mut writable_state = state.write().unwrap();
                        writable_state.flash_state = FlashState { flash: format!("Contact with ID {} Edited successfully!",id_set.unwrap()).to_string(), flash_count: 1};
                    },
                    _ => println!("Updated Unsuccessfully"),
                };
                //writable_state.contacts_state.remove(contact_position);
                //writable_state.contacts_state.insert(contact_position, edited_contact);
                Ok(Redirect::to("/contacts/show?page_p=1"))
            },
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();
                writable_state.error_state = new_error;
                writable_state.flash_state = FlashState::default();            
                let uri = format!("/contacts/edit?id_p={}&first_p={}&last_p={}&phone_p={}&email_p={}",                                  
                id_set.unwrap(), first_set, last_set, phone_set, email_set);
                Ok(Redirect::to(uri.as_str())) 
            }
        }
    }
    
    async fn handler_delete_contact (
        State(state): State<AppStateType>,
        Query(params_query): Query<ViewContactParams>,
        headers: HeaderMap
    ) -> Result<impl IntoResponse, AppError> {
        let id_set = params_query.id_p.unwrap();
        let hx_header = headers.get("HX-trigger");

        let pool = state.read().unwrap().contacts_state.clone();
        let rows_affected = sqlx::query!(
            r#"
            DELETE FROM contacts_table
            WHERE id = ?1
            "#, id_set
        ).execute(&pool).await?.rows_affected();
        
        //let contact_position = contacts_all.into_iter().position(|x| x.id == id_set).unwrap();        
        let mut writable_state = state.write().unwrap();
        //writable_state.contacts_state.remove(contact_position);
        match rows_affected {
            1 => {
                println!("Deleted Successfully");
                writable_state.flash_state = FlashState { flash: format!("Contact with ID {} deleted successfully!",id_set).to_string(), flash_count: 1};
            },
            _ => println!("Deleted Unsuccessfully"),
        };
        match hx_header {
            Some(header_value) => {
                match header_value.to_str().unwrap() {
                    "delete_btn" => { return Ok(TypeOr::Redir); },
                    _ => { return Ok(TypeOr::EmptyStr); }
                }
            },
            None => {
                return Ok(TypeOr::EmptyStr);
            }
        };
    }
    
    async fn handler_delete_bulk (
        State(state): State<AppStateType>,
        ExtraForm (params_form): ExtraForm<DeleteBulkParams>        
    ) -> Result<impl IntoResponse, AppError> {
        let ids_opt: Option<Vec<String>> = params_form.ids_p;
        //let mut contact_position: usize;        
        //let mut writable_state = state.write().unwrap();
        //let ids_usize: Vec<usize>;
        let pool = state.read().unwrap().contacts_state.clone();
        let mut rows_affected_sum: u32 = 0;
        match (ids_opt) {
            Some(ids_set) => { 
/*                 ids_usize = ids_set.into_iter().map(|u| {u.parse::<usize>().unwrap()}).collect::<Vec<usize>>();
                for (index, id_in) in ids_usize.iter().enumerate() {
                    contact_position = contacts_all.iter().position(|x| {&x.id == id_in}).unwrap();   
                    contact_position = contact_position - index;
                    writable_state.contacts_state.remove(contact_position);
                               
                };
 */             let ids_u32 = ids_set.into_iter().map(|u| {u.parse::<u32>().unwrap()}).collect::<Vec<u32>>();
                for id_set in ids_u32 {
                    let rows_affected = sqlx::query!(
                        r#"
                        DELETE FROM contacts_table
                        WHERE id = ?1
                        "#, id_set
                    ).execute(&pool).await?.rows_affected();
                    rows_affected_sum += 1; 
                };
                match rows_affected_sum {
                    0 => {
                        println!("Deleted UnSuccessfully");
                    },
                    _ => println!("Deleted Successfully {} Contacts", rows_affected_sum),
                };
                return Ok(Redirect::to("/contacts/show?page_p=1"))
            },
            None =>{
                return Ok(Redirect::to("/contacts/show?page_p=1"))
            }
        }
    }    
    
    async fn handler_get_validate_email (
        State(state): State<AppStateType>,
        Query(params_query): Query<ValidateEmailParams>
    ) -> Result<String, AppError> {
        let email_set = params_query.email_p.unwrap();
        let id_set_opt = params_query.id_p;

        let pool = state.read().unwrap().contacts_state.clone();
        let email_validated = validate_email(&pool, &email_set, &id_set_opt).await?;
        Ok(email_validated)

        //let email_validated = contacts_all_but.validate_email(&email_set);        
    }
    
    async fn handler_contacts_count (
        State(state): State<AppStateType>
        //State(state_contacts): State<ContactState>
    ) -> Result<String, AppError> {
        let pool = state.read().unwrap().contacts_state.clone();

        let rec = sqlx::query!(
            r#"
            SELECT COUNT(*) as count 
            FROM contacts_table
            "#
        ).fetch_one(&pool).await?;
        let contacts_count = rec.count;

        //let contacts_count = contacts_all.len();
        let span = format!("({} total contacts)", contacts_count);
        thread::sleep(Duration::from_millis(900));
        Ok(span)
    }
    
    async fn handler_post_archive (
        State(state): State<AppStateType>
        //State(state_archive): State<ArchiverState>
    ) -> impl IntoResponse {
        let archiver = state.read().unwrap().archiver_state.clone();
        //let archiver = archiver_type.read().unwrap().clone();  
        if archiver.archive_status == "Waiting".to_owned() {
                let mut write = state.write().unwrap();
                write.archiver_state.archive_status = "Running".to_owned();       
                write.archiver_state.archive_progress = 0.0;
                drop(write);
                //let new_lock = Arc::new(TokioRwLock::new(archiver));
                let clone = state.clone();
                let handle = tokio::spawn(async move {
                    run_thread(clone).await;
                });
                //tokio::join!(run_thread(clone),);
                
        //archiver.clone().run();
        //let archiver_then = state.read().unwrap().archiver_state.clone();    
        };
        let archiver_then = state.read().unwrap().archiver_state.clone();
        
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver_then,
        };
        Html(archive_ui.render().unwrap())
    }
    
    async fn handler_get_archive (
        State(state): State<AppStateType>
    ) -> impl IntoResponse {
        let archiver = state.read().unwrap().archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver,
        };
        Html(archive_ui.render().unwrap())
    }
    
    async fn handler_get_archive_file (
        State(state): State<AppStateType>
    ) -> impl IntoResponse {
        let archiver = state.read().unwrap().archiver_state.clone();
        let file = tokio::fs::File::open(archiver.archive_file()).await.unwrap();
        let stream = ReaderStream::new(file);
        let body = axum::body::Body::from_stream(stream);
        let filename = "archive.json";
        let headers = [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"archive.json\""),
            //&format!("attachment; filename=\"{:?}\"", filename)),
        ];
        (headers, body)
    }
    
    async fn handler_delete_archive_file (
        State(state): State<AppStateType>
    ) -> impl IntoResponse {
        let mut write = state.write().unwrap();
        write.archiver_state.archive_status = "Waiting".to_owned();
        drop(write);
        let archiver = state.read().unwrap().archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver
        };
        Html(archive_ui.render().unwrap())
    }
    
    Router::new()
    .route("/contacts/show", get(handler_get_showcontacts).delete(handler_delete_bulk))
    .route("/contacts/new", get(handler_get_newcontact).post(handler_post_newcontact)) 
    .route("/contacts/view", get(handler_get_viewcontact).delete(handler_delete_contact))
    .route("/contacts/edit", get(handler_get_editcontact).post(handler_post_editcontact))  
    .route("/contacts/validate_email", get(handler_get_validate_email))
    .route("/contacts/count", get(handler_contacts_count))
    .route("/contacts/archive", post(handler_post_archive).get(handler_get_archive))
    .route("/contacts/archive/file", get(handler_get_archive_file).delete(handler_delete_archive_file))
}

enum TypeOr {
    Redir,
    EmptyStr, 
}
impl IntoResponse for TypeOr
{
    fn into_response(self) -> Response {
        match self {
            TypeOr::EmptyStr => {return ([("HX-Trigger", "fire_reload")], "").into_response(); },
            TypeOr::Redir => { return Redirect::to("/contacts/show?page_p=1").into_response();}
        }
    }
}
struct AppError(anyhow::Error);
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (   
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong {}", self.0)
        ).into_response()
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