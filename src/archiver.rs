use rand::Rng;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

use crate::AppStateType;

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
