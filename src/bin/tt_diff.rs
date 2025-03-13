use lib::tt_diff::helpers;
use lib::tt_diff::models;

use std::collections::{HashMap, HashSet};

use clap::Parser;
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use futures::future;
use helpers::{
    find_diffs_in_events, generate_email, get_educator_events_by_id, get_previous_events,
    get_users, write_previous_events,
};
use lettre::{
    transport::smtp::authentication::{Credentials, Mechanism},
    SmtpTransport, Transport,
};
use log::info;
use models::{Args, Config};

#[tokio::main]
async fn main() {
    /* Setup logging */
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .init();

    /* Get all the required resources */
    let args = Args::parse();
    let http_client = reqwest::Client::new();
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
    let sender = SmtpTransport::relay(&config.email_relay)
        .unwrap()
        .credentials(Credentials::new(
            config.email_sender_username.to_owned(),
            config.email_sender_password.to_owned(),
        ))
        .authentication(vec![Mechanism::Plain])
        .build();

    /* Get latest info about users wishes */
    let users = get_users(&args).unwrap();
    let watched_educators = users
        .iter()
        .flat_map(|user| &user.watch_educators)
        .cloned()
        .collect::<HashSet<_>>();

    /* Get previous schedule */
    let educator_events_old = get_previous_events(&args).unwrap();
    info!("Found {} educators in db", educator_events_old.len());

    /* Collect new info from timetable */
    let educator_events_new = future::join_all(
        watched_educators
            .into_iter()
            .map(|id| get_educator_events_by_id(&http_client, id)),
    )
    .await
    .into_iter()
    .collect::<Result<HashMap<_, _>, _>>()
    .unwrap();
    info!("Collected {} educator events", educator_events_new.len());

    /* Collect diffs */
    let educators_changed =
        find_diffs_in_events(&educator_events_old, &educator_events_new).unwrap();
    info!(
        "Found {} changed educators schedules",
        educators_changed.len()
    );

    /* Send emails */
    for user in users.iter() {
        for educator in user.watch_educators.iter() {
            if let Some((events, diff)) = educators_changed.get(educator) {
                let email = generate_email(&config, user, events, diff).unwrap();
                let code = sender.send(&email).unwrap();
                info!("Sent email to {} with response {:?}", user.name, code);
            }
        }
    }

    write_previous_events(&args, educator_events_new).unwrap();
}
