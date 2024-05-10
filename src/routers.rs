use crate::models::*;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_extra::extract::Form as ExtraForm;
use axum_messages::Level;
use axum_messages::{Message, Messages};
use serde::Deserialize;
use tokio_util::io::ReaderStream;

// region: INDEX

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate<'a> {
    pub name: &'a str,
    pub archive_t: ArchiverState,
}

pub fn index_router() -> Router<AppStateType> {
    Router::new().route("/", get(self::get::handler_root))
}

mod get {
    use super::*;

    pub async fn handler_root(
        State(state): State<AppStateType>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_root", get_time());
        let root_tmpl = RootTemplate {
            name: "Guest!",
            archive_t: state.read().await.archiver_state.clone(),
        };
        Ok(root_tmpl.into_response())
    }
}

// endregion: INDEX

// region: SHOW

#[derive(Template)]
#[template(path = "show.html")]
pub struct ShowTemplate<'a> {
    pub contacts_t: Vec<Contact>,
    pub search_t: &'a str,
    pub messages_t: Vec<Message>,
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
        get(handler_get_showcontacts).delete(handler_delete_bulk),
    )
}

pub async fn handler_get_showcontacts(
    State(state): State<AppStateType>,
    Query(params): Query<ShowParams>,
    headers: HeaderMap,
    messages: Messages,
    //auth_session: AuthSession,
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_get_showcontacts", get_time());
    //std::thread::sleep(std::time::Duration::from_millis(900));

    //if let Some(user) = auth_session.user {
    let search_bar = params.search_p.as_deref().unwrap_or("");
    let page_set = params.page_p;

    let archiver = state.read().await.archiver_state.clone();

    /* let messages = messages
    .into_iter()
    .map(|message| format!("{}: {}", message.level, message))
    .collect::<Vec<_>>()
    .join(", "); */

    let pool = state.read().await.contacts_state.clone();

    let (contacts_set, length, page_set, max_page) =
        Contacts::match_contacts(pool, search_bar, page_set).await?;

    let time_now = get_time();

    let rows_tmpl = RowsTemplate {
        contacts_t: contacts_set.contacts.clone(),
        length_t: length,
        page_t: page_set,
        max_page_t: max_page,
    };
    let contacts_tmpl = ShowTemplate {
        messages_t: messages.into_iter().collect(),
        search_t: search_bar,
        contacts_t: contacts_set.contacts,
        length_t: length,
        page_t: page_set,
        max_page_t: max_page,
        archive_t: archiver,
        time_t: time_now,
    };
    let mut writable_state = state.write().await;
    writable_state.error_state = CreationErrorState::default();

    let header_hx = headers.get("HX-Trigger");
    match header_hx {
        Some(header_value) => match header_value.to_str()? {
            "search" => Ok(([(header::VARY, "HX-Trigger")], rows_tmpl.into_response())),
            _ => Ok((
                [(header::VARY, "HX-Trigger")],
                contacts_tmpl.into_response(),
            )),
        },
        None => Ok((
            [(header::VARY, "HX-Trigger")],
            contacts_tmpl.into_response(),
        )),
    }
    /*} else {
        Ok((
            [(header::VARY, "HX-Trigger")],
            StatusCode::UNAUTHORIZED.into_response(),
        ))
    } */
}

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
            Ok(DoNothingOrRedirect::Redirect)
        }
        None => Ok(DoNothingOrRedirect::DoNothing),
    }
}

enum DoNothingOrRedirect {
    Redirect,
    DoNothing,
}
impl IntoResponse for DoNothingOrRedirect {
    fn into_response(self) -> askama_axum::Response {
        match self {
            DoNothingOrRedirect::Redirect => {
                Redirect::to("/contacts/show?page_p=1").into_response()
            }
            DoNothingOrRedirect::DoNothing => StatusCode::NOT_IMPLEMENTED.into_response(),
        }
    }
}

// endregion: SHOW

// region: VIEW

#[derive(Template)]
#[template(path = "view.html")]
pub struct ViewContactTemplate {
    pub contact_t: Contact,
    pub archive_t: ArchiverState,
}

#[derive(Deserialize)]
pub struct ViewContactParams {
    pub id_p: u32,
}

pub fn view_router() -> Router<AppStateType> {
    Router::new().route(
        "/contacts/view",
        get(handler_get_viewcontact).delete(handler_delete_contact),
    )
}

pub async fn handler_get_viewcontact(
    State(state): State<AppStateType>,
    Query(params): Query<ViewContactParams>,
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_get_viewcontact", get_time());
    let id_set = params.id_p;

    let pool = state.read().await.contacts_state.clone();
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
        archive_t: state.read().await.archiver_state.clone(),
    };
    Ok(view_contact_template.into_response())
}

pub async fn handler_delete_contact(
    State(state): State<AppStateType>,
    Query(params_query): Query<ViewContactParams>,
    headers: HeaderMap,
    messages: Messages,
) -> Result<impl IntoResponse, AppError> {
    println!(
        "->> {} - HANDLER: handler_delete_contact - MOD: view.rs",
        get_time()
    );
    let id_set = params_query.id_p as i64;
    let header_hx_trigger = headers.get("HX-trigger");

    let pool = state.read().await.contacts_state.clone();
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
    match header_hx_trigger {
        Some(header_value) => match header_value.to_str()? {
            "delete_btn" => Ok(Redirect::to("/contacts/show?page_p=1").into_response()),
            _ => Ok(([("HX-Trigger", "fire_reload")]).into_response()),
        },
        None => Ok(([("HX-Trigger", "fire_reload")]).into_response()),
    }
}

// endregion: VIEW

// region: NEW

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewContactTemplate {
    pub errors_t: CreationErrorState,
    pub contact_t: Contact,
    pub archive_t: ArchiverState,
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
        get(handler_get_newcontact).post(handler_post_newcontact),
    )
}

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
        archive_t: state.read().await.archiver_state.clone(),
    };
    Ok(Html(new_contact_templ.render()?))
}

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

            messages.info(format!("Contact ID {} Created Successfully!", id_inserted).to_string());

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

// endregion: NEW

// region: EDIT

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
        get(handler_get_editcontact).post(handler_post_editcontact),
    )
}

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
                1 => messages.success("Contact Updated!"),
                _ => messages.error("Contact Update Failed!"),
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

// endregion: EDIT

// region: UTILS

#[derive(Deserialize)]
pub struct ValidateEmailParams {
    pub email_p: String,
    pub id_p: Option<i64>,
}

pub fn utils_router() -> Router<AppStateType> {
    Router::new()
        .route("/contacts/validate_email", get(handler_get_validate_email))
        .route("/contacts/count", get(handler_get_count))
        .route("/utils/close-flash", get(handler_close_flash))
}

pub async fn handler_get_validate_email(
    State(state): State<AppStateType>,
    Query(params_query): Query<ValidateEmailParams>,
) -> Result<String, AppError> {
    println!("->> {} - HANDLER: handler_get_validate_email", get_time());
    let email_set = params_query.email_p;
    let id_set_opt = params_query.id_p;

    let pool = state.read().await.contacts_state.clone();
    match id_set_opt {
        Some(id) => {
            let email_validated = Contact::validate_email(&pool, &email_set, id).await?;
            Ok(email_validated)
        }
        None => {
            let id = 0;
            let email_validated = Contact::validate_email(&pool, &email_set, id).await?;
            Ok(email_validated)
        }
    }
}

pub async fn handler_get_count(
    State(state): State<AppStateType>, //State(state_contacts): State<ContactState>
) -> Result<String, AppError> {
    println!("->> {} - HANDLER: handler_contacts_count", get_time());
    let pool = state.read().await.contacts_state.clone();

    let rec = sqlx::query!(
        r#"
            SELECT COUNT(*) as count 
            FROM contacts_table
            "#
    )
    .fetch_one(&pool)
    .await?;
    let contacts_count = rec.count;
    let span = format!("{} contacts", contacts_count);
    //thread::sleep(Duration::from_millis(900));
    Ok(span)
}

pub async fn handler_close_flash() -> Result<String, AppError> {
    println!("->> {} - HANDLER: handler_close_flash", get_time());
    let span = format!("");
    Ok(span)
}

// endregion: UTILS

// region: ARCHIVE

#[derive(Template)]
#[template(path = "archive_ui.html")]
pub struct ArchiveUiTemplate {
    pub archive_t: ArchiverState,
}

pub fn archive_router() -> Router<AppStateType> {
    Router::new()
        .route(
            "/contacts/archive",
            get(handler_get_archive).post(handler_post_archive),
        )
        .route(
            "/contacts/archive/file",
            get(handler_get_archive_file).delete(handler_delete_archive_file),
        )
}

pub async fn handler_get_archive(
    State(state): State<AppStateType>,
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_get_archive", get_time());
    let archiver = state.read().await.archiver_state.clone();
    let archive_ui_tmpl = ArchiveUiTemplate {
        archive_t: archiver,
    };
    Ok(archive_ui_tmpl.into_response())
}

pub async fn handler_get_archive_file(
    State(state): State<AppStateType>,
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_get_archive_file", get_time());
    let archiver = state.read().await.archiver_state.clone();
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

pub async fn handler_post_archive(
    State(state): State<AppStateType>, //State(state_archive): State<ArchiverState>
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_post_archive", get_time());
    let archiver = state.read().await.archiver_state.clone();
    if archiver.archive_status == "Waiting" {
        let mut write = state.write().await;
        write.archiver_state.archive_status = "Running".to_owned();
        write.archiver_state.archive_progress = 0.0;
        drop(write);
        let clone = state.clone();
        let _handle = tokio::spawn(async move {
            run_thread(clone).await;
        });
    };
    let archiver_then = state.read().await.archiver_state.clone();

    let archive_ui_tmpl = ArchiveUiTemplate {
        archive_t: archiver_then,
    };
    Ok(archive_ui_tmpl.into_response())
}

pub async fn handler_delete_archive_file(
    State(state): State<AppStateType>,
) -> Result<impl IntoResponse, AppError> {
    println!("->> {} - HANDLER: handler_delete_archive_file", get_time());
    let mut write = state.write().await;
    write.archiver_state.archive_status = "Waiting".to_owned();
    drop(write);
    let archiver = state.read().await.archiver_state.clone();
    let archive_ui_tmpl = ArchiveUiTemplate {
        archive_t: archiver,
    };
    Ok(archive_ui_tmpl.into_response())
}

// endregion: ARCHIVE

// region: LOGIN

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
    //submit: String,
}

pub fn login_router() -> Router<AppStateType> {
    Router::new()
        .route("/login", post(handler_post_login))
        .route("/login", get(handler_get_login))
        .route("/logout", get(handler_get_logout))
        .route("/signup", post(handler_post_signup))
}

pub async fn handler_post_login(
    mut auth_session: AuthSession,
    messages: Messages,
    Form(creds): Form<CredentialsParam>,
) -> impl IntoResponse {
    println!("->> {} - HANDLER: handler_login", get_time());
    //let submit = creds.clone().submit;
    //let submit = submit.as_str();
    //match submit {
    //  "Log in" => {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid credentials!");

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
/* _ => {
            todo!()
        }
    }
}*/

pub async fn handler_get_login(
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

pub async fn handler_get_logout(mut auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => Redirect::to("/login").into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn handler_post_signup(messages: Messages) -> impl IntoResponse {
    messages.success("Failed Successfully!");
    Redirect::to("/login").into_response()
}

// endregion: LOGIN
