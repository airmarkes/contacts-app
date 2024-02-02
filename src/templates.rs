use askama::Template;
use crate::contacts_app::*;
use serde::Deserialize;
//PARAMETERS FROM HTTP BODY FOR AXUM
#[derive(Deserialize)]
pub struct ShowParams {
    pub search_p: Option<String>,
    pub page_p: Option<usize>    
}

#[derive(Deserialize)]
pub struct NewContactParams {
    pub first_p: Option<String>, 
    pub last_p: Option<String>,
    pub phone_p: Option<String>,
    pub email_p: Option<String>,     
} 

#[derive(Deserialize)]
pub struct ViewContactParams{
    pub id_p: Option<usize>
}

#[derive(Deserialize)]
pub struct EditContactParams{
    pub id_p: Option<usize>,
    pub first_p: Option<String>, 
    pub last_p: Option<String>,
    pub phone_p: Option<String>,
    pub email_p: Option<String>, 
}

#[derive(Deserialize)]
pub struct ValidateEmailParams{
    pub email_p: Option<String>,
    pub id_p: Option<usize>,
}

#[derive(Deserialize)]
pub struct DeleteBulkParams{
    #[serde(rename = "ids_p")]
    pub ids_p: Option<Vec<String>>,
}

//TEMPLATES FOR ASKAMA

#[derive(Template)]
#[template(path = "root.html")]
pub struct RootTemplate<'a> {
    pub name: &'a str,
}

#[derive(Template)]
#[template(path = "overview.html")]
pub struct OverviewTemplate {}

#[derive(Template)]
#[template(path = "show.html")]
pub struct ShowTemplate<'a> {
    pub contacts_t: ContactState,
    pub search_t: &'a str,
    pub flash_t: FlashState,
    pub length_t: usize,
    pub page_t: usize,
    pub max_page_t: usize,
    pub archive_t: ArchiverState,
}

#[derive(Template)]
#[template(path = "show_rows.html")]
pub struct RowsTemplate {
    pub contacts_t: ContactState,
    pub length_t: usize,
    pub page_t: usize,
    pub max_page_t: usize
}

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewContactTemplate<'a> {    
    pub errors_t: CreationErrorState,
    pub first_t: &'a str,
    pub last_t: &'a str,
    pub phone_t: &'a str,
    pub email_t: &'a str,
}

#[derive(Template)]
#[template(path = "view.html")]
pub struct ViewContactTemplate {
    pub contact_t: Contact,
}

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditContactTemplate {
    pub errors_t: CreationErrorState,
    pub contact_t: Contact,
}

#[derive(Template)]
#[template(path = "archive_ui.html")]
pub struct ArchiveUiTemplate {
    pub archive_t: ArchiverState,
}