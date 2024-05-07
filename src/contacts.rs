use serde::Deserialize;
use sqlx::FromRow;
use sqlx::{Pool, Sqlite};
//use std::sync::{Arc, RwLock};

use crate::get_time;

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
