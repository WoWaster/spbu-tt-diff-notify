use serde::{Deserialize, Serialize};

pub mod educator_model;

/// Model for `users.json`
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub watch_educators: Vec<u32>,
    pub watch_groups: Vec<u32>,
    pub email: String,
}
