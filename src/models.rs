use axum::{http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Local};
use rand::Rng;
use serde::Deserialize;
use sqlx::FromRow;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// region: APP

#[derive(Clone)]
pub struct AppState {
    pub contacts_state: Pool<Sqlite>,
    pub error_state: CreationErrorState,
    pub archiver_state: ArchiverState,
}
pub type AppStateType = Arc<RwLock<AppState>>;

pub fn get_time() -> String {
    let time_stamp_now = std::time::SystemTime::now();
    let datetime = DateTime::<Local>::from(time_stamp_now);
    let timestamp_str = datetime.format("%Y-%m-%d").to_string(); //%H:%M:%S
    timestamp_str
}

// endregion: APP

// region: ERRORS

#[derive(thiserror::Error, Debug)]
pub enum MyError {
    //#[error("error accessing file")]
    //FileAccess(#[from] tokio::io::Error),
    //#[error("error parsing json")]
    //JsonParse(#[from] serde_json::Error),
    #[error("test custom error")]
    CustomError,
    // bail!(MyError::CustomError);
}

#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong {}", self.0),
        )
            .into_response()
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

// endregion: ERRORS

// region: CONTACTS

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
pub struct Contacts {
    pub contacts: Vec<Contact>,
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

impl Contacts {
    pub async fn match_contacts(
        pool: Pool<Sqlite>,
        search_bar: &str,
        mut page_set: u32,
    ) -> anyhow::Result<(Contacts, u32, u32, u32)> {
        let num_of_rows = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM contacts_table
            "#
        )
        .fetch_one(&pool)
        .await?;
        let num_of_rows = num_of_rows.count as u32;
        let page_size: u32 = 8;
        let mut max_page = num_of_rows.div_ceil(page_size);
        if max_page == 0 {
            max_page = 1;
        }
        if page_set == 0 {
            page_set = 1;
        } else if page_set > max_page {
            page_set = max_page;
        };
        let offset = (page_set - 1) * page_size;
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
                Ok((
                    Contacts {
                        contacts: contacts_set,
                    },
                    length,
                    page_set,
                    max_page,
                ))
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

                Ok((
                    Contacts {
                        contacts: contacts_set,
                    },
                    length,
                    page_set,
                    max_page,
                ))
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
                Ok((
                    Contacts {
                        contacts: contacts_set,
                    },
                    length,
                    page_set,
                    max_page,
                ))
            }
        }
    }
}
impl Contact {
    pub async fn check_errors(
        &self,
        pool: &Pool<Sqlite>,
    ) -> anyhow::Result<Option<CreationErrorState>> {
        let new_error = CreationErrorState {
            first_error: if self.first_name.is_empty() {
                "First Name Required".to_string()
            } else {
                "".to_string()
            },
            last_error: if self.last_name.is_empty() {
                "Last Name Required".to_string()
            } else {
                "".to_string()
            },
            phone_error: if self.phone.is_empty() {
                "Phone Required".to_string()
            } else {
                "".to_string()
            },
            email_error: if self.email.is_empty() {
                "Email Required".to_string()
            } else {
                "".to_string()
            },
            email_unique_error: Self::validate_email(pool, self.email.as_str(), self.id).await?,
            birth_error: if self.birth_date.is_empty() {
                "Birth Date Required".to_string()
            } else {
                "".to_string()
            },
        };
        if new_error.first_error.is_empty()
            && new_error.last_error.is_empty()
            && new_error.phone_error.is_empty()
            && new_error.email_error.is_empty()
            && new_error.email_unique_error.is_empty()
            && new_error.birth_error.is_empty()
        {
            Ok(None)
        } else {
            Ok(Some(new_error))
        }
    }
    pub async fn create_contact(&self, pool: Pool<Sqlite>) -> anyhow::Result<u32> {
        let timestamp_str = get_time();
        let mut conn = pool.acquire().await?;
        let id_inserted = sqlx::query!(
            r#"
            INSERT INTO contacts_table ( first_name, last_name, phone, email, birth_date, time_creation)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            self.first_name,
            self.last_name,
            self.phone,
            self.email,
            self.birth_date,
            timestamp_str
        )
        .execute(&mut *conn)
        .await?
        .last_insert_rowid();
        Ok(id_inserted as u32)
    }
    pub async fn edit_contact(pool: Pool<Sqlite>, contact: Contact) -> anyhow::Result<u32> {
        let contact_set = sqlx::query_as!(
            Contact,
            r#"
            SELECT *
            FROM contacts_table
            WHERE id = ?
            "#,
            contact.id
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
            contact.first_name,
            contact.last_name,
            contact.phone,
            contact.email,
            contact.birth_date,
            contact_set.time_creation,
            contact.id,
        )
        .execute(&pool)
        .await?
        .rows_affected();
        Ok(rows_affected as u32)
    }
    pub async fn validate_email(
        pool: &Pool<Sqlite>,
        email_set: &str,
        id_set_opt: i64,
    ) -> anyhow::Result<String> {
        let email_equal;
        match id_set_opt {
            0 => {
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
            x => {
                let rec = sqlx::query!(
                    r#"
                        SELECT COUNT(*) as count FROM contacts_table
                        WHERE email = ?1 AND NOT id = ?2 
                        "#,
                    email_set,
                    x
                )
                .fetch_one(pool)
                .await?;
                email_equal = rec.count;
            }
        }
        match email_equal {
            0 => Ok("".to_string()),
            _ => Ok("Email must be unique".to_string()),
        }
    }
}

// endregion: CONTACTS

// region: ARCHIVER

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
        //"/db/contacts.db"
        "D:/RustProjects/axum-3-htmx/db/contacts.db"
    }
}
pub async fn run_thread(state: AppStateType) {
    for i in 0..10 {
        let random = rand::thread_rng().gen::<f64>();
        let sleep_time = (1000.0 * random) as u64;
        sleep(Duration::from_millis(sleep_time)).await;
        let mut write = state.write().await;
        write.archiver_state.archive_progress = ((i as f64) + 1.0) / 10.0;
        drop(write);
        //if state.read().await.archiver_state.archive_status != "Running" {
        //    return;
        //}
    }
    state.write().await.archiver_state.archive_status = "Complete".to_owned();
}

// endregion: ARCHIVER

// region: USERS

use axum::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use password_auth::verify_password;
use sqlx::SqlitePool;
use tokio::task;

use crate::routers::CredentialsParam;

#[derive(Clone, Deserialize, FromRow)]
pub struct User {
    id: i64,
    pub username: String,
    passwordd: String,
}
// Here we've implemented `Debug` manually to avoid accidentally logging the
// password hash.
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}
impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.passwordd.as_bytes() // We use the password hash as the auth
                                  // hash--what this means
                                  // is when the user changes their password the
                                  // auth session becomes invalid.
    }
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: SqlitePool,
}

impl Backend {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = CredentialsParam;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT * FROM users_table 
            WHERE username = ?
            "#,
            creds.username
        )
        //.bind(creds.username)
        .fetch_optional(&self.db)
        .await?;

        // Verifying the password is blocking and potentially slow, so we'll do so via
        // `spawn_blocking`.
        task::spawn_blocking(|| {
            // We're using password-based authentication--this works by comparing our form
            // input with an argon2 password hash.
            Ok(user.filter(|user| verify_password(creds.password, &user.passwordd).is_ok()))
        })
        .await?
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as("select * from users where id = ?")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;

// endregion: USERS
