use axum::{extract::FromRef, response::IntoResponse};
use chrono::prelude::*;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, sqlite::SqliteRow, Pool, Sqlite};
use std::{
    default,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tokio::time::{sleep, Duration as TokioDuration};

use crate::TypeOr;

#[derive(Clone)]
pub struct AppState {
    pub contacts_state: Pool<Sqlite>,
    pub error_state: CreationErrorState,
    pub archiver_state: ArchiverState,
}
pub type AppStateType = Arc<RwLock<AppState>>;

#[derive(Debug, Default, Clone, Deserialize, FromRow)]
pub struct Contact {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email: String,
    pub birth_date: String,
    pub time_creation: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CreationErrorState {
    pub first_error: String,
    pub last_error: String,
    pub phone_error: String,
    pub email_error: String,
    pub email_unique_error: String,
    pub birth_error: String,
}

pub async fn match_contacts(
    pool: Pool<Sqlite>,
    search_bar: &str,
    mut page_set: u32,
) -> anyhow::Result<(Vec<Contact>, u32, u32, u32)> {
    let num_of_rows = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM contacts_table
        "#
    )
    .fetch_one(&pool)
    .await?;

    let num_of_rows = num_of_rows.count as u32;
    let mut max_page = num_of_rows.div_ceil(10);
    if max_page == 0 {
        max_page = 1;
    }
    if page_set <= 0 {
        page_set = 1;
    } else if page_set > max_page {
        page_set = max_page;
    };
    let page_size: u32 = 10;
    let offset = ((page_set - 1) * page_size);
    let this_month = get_time();
    match search_bar {
        "" => {
            let contacts_set = sqlx::query_as!(
                Contact,
                r#"
                SELECT *
                FROM contacts_table
                ORDER BY birth_date
                LIMIT ?1 OFFSET ?2
                "#,
                page_size,
                offset
            )
            .fetch_all(&pool)
            .await?;
            let length = contacts_set.len() as u32;
            return Ok((contacts_set, length, page_set, max_page));
        }
        "bday" => {
            let contacts_set = sqlx::query_as!(
                Contact,
                r#"
                SELECT *
                FROM contacts_table
                WHERE SUBSTR(birth_date, 6) >= STRFTIME('%m-%d', DATE('now', 'localtime'))
                AND SUBSTR(birth_date, 6) < STRFTIME('%m-%d', DATE('now', 'localtime', '+1 MONTH'))
                ORDER BY STRFTIME('%m-%d', birth_date)
                LIMIT ?1 OFFSET ?2
                "#,
                page_size,
                offset
            )
            .fetch_all(&pool)
            .await?;
            let length = contacts_set.len() as u32;

            return Ok((contacts_set, length, page_set, max_page));
        }
        _ => {
            let contacts_set = sqlx::query_as!(
                Contact,
                r#"
                SELECT * FROM contacts_table
                WHERE (first_name LIKE '%' || ?1 || '%' 
                OR  last_name LIKE '%' || ?1 || '%'            
                OR phone LIKE '%' || ?1 || '%'
                OR email LIKE '%' || ?1 || '%'
                OR time_creation LIKE '%' || ?1 || '%' )
                ORDER BY id
                LIMIT ?2 OFFSET ?3
                "#,
                search_bar,
                page_size,
                offset
            )
            .fetch_all(&pool)
            .await?;
            let length = contacts_set.len() as u32;
            return Ok((contacts_set, length, page_set, max_page));
        }
    }
}
pub async fn check_errors(
    pool: &Pool<Sqlite>,
    first: &String,
    last: &String,
    phone: &String,
    email: &String,
    birth: &String,
    id_set_opt: &Option<u32>,
) -> anyhow::Result<Option<CreationErrorState>> {
    let new_error = CreationErrorState {
        first_error: if first == "" {
            "First Name Required".to_string()
        } else {
            "".to_string()
        },
        last_error: if last == "" {
            "Last Name Required".to_string()
        } else {
            "".to_string()
        },
        phone_error: if phone == "" {
            "Phone Required".to_string()
        } else {
            "".to_string()
        },
        email_error: if email == "" {
            "Email Required".to_string()
        } else {
            "".to_string()
        },
        email_unique_error: validate_email(pool, email, id_set_opt).await?,
        birth_error: if birth == "" {
            "Birth Date Required".to_string()
        } else {
            "".to_string()
        },
    };
    if new_error.first_error == ""
        && new_error.last_error == ""
        && new_error.phone_error == ""
        && new_error.email_error == ""
        && new_error.email_unique_error == ""
        && new_error.birth_error == ""
    {
        return Ok(None);
    }
    return Ok(Some(new_error));
}
pub async fn create_contact(
    pool: Pool<Sqlite>,
    first_set: String,
    last_set: String,
    phone_set: String,
    email_set: String,
    birth_set: String,
) -> anyhow::Result<u32> {
    let timestamp_str = get_time();
    let mut conn = pool.acquire().await?;
    let id_inserted = sqlx::query!(
        r#"
        INSERT INTO contacts_table ( first_name, last_name, phone, email, birth_date, time_creation)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        first_set,
        last_set,
        phone_set,
        email_set,
        birth_set,
        timestamp_str
    )
    .execute(&mut *conn)
    .await?
    .last_insert_rowid();
    return Ok(id_inserted as u32);
}
pub async fn edit_contact(
    pool: Pool<Sqlite>,
    first_set: String,
    last_set: String,
    phone_set: String,
    email_set: String,
    birth_set: String,
    id_set: u32,
) -> anyhow::Result<u32> {
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

    let rows_affected = sqlx::query!(
        r#"
        UPDATE contacts_table
        SET first_name = ?1,
            last_name = ?2,
            phone = ?3,
            email = ?4,
            birth_date = ?5,
            time_creation = ?6
        WHERE id = ?7                
        "#,
        first_set,
        last_set,
        phone_set,
        email_set,
        birth_set,
        contact_set.time_creation,
        id_set
    )
    .execute(&pool)
    .await?
    .rows_affected();
    return Ok(rows_affected as u32);
}
pub async fn validate_email(
    pool: &Pool<Sqlite>,
    email_set: &String,
    id_set_opt: &Option<u32>,
) -> anyhow::Result<String> {
    let mut email_equal = 0;
    match id_set_opt {
        Some(id_set) => {
            let rec = sqlx::query!(
                r#"
                    SELECT COUNT(*) as count FROM contacts_table
                    WHERE email = ?1 AND NOT id = ?2 
                    "#,
                email_set,
                id_set
            )
            .fetch_one(pool)
            .await?;
            email_equal = rec.count;
        }
        None => {
            let result = sqlx::query!(
                r#"
                    SELECT COUNT(*) as count FROM contacts_table
                    WHERE email = ?1
                    "#,
                email_set
            )
            .fetch_one(pool)
            .await?;
            email_equal = result.count;
        }
    }
    match email_equal {
        0 => {
            return Ok("".to_string());
        }
        _ => {
            return Ok("Email must be unique".to_string());
        }
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
    pub fn archive_file(&self) -> &str {
        //return "/db/contacts.db";
        return "D:/RustProjects/axum-3-htmx/db/contacts.db";
    }
}

pub async fn run_thread(state: AppStateType) -> () {
    for i in (0..10) {
        let random = rand::thread_rng().gen::<f64>();
        let sleep_time = (1000.0 * random) as u64;
        sleep(TokioDuration::from_millis(sleep_time)).await;
        let mut write = state.write().unwrap();
        write.archiver_state.archive_progress = ((i as f64) + 1.0) / 10.0;
        drop(write);
        if state.read().unwrap().archiver_state.archive_status != "Running" {
            return;
        }
    }
    state.write().unwrap().archiver_state.archive_status = "Complete".to_owned();
    return;
}
pub fn get_time() -> String {
    let time_stamp_now = std::time::SystemTime::now();
    let datetime = DateTime::<Local>::from(time_stamp_now);
    let timestamp_str = datetime.format("%Y-%m-%d").to_string(); //%H:%M:%S
    timestamp_str
}
