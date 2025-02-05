use std::path::PathBuf;

use clap::{command, Parser};
use serde::{Deserialize, Serialize};

pub mod educator_model;

/// Model for `users.json`
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub watch_educators: Vec<i64>,
    pub watch_groups: Vec<i64>,
    pub email: String,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_name = "FILE", default_value = "users.json")]
    pub users_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "schedule.sqlite3")]
    pub schedule_sqlite3_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "config.json")]
    pub config_json_path: PathBuf,
}

#[derive(Deserialize)]
pub struct Config {
    pub email_relay: String,
    pub email_sender_username: String,
    pub email_sender_fullname: String,
    pub email_sender_password: String,
}
