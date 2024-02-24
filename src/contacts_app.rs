use std::{sync::{Arc, RwLock}, default, thread, time::Duration};
use axum::{extract::FromRef, response::IntoResponse};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use rand::prelude::*;
use sqlx::{prelude::FromRow, sqlite::SqliteRow, Pool, Sqlite};
use tokio::time::{sleep, Duration as TokioDuration};
//use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::TypeOr;

#[derive(Clone)]
pub struct AppState {
    pub contacts_state: Pool<Sqlite>, 
    pub error_state: CreationErrorState,
    pub flash_state: FlashState,  
    pub archiver_state: ArchiverState
    //pub archiver_state: ArchiverStateType,  
}
//pub type ContactState = Vec<Contact>;
pub type AppStateType = Arc<RwLock<AppState>>;
//pub type ArchiverStateType = Arc<RwLock<ArchiverState>>;

/*impl FromRef<AppState> for Arc<RwLock<ArchiverState>> {
    fn from_ref(input: &AppState) -> Arc<RwLock<ArchiverState>> {
        input.archiver_state.clone()
    }
}
impl FromRef< Arc<RwLock<AppState>>> for ContactState {
    fn from_ref(input: &AppState) -> ContactState {
        input.contacts_state.clone()
    }
}
impl FromRef< Arc<RwLock<AppState>>> for CreationErrorState {
    fn from_ref(input: &AppState) -> CreationErrorState {
        input.error_state.clone()        
    }
}
impl FromRef< Arc<RwLock<AppState>>> for FlashState {
    fn from_ref(input: &AppState) -> FlashState {
        input.flash_state.clone()        
    }
}*/

#[derive(Debug, Default, Clone, Deserialize, FromRow)]
pub struct Contact {   
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email: String,
    pub time_creation: String,       
}
#[derive(sqlx::Type, Debug, Default, Clone, Deserialize)]
#[sqlx(transparent)]
pub struct MyU64(u64);

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

/* pub fn match_contacts(contacts_all: &Vec<SqliteRow>, search_bar: &str, page_set: usize) -> (Vec<Contact>, usize) {
    let page_size: usize = 10;
    let start = (page_set - 1) * page_size;
    let end = start + page_size;
    match search_bar {
        "" => { 
            let contacts_set = &contacts_all.clone().into_iter().enumerate()
            .filter(|&(i,_)| i >= start && i < end).map(|(_, e)| e).collect::<Vec<Contact>>();
            let len = contacts_set.len();
            return (contacts_set,  len)
        },
        _  => { 
            let contacts_set = &contacts_all.clone().into_iter().filter(|s| 
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
 */
pub async fn match_contacts(pool: Pool<Sqlite>, search_bar: &str, mut page_set: u32) 
-> anyhow::Result<(Vec<Contact>, u32, u32, u32)> {
    let num_of_rows = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM contacts_table
        "#
    ).fetch_one(&pool).await?;

    let num_of_rows = num_of_rows.count as u32;
    println!("Number of rows: {num_of_rows}");
    //let mut max_page = contacts_all.len().div_ceil(10);
    let mut max_page = num_of_rows.div_ceil(10);
    if max_page == 0 { max_page = 1; }
    if page_set <= 0 { page_set = 1;} 
    else if page_set > max_page { page_set = max_page;};
    let page_size: u32 = 10;
    let offset = ((page_set - 1) * page_size) ;        

    let contacts_set = sqlx::query_as!(Contact,
        r#"
        SELECT *
        FROM contacts_table
        ORDER BY id
        LIMIT ?2 OFFSET ?3
        "#, search_bar, page_size, offset
    ).fetch_all(&pool).await?;  

    //let (contacts_set, length) = contacts_all.match_contacts(&search_bar, page_set);    
    let length = contacts_set.len() as u32;
    return Ok((contacts_set, length, page_set, max_page));

}
pub async fn check_errors(pool: &Pool<Sqlite>, first: &String, last: &String, phone: &String, email: &String, id_set_opt: &Option<u32>)
 -> anyhow::Result<Option<CreationErrorState>> {
    let new_error = CreationErrorState {
        first_error: if first == "" {"First Name Required".to_string()} else {"".to_string()},
        last_error: if last == "" {"Last Name Required".to_string()} else {"".to_string()},
        phone_error: if phone == "" {"Phone Required".to_string()} else {"".to_string()},
        email_error: if email == "" {"Email Required".to_string()} else {"".to_string()},
        email_unique_error: validate_email(pool, email, id_set_opt).await?,
    };
    if new_error.first_error == "" &&
        new_error.last_error == "" &&
        new_error.phone_error == "" &&
        new_error.email_error == "" &&
        new_error.email_unique_error == "" {
            return Ok(None); 
    } else {
    return Ok(Some(new_error));
    }
}
/*pub fn create_contact(first_set: String, last_set: String, phone_set: String, email_set: String) -> Contact {
    let max_id: usize;
    let opt_id = &contacts_all.clone().iter().max_by(|x, y| x.id.cmp(&y.id));
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
}*/
pub async fn create_contact(pool: Pool<Sqlite>, first_set: String, last_set: String, phone_set: String, email_set: String) 
-> anyhow::Result<u32> {
    let time_stamp_now = std::time::SystemTime::now();
    let datetime = DateTime::<Local>::from(time_stamp_now);
    let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string(); 

    let mut conn = pool.acquire().await?;
    let id_inserted = sqlx::query!(
        r#"
        INSERT INTO contacts_table ( first_name, last_name, phone, email, time_creation)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        first_set, last_set, phone_set, email_set, timestamp_str
    ).execute(&mut *conn).await?.last_insert_rowid();                               
    return Ok(id_inserted as u32);

}
/* pub fn edit_contact(contacts_all: &Vec<SqliteRow>, id_set: usize, first_set: String, last_set: String, phone_set: String, email_set: String)
-> (Contact, usize) {
    let contact_position = &contacts_all.iter().position(|x| x.id == id_set).unwrap(); 
    let contact_set = &contacts_all.clone().into_iter().filter(|x| x.id == id_set ).collect::<ContactState>().swap_remove(0);                   
    let time_creation_set = &contact_set.time_creation;
    let edited_contact = Contact {
        id: contact_set.id,
        first: first_set,
        last: last_set,
        phone: phone_set,
        email: email_set,
        time_creation: contact_set.time_creation,
    };
    (edited_contact, contact_position)
} */
pub async fn edit_contact(pool: Pool<Sqlite>, first_set: String, last_set: String, phone_set: String, email_set: String, id_set: u32)
-> anyhow::Result<u32> {
    let contact_set = sqlx::query_as!(Contact,
        r#"
        SELECT *
        FROM contacts_table
        WHERE id = ?
        "#, id_set
    ).fetch_one(&pool).await?;

    let rows_affected = sqlx::query!(
        r#"
        UPDATE contacts_table
        SET first_name = ?1,
            last_name = ?2,
            phone = ?3,
            email = ?4,
            time_creation = ?5
        WHERE id = ?6                
        "#, first_set, last_set, phone_set, email_set, contact_set.time_creation, id_set
    ).execute(&pool).await?.rows_affected();
    return Ok(rows_affected as u32);
}
 pub async fn validate_email(pool: &Pool<Sqlite>, email_set: &String, id_set_opt: &Option<u32>)
-> anyhow::Result<String> {   
    //let email_equal = &contacts_all.iter().any(|x| x.email.as_str() == email_set); 
/*         match email_equal {
            true => { return "Email must be unique".to_string(); },
            false => { return "".to_string(); }
        };
        */
        let mut email_equal = 0;
        //let mut contacts_all_but = Vec::new();
        match id_set_opt {
            Some(id_set) => {
                //contacts_all_but = contacts_all.clone().into_iter().filter(|x| x.id != id_set).collect::<ContactState>();        
                let rec = sqlx::query!(
                    r#"
                    SELECT COUNT(*) as count FROM contacts_table
                    WHERE email = ?1 AND NOT id = ?2 
                    "#, email_set, id_set
                ).fetch_one(pool).await?;
                let email_equal = rec.count; 
            },
            None => {
                //contacts_all_but = contacts_all;
                let result = sqlx::query!(
                    r#"
                    SELECT COUNT(*) as count FROM contacts_table
                    WHERE email = ?1
                    "#, email_set
                ).fetch_one(pool).await?;
                let email_equal = result.count; 
            }
        }
        match email_equal == 1 {
            true => { return Ok("Email must be unique".to_string()); },
            false => { return Ok("".to_string()); }
        };
} 


#[derive(Clone, Deserialize)]
pub struct ArchiverState {
    pub archive_status: String,
    pub archive_progress: f64,
}
impl Default for ArchiverState {
    fn default() -> Self {
        ArchiverState {
        archive_status: "Waiting".to_owned(),
        archive_progress: 0.0,
        }    
    }
}
impl ArchiverState {
    pub fn status(&self) -> String {
        self.archive_status.clone()
    }
    pub fn progress(&self) -> f64 {
        self.archive_progress
    }
    /*pub fn run(self, rw: ArchiverStateType) -> () {
        if self.archive_status == "Waiting".to_owned() {
            let mut mutable_state = .write().unwrap();
            mutable_state.archive_status = "Running".to_owned(); 
            //mutable_state.archive_progress = 0;
            //self.archive_status = "Running".to_owned();
            //self.archive_progress = 0.0;
            tokio::spawn(async move {
                self.run_thread().await;
            });
        }
    } 
     
    pub fn reset(&self) -> () {
        self.archive_status = "Waiting".to_owned();
    } */
    pub fn archive_file(&self) -> &str {
        return "D:/RustProjects/axum-3-htmx/assets/db.json"
    }
}

pub async fn run_thread(state: /*Arc<TokioRwLock<ArchiverState>>*/ AppStateType) -> () {
    //thread::sleep(Duration::from_millis(3000));
    for i in (0..10) {
        let random = rand::thread_rng().gen::<f64>();
        //println!("{random}");
        let sleep_time = (1000.0 * random) as u64;
        sleep(TokioDuration::from_millis(sleep_time)).await;
        //println!("{sleep_time}");
        //thread::sleep(Duration::from_millis((3000.0 * rand::thread_rng().gen::<f64>()) as u64));
        let mut write = state.write().unwrap();
        write.archiver_state.archive_progress = ((i as f64) + 1.0) / 10.0;
        drop(write);
        //println!("Here... {}", state.read().unwrap().archiver_state.archive_progress);
        if state.read().unwrap().archiver_state.archive_status != "Running" { return; } 

    }
    state.write().unwrap().archiver_state.archive_status = "Complete".to_owned();
    return;
}    
