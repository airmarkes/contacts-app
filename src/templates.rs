use crate::models::*;
use askama::Template;

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
    pub contacts_t: Vec<Contact>,
    pub search_t: &'a str,
    pub flash_t: FlashState,
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

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewContactTemplate<'a> {
    pub errors_t: CreationErrorState,
    pub first_t: &'a str,
    pub last_t: &'a str,
    pub phone_t: &'a str,
    pub email_t: &'a str,
    pub birth_t: &'a str,
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
