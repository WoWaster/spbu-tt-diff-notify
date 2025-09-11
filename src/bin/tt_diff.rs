use lib::tt_diff::helpers;
use lib::tt_diff::models;
use lib::tt_diff::run_tool::run;

use clap::Parser;
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use helpers::write_previous_events;
use lettre::{
    transport::smtp::authentication::{Credentials, Mechanism},
    SmtpTransport,
};
use log::info;
use models::{Args, Config};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .init();
    let args = Args::parse();
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

    let http_client = reqwest::Client::new();
    let sender = SmtpTransport::relay(&config.email_relay)
        .unwrap()
        .credentials(Credentials::new(
            config.email_sender_username.to_owned(),
            config.email_sender_password.to_owned(),
        ))
        .authentication(vec![Mechanism::Plain])
        .build();
    let new_events = run(http_client, sender, &args, config).await;
    write_previous_events(&args, new_events).unwrap();
}
