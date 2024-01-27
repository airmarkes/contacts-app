use std::sync::{Arc, RwLock};
use axum::{extract::FromRef, response::IntoResponse};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;

use crate::TypeOr;

#[derive(Default, Clone, Deserialize, Debug)]
pub struct AppState {
    pub contacts_state: ContactState, 
    pub error_state: CreationErrorState,
    pub flash_state: FlashState,        
}
pub type ContactState = Vec<Contact>;
pub type AppStateType = Arc<RwLock<AppState>>;

/*impl FromRef<AppState> for ContactState {
    fn from_ref(input: &AppState) -> ContactState {
        input.contacts_state.clone()
    }
}
impl FromRef<AppState> for CreationErrorState {
    fn from_ref(input: &AppState) -> CreationErrorState {
        input.error_state.clone()        
    }
}
impl FromRef<AppState> for TryState {
    fn from_ref(input: &AppState) -> TryState {
        input.try_state.clone()        
    }
}*/

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Contact {   
    pub id: usize,
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
    pub time_creation: String,       
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CreationErrorState {
    pub first_error: String,
    pub last_error: String,
    pub phone_error: String,
    pub email_error: String,
    pub email_unique_error: String,
}
#[derive(Debug, Default, Clone, Deserialize)]
pub struct FlashState {
    pub flash: String,
    pub flash_count: u8
}

pub fn match_contacts(search_bar: &str, contacts_all: Vec<Contact>, page_set: usize) -> (Vec<Contact>, usize) {
    let page_size: usize = 10;
    let start = (page_set - 1) * page_size;
    let end = start + page_size;
    match search_bar {
        "" => { let contacts_set = contacts_all.into_iter().enumerate()
            .filter(|&(i,_)| i >= start && i < end).map(|(_, e)| e).collect::<Vec<Contact>>();
            let len = contacts_set.len();
            return (contacts_set,  len)
        },
        _  => { let contacts_set = contacts_all.into_iter().filter(|s| 
            s.first == search_bar ||
            s.last == search_bar ||
            s.phone == search_bar ||
            s.email == search_bar).collect::<Vec<Contact>>();
            let contacts_set = contacts_set.into_iter().enumerate()
            .filter(|&(i,_)| i >= start && i <= end).map(|(_, e)| e).collect::<Vec<Contact>>();
            let len = contacts_set.len();
            return (contacts_set, len)
        }
    }
}
pub fn check_errors(new_contact: &Contact, contacts_all: &Vec<Contact>) -> Option<CreationErrorState> {
    let new_error = CreationErrorState {
        first_error: if new_contact.first == "" {"First Name Required".to_string()} else {"".to_string()},
        last_error: if new_contact.last == "" {"Last Name Required".to_string()} else {"".to_string()},
        phone_error: if new_contact.phone == "" {"Phone Required".to_string()} else {"".to_string()},
        email_error: if new_contact.email == "" {"Email Required".to_string()} else {"".to_string()},
        email_unique_error: validate_email(&contacts_all, &new_contact.email),
    };
    if new_error.first_error == "" &&
        new_error.last_error == "" &&
        new_error.phone_error == "" &&
        new_error.email_error == "" &&
        new_error.email_unique_error == "" {
            return None; 
    } else {
    return Some(new_error);
    }
}
pub fn create_contact(first_set: String, last_set: String, phone_set: String, email_set: String, contacts_all: &Vec<Contact>) -> Contact {
    let max_id: usize;
    let opt_id = contacts_all.iter().max_by(|x, y| x.id.cmp(&y.id));
    match opt_id {
        None => max_id = 0,
        Some(i) => max_id = i.id,           
    }
    let id_set = max_id + 1;               
    let time_stamp_now = std::time::SystemTime::now();
    let datetime = DateTime::<Local>::from(time_stamp_now);
    let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string(); 
    let new_contact = Contact { 
        id: id_set,                   
        first: first_set,
        last: last_set,
        phone: phone_set,
        email: email_set,
        time_creation: timestamp_str,            
    };
    return new_contact;
}
pub fn edit_contact(id_set: usize, first_set: String, last_set: String, phone_set: String, email_set: String, time_creation_set: String)
-> Contact {
    let edited_contact = Contact {
        id: id_set,
        first: first_set,
        last: last_set,
        phone: phone_set,
        email: email_set,
        time_creation: time_creation_set,
    };
    edited_contact

}
pub fn validate_email(contacts_all: &Vec<Contact>, email_set: &String)
-> String {   
    let email_equal = contacts_all.into_iter().any(|x| x.email.as_str() == email_set); 
        match email_equal {
            true => { return "Email must be unique".to_string(); },
            false => { return "".to_string(); }
        };
}
