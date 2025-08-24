use clap::{command, Parser};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_name = "FILE", default_value = "users.json")]
    pub users_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "config.json")]
    pub config_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "previous_pdf_states.json")]
    pub previous_pdf_states_json_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub watch_tables: Vec<String>,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub email_relay: String,
    pub email_sender_username: String,
    pub email_sender_fullname: String,
    pub email_sender_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Table {
    pub table_name: String,
    pub link: String,
    pub hash: String,
}
