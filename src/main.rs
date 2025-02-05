mod db;
mod helpers;
mod models;

use std::collections::{HashMap, HashSet};

use clap::Parser;
use db::{
    add_new_educator_to_db, get_educators_from_db, get_educators_ids_from_db, init_connection,
    remove_educator_from_db, update_educator_events_in_db,
};
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use futures::future;
use helpers::{
    find_diffs_in_events, generate_email, generate_sorts_of_educators, get_educator_events_by_id,
    get_users,
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
    let conn = init_connection(&args.schedule_sqlite3_path).unwrap();
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

    /* Get info from db */
    let educators_in_db = get_educators_ids_from_db(&conn).unwrap();
    info!("Found {} educators in db", educators_in_db.len());

    /* Get sorts of educators */
    let (new_educators, stale_educators, _stable_educators) =
        generate_sorts_of_educators(&watched_educators, &educators_in_db).unwrap();

    /* Collect new info from timetable */
    let mut new_educator_events = HashMap::new();
    for json in future::join_all(
        watched_educators
            .into_iter()
            .map(|id| get_educator_events_by_id(&http_client, id)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
    {
        // This is kinda ugly hack!
        // We need only a small portion of original JSON and we need a formatted one,
        // so this is just a quick de-ser round.
        // FIXME: But this breaks final email, because educator can only be
        // referenced by id, despite having fullname field.
        let educator_events_str = serde_json::to_string_pretty(&json).unwrap();
        new_educator_events.insert(json.educator_master_id, educator_events_str);
    }
    let new_educator_events = new_educator_events;
    info!("Collected {} educator events", new_educator_events.len());

    /* Add new educators into db */
    for new_educator in new_educators.into_iter() {
        add_new_educator_to_db(
            &conn,
            new_educator,
            new_educator_events.get(&new_educator).unwrap(), // unwrap here must be safe!
        )
        .unwrap();
    }

    /* Remove unwatched educators from db */
    for stale_educator in stale_educators.into_iter() {
        remove_educator_from_db(&conn, stale_educator).unwrap();
    }

    /* Find out what changed */
    let old_educator_events = get_educators_from_db(&conn).unwrap();

    let changed_educators = find_diffs_in_events(&new_educator_events, &old_educator_events);
    info!(
        "Found {} change(s) in educators schedules",
        changed_educators.len()
    );

    let mut pretty_diffs: HashMap<u32, String> = HashMap::new();
    for (changed_educator_id, (changed_educator_str, changed_educator_diff)) in
        changed_educators.into_iter()
    {
        update_educator_events_in_db(&conn, changed_educator_id, changed_educator_str).unwrap();
        pretty_diffs.insert(changed_educator_id, changed_educator_diff);
    }
    let pretty_diffs = pretty_diffs;

    /* Send emails */
    for user in users.iter() {
        for educator in user.watch_educators.iter() {
            let Some(diff) = pretty_diffs.get(educator) else {
                continue;
            };

            let email = generate_email(&config, user, educator, diff).unwrap();
            let code = sender.send(&email).unwrap();
            info!("Sent email to {} with response {:?}", user.name, code);
        }
    }
}
