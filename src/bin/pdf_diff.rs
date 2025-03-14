use lettre::Transport;
use lib::pdf_diff::helpers::{
    fetch_and_hash_pdf, generate_email, get_tables, get_users, write_updated_table_hashes,
};
use lib::pdf_diff::models;

use clap::Parser;
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use lettre::{
    transport::smtp::authentication::{Credentials, Mechanism},
    SmtpTransport,
};
use log::info;

use models::{Args, Config, Table};

#[tokio::main]
async fn main() {
    /* Setup logging */
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .init();

    /* Get users and config from corresponding json's */
    let args = Args::parse();
    let users = get_users(&args).unwrap();
    let config: Config = Figment::new()
        .merge(Json::file(&args.config_json_path))
        .merge(Env::prefixed("TT_"))
        .extract()
        .unwrap();
    info!(
        "Read config.json from {}",
        std::path::absolute(&args.config_json_path)
            .unwrap()
            .display()
    );

    /* Get tables and find changed ones */
    let http_client = reqwest::Client::new();
    let tables: Vec<Table> = get_tables(&args).unwrap();
    let mut changed_tables = Vec::<(String, String)>::new();
    for table in &tables {
        let new_hash = fetch_and_hash_pdf(&http_client, &table.link).await.unwrap();
        if new_hash != table.hash {
            changed_tables.push((table.table_name.clone(), new_hash));
        }
    }

    /* Build a setup for sending mails */
    let sender = SmtpTransport::relay(&config.email_relay)
        .unwrap()
        .credentials(Credentials::new(
            config.email_sender_username.to_owned(),
            config.email_sender_password.to_owned(),
        ))
        .authentication(vec![Mechanism::Plain])
        .build();

    /* Find users that are interested in found changes, generate and send emails */
    for user in users {
        let watched_changes: Vec<&(String, String)> = changed_tables
            .iter()
            .filter(|(table_name, _)| user.watch_tables.contains(table_name))
            .collect();

        if watched_changes.is_empty() {
            continue;
        }
        let email = generate_email(&config, &user, &watched_changes).unwrap();
        let code = sender.send(&email).unwrap();
        info!("Sent email to {} with response {:?}", user.name, code);
    }

    /* Set hash changes into json */
    if !changed_tables.is_empty() {
        write_updated_table_hashes(&args, &changed_tables).unwrap();
    }
}
