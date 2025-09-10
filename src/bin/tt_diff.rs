use lib::tt_diff::helpers;
use lib::tt_diff::mail_sender::LetterSender;
use lib::tt_diff::models;
use lib::tt_diff::schedule_getter::ScheduleGetter;

use clap::Parser;
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use helpers::{generate_diff_messages, get_previous_events, get_users, write_previous_events};
use lettre::{
    transport::smtp::authentication::{Credentials, Mechanism},
    SmtpTransport,
};
use log::info;
use models::{Args, Config};

async fn run<SG: ScheduleGetter, LS: LetterSender>(
    schedule_getter: SG,
    letter_sender: LS,
    args: Args,
    config: Config,
) {
    let users = get_users(&args).unwrap();
    let educator_events_old = get_previous_events(&args).unwrap();
    info!("Found {} educators in db", educator_events_old.len());
    let educator_events_new = schedule_getter.get_schedule(&users).await;
    let educators_changed = generate_diff_messages(&educator_events_old, &educator_events_new);
    info!(
        "Found {} changed educators schedules",
        educators_changed.len()
    );
    letter_sender.form_and_send_letters(users, config, educators_changed);
    write_previous_events(&args, educator_events_new).unwrap();
}

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
    // is async-await mechanic here really necessary?
    run(http_client, sender, args, config).await;
}
