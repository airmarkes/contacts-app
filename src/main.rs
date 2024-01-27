#![allow(unused)]
use std::{sync::{Arc, RwLock, RwLockWriteGuard}, fmt::write, thread, time::Duration};
use serde::Deserialize;
use tokio::{net::TcpListener, fs::File, io::AsyncReadExt};
use axum::{Router, routing::{get, post, head}, response::{IntoResponse, Html, Redirect, Response}, extract::{Query, State, Path}, Json, Form, http::{HeaderMap, header}};
use askama::Template;
use tower_http::services::ServeDir;
use axum_extra::extract::Form as ExtraForm;

pub mod contacts_app;
use contacts_app::*;

pub mod db;
use db::*;

#[tokio::main]
async fn main() {
    //let contact_state = Arc::new(RwLock::new(ContactState::default()));
    let db = connect_db().await.unwrap();
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
#[derive(Template)]
#[template(path = "root.html")]
struct RootTemplate<'a> {
    name: &'a str,
}

async fn handler_overview() -> impl IntoResponse {
    let overview_tmpl = OverviewTemplate{};
    Html(overview_tmpl.render().unwrap())
}
#[derive(Template)]
#[template(path = "overview.html")]
struct OverviewTemplate {}

fn contacts_management() -> Router<AppStateType> {
    async fn handler_get_showcontacts(
        State(state): State<AppStateType>,
        Query(params): Query<ShowParams>,
        headers: HeaderMap,
    ) -> impl IntoResponse {        
        let search_bar = params.search_p.as_deref().unwrap_or("");
        let contacts_all = state.read().unwrap().contacts_state.clone();  
        let max_page = contacts_all.len().div_ceil(10);
        let flash = state.read().unwrap().flash_state.clone();  
        let mut page_set = params.page_p.unwrap();
        if page_set <= 0 { page_set = 1;} 
        else if page_set > max_page { page_set = max_page;};
        let (contacts_set, length) = match_contacts(&search_bar, contacts_all, page_set);    
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
        }; 
            let mut writable_state = state.write().unwrap(); 
            writable_state.flash_state = FlashState::default();
            writable_state.error_state = CreationErrorState::default();
            thread::sleep(Duration::from_millis(300));

        let header_hx = headers.get("HX-Trigger");
        match header_hx {
            Some(header_value) => {
                match header_value.to_str().unwrap() {
                    "search" => { return ([(header::VARY, "HX-Trigger")],Html(rows_tmpl.render().unwrap())); },
                    _ => { return ([(header::VARY, "HX-Trigger")], Html(contacts_tmpl.render().unwrap())); },
                };        
            }
            None => {                
                return ([(header::VARY, "HX-Trigger")], Html(contacts_tmpl.render().unwrap())); 
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
    ) -> Redirect {            
        let contacts_all = state.read().unwrap().contacts_state.clone();        
        let errors_all = CreationErrorState::default();  
        let first_set = params.first_p.unwrap();
        let last_set = params.last_p.unwrap();
        let phone_set = params.phone_p.unwrap();
        let email_set = params.email_p.unwrap();
        let new_contact = create_contact(first_set, last_set, phone_set, email_set, &contacts_all);
        let new_error = check_errors(&new_contact, &contacts_all);
        match new_error {
            None => {                                
                let mut writable_state = state.write().unwrap();
                writable_state.flash_state = FlashState { flash: format!("Contact ID {} Created Successfully!",new_contact.id).to_string(), flash_count: 1};
                writable_state.contacts_state.push(new_contact);             
                writable_state.error_state = CreationErrorState::default(); 
                Redirect::to("/contacts/show?page_p=1") 
            }, 
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();            
                writable_state.error_state = new_error;
                writable_state.flash_state = FlashState::default();            
                let uri = format!("/contacts/new?first_p={}&last_p={}&phone_p={}&email_p={}",                                  
                new_contact.first, new_contact.last, new_contact.phone, new_contact.email);
                Redirect::to(uri.as_str()) 
            },    
        }
    }   
    async fn handler_get_viewcontact(
        State(state): State<AppStateType>,
        Query(params): Query<ViewContactParams>        
    ) -> impl IntoResponse {        
        let id_set = params.id_p.unwrap();
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let contact_set = contacts_all.into_iter().filter(|x| x.id == id_set).collect::<ContactState>().swap_remove(0);              
        let view_contact_template = ViewContactTemplate { contact_t: contact_set };
        Html(view_contact_template.render().unwrap())
    }
    async fn handler_get_editcontact (
        State(state): State<AppStateType>,
        Query(params): Query<EditContactParams>
    ) -> Html<String> {
        let errors_all = state.read().unwrap().error_state.clone();
        let id_set = params.id_p.unwrap();
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let contact_set = contacts_all.into_iter().filter(|x| x.id == id_set ).collect::<ContactState>().swap_remove(0);        
        let edit_contact_template = EditContactTemplate { errors_t: errors_all, contact_t: contact_set};
        Html(edit_contact_template.render().unwrap())

    }
    async fn handler_post_editcontact (
        State(state): State<AppStateType>,
        Query(params_query): Query<EditContactParams>,
        Form(params_form): Form<EditContactParams>
    ) -> Redirect {
        let id_set = params_query.id_p.unwrap();
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let contact_position = contacts_all.clone().into_iter().position(|x| x.id == id_set).unwrap(); 
        let contact_set = contacts_all.clone().into_iter().filter(|x| x.id == id_set ).collect::<ContactState>().swap_remove(0);                   
        let contacts_all_but = contacts_all.clone().into_iter().filter(|x| x.id != id_set).collect::<ContactState>();        
        let first_set = params_form.first_p.unwrap();
        let last_set = params_form.last_p.unwrap();
        let phone_set = params_form.phone_p.unwrap();
        let email_set = params_form.email_p.unwrap();
        let time_creation_set = &contact_set.time_creation;
        let edited_contact = edit_contact(contact_set.id, first_set, last_set, phone_set, email_set, contact_set.time_creation);
        let new_error = check_errors(&edited_contact, &contacts_all_but);
        match new_error {
            None => { 
                let mut writable_state = state.write().unwrap();
                writable_state.contacts_state.remove(contact_position);
                writable_state.contacts_state.insert(contact_position, edited_contact);
                writable_state.flash_state = FlashState { flash: format!("Contact {} Edited Successfully!",id_set).to_string(), flash_count: 1};
                Redirect::to("/contacts/show?page_p=1")
            },
            Some(new_error) => {
                let mut writable_state = state.write().unwrap();            
                writable_state.error_state = new_error;
                writable_state.flash_state = FlashState::default();            
                let uri = format!("/contacts/edit?id_p={}&first_p={}&last_p={}&phone_p={}&email_p={}",                                  
                edited_contact.id, edited_contact.first, edited_contact.last, edited_contact.phone, edited_contact.email);
                Redirect::to(uri.as_str()) 
            }
        }
         
  
    }
    async fn handler_delete_contact (
        State(state): State<AppStateType>,
        Query(params_query): Query<ViewContactParams>,
        headers: HeaderMap
    ) -> impl IntoResponse {
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let id_set = params_query.id_p.unwrap();
        let contact_position = contacts_all.into_iter().position(|x| x.id == id_set).unwrap();        
        let hx_header = headers.get("HX-trigger");
        let mut writable_state = state.write().unwrap();
        writable_state.contacts_state.remove(contact_position);
        writable_state.flash_state = FlashState { flash: format!("Contact with ID {} deleted successfully!",id_set).to_string(), flash_count: 1};
        match hx_header {
            Some(header_value) => {
                match header_value.to_str().unwrap() {
                    "delete_btn" => { return TypeOr::Redir; },
                    _ => { return TypeOr::EmptyStr; }
                }
            },
            None => {
                return TypeOr::EmptyStr;
            }
        };
    }
    async fn handler_delete_bulk (
        State(state): State<AppStateType>,
        ExtraForm (params_form): ExtraForm<DeleteBulkParams>        
    ) -> impl IntoResponse {
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let ids_opt: Option<Vec<String>> = params_form.ids_p;
        let mut contact_position: usize;        
        let mut writable_state = state.write().unwrap();
        let ids_usize: Vec<usize>;
        match (ids_opt) {
            Some(ids_set) => { 
                ids_usize = ids_set.into_iter().map(|u| {u.parse::<usize>().unwrap()}).collect::<Vec<usize>>();
                for (index, id_in) in ids_usize.iter().enumerate() {
                    contact_position = contacts_all.iter().position(|x| {&x.id == id_in}).unwrap();   
                    contact_position = contact_position - index;
                    writable_state.contacts_state.remove(contact_position);
                               
                }
                return Redirect::to("/contacts/show?page_p=1");
            },
            None =>{
                return Redirect::to("/contacts/show?page_p=1");
            }
        }
    }    
    async fn handler_get_validate_email (
        State(state): State<AppStateType>,
        Query(params_query): Query<ValidateEmailParams>
    ) -> String {
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let email_set = params_query.email_p.unwrap();
        let email_validated = validate_email(&contacts_all, &email_set);        
        email_validated        
    }
    async fn handler_contacts_count (
        State(state): State<AppStateType>
    ) -> String {
        let contacts_all = state.read().unwrap().contacts_state.clone();
        let contacts_count = contacts_all.len();
        let span = format!("({} total contacts)", contacts_count);
        thread::sleep(Duration::from_millis(900));
        span
    }
    Router::new()
    .route("/contacts/show", get(handler_get_showcontacts).delete(handler_delete_bulk))
    .route("/contacts/new", get(handler_get_newcontact).post(handler_post_newcontact)) 
    .route("/contacts/view", get(handler_get_viewcontact).delete(handler_delete_contact))
    .route("/contacts/edit", get(handler_get_editcontact).post(handler_post_editcontact))  
    .route("/contacts/validate_email", get(handler_get_validate_email))
    .route("/contacts/count", get(handler_contacts_count))
}

#[derive(Template)]
#[template(path = "show.html")]
struct ShowTemplate<'a> {
    contacts_t: ContactState,
    search_t: &'a str,
    flash_t: FlashState,
    length_t: usize,
    page_t: usize,
    max_page_t: usize,
}
#[derive(Debug, Deserialize)]
struct ShowParams {
    search_p: Option<String>,
    page_p: Option<usize>    
}
#[derive(Template)]
#[template(path = "show_rows.html")]
struct RowsTemplate {
    contacts_t: ContactState,
    length_t: usize,
    page_t: usize,
    max_page_t: usize
}
#[derive(Template)]
#[template(path = "new.html")]
struct NewContactTemplate<'a> {    
    errors_t: CreationErrorState,
    first_t: &'a str,
    last_t: &'a str,
    phone_t: &'a str,
    email_t: &'a str,
}
#[derive(Debug, Deserialize)]
struct NewContactParams {
    first_p: Option<String>, 
    last_p: Option<String>,
    phone_p: Option<String>,
    email_p: Option<String>,     
} 

#[derive(Template)]
#[template(path = "view.html")]
struct ViewContactTemplate {
    contact_t: Contact,
}
#[derive(Debug, Deserialize)]
struct ViewContactParams{
    id_p: Option<usize>
}

#[derive(Template)]
#[template(path = "edit.html")]
struct EditContactTemplate {
    errors_t: CreationErrorState,
    contact_t: Contact,
}
#[derive(Debug, Deserialize)]
struct EditContactParams{
    id_p: Option<usize>,
    first_p: Option<String>, 
    last_p: Option<String>,
    phone_p: Option<String>,
    email_p: Option<String>, 
}

#[derive(Debug, Deserialize)]
struct ValidateEmailParams{
    email_p: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteBulkParams{
    #[serde(rename = "ids_p")]
    ids_p: Option<Vec<String>>,
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
