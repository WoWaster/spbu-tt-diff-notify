use std::{collections::BTreeSet, path::PathBuf};

use clap::{command, Parser};
use serde::{Deserialize, Serialize};

pub mod educator_model;

/// A model for describing users of the tool.
/// Consists of:
/// 1. User's name. Should be full, because it will be written in the beginning of the letter
/// 2. IDs of educators that user watches
/// 3. IDs of groups that user watches
/// 4. User's email address to which they will receive letters
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub watch_educators: BTreeSet<u32>,
    pub watch_groups: BTreeSet<u32>,
    pub email: String,
}

/// A model for describing ARGS of the tool.
/// Consists of:
/// 1. Path to user.json, that provides the info about users who will receive notifications and the list of watched educators for each one of them.
/// 2. Path to config.json, that contains email sender configuration parameters.
/// 3. Path to previous_events.json, that contains the information about schedule state at the time of the last Geraltt's launch.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_name = "FILE", default_value = "users.json")]
    pub users_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "config.json")]
    pub config_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "previous_events.json")]
    pub previous_events_json_path: PathBuf,
}

/// A model for describing configuration of the tool.
/// Consists of:
/// 1. SMTP server address
/// 2. Email address from which the letters will be sent
/// 3. Email sender display name, that will be shown in the letter
/// 4. Password for email account from which the letters will be sent
#[derive(Deserialize)]
pub struct Config {
    pub email_relay: String,
    pub email_sender_username: String,
    pub email_sender_fullname: String,
    pub email_sender_password: String,
}
