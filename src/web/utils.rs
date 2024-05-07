use axum::extract::Query;
use axum::routing::get;
use axum::{extract::State, Router};
use serde::Deserialize;
use std::{thread, time::Duration};

use crate::contacts::*;
use crate::errors::*;
use crate::{get_time, AppStateType};

#[derive(Deserialize)]
pub struct ValidateEmailParams {
    pub email_p: String,
    pub id_p: Option<i64>,
}

pub fn utils_router() -> Router<AppStateType> {
    Router::new()
        .route(
            "/contacts/validate_email",
            get(self::get::handler_get_validate_email),
        )
        .route("/contacts/count", get(self::get::handler_contacts_count))
}

mod get {
    use super::*;

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

    pub async fn handler_contacts_count(
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
        let span = format!("({} total)", contacts_count);
        thread::sleep(Duration::from_millis(900));
        Ok(span)
    }
}
