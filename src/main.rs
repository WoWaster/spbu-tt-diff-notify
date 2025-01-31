mod db;
mod helpers;
mod models;

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    path::PathBuf,
};

use clap::Parser;
use db::{
    add_new_educator_to_db, get_educators_from_db, get_educators_ids_from_db, init_connection,
    remove_educator_from_db, update_educator_events_in_db,
};
use figment::{
    providers::{Env, Format, Json},
    Figment,
};
use helpers::{find_diffs_in_events, get_educator_events_by_id, log_all_users};
use lettre::{
    transport::smtp::authentication::{Credentials, Mechanism},
    Message, SmtpTransport, Transport,
};
use log::info;
use models::User;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "FILE", default_value = "users.json")]
    users_json_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "schedule.sqlite3")]
    schedule_sqlite3_path: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "config.json")]
    config_json_path: PathBuf,
}

#[derive(Deserialize)]
struct Config {
    email_relay: String,
    email_sender_username: String,
    email_sender_fullname: String,
    email_sender_password: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    info!(
        "Reading users.json from {}",
        std::path::absolute(&args.users_json_path)?.display()
    );
    let users_file = fs::File::open(args.users_json_path)?;
    let users: Vec<User> = serde_json::from_reader(users_file)?;
    log_all_users(&users);

    let conn = init_connection(args.schedule_sqlite3_path)?;

    let educators_in_db = get_educators_ids_from_db(&conn)?;
    info!("Found {} educators in db", educators_in_db.len());

    let watched_educators = users
        .iter()
        .flat_map(|user| &user.watch_educators)
        .cloned()
        .collect::<HashSet<_>>();

    let new_educators = watched_educators
        .difference(&educators_in_db)
        .cloned()
        .collect::<HashSet<_>>();
    info!("Going to add {} new educators to db", new_educators.len());
    let stable_educators = watched_educators
        .intersection(&educators_in_db)
        .cloned()
        .collect::<HashSet<_>>();
    info!("Going to diff {} educators", stable_educators.len());
    let stale_educators = educators_in_db
        .difference(&watched_educators)
        .cloned()
        .collect::<HashSet<_>>();
    info!(
        "Going to remove {} educators from db",
        stale_educators.len()
    );

    let http_client = reqwest::blocking::Client::new();

    let mut new_educator_events = HashMap::new();
    for id in watched_educators.into_iter() {
        // This is kinda ugly hack!
        // We need only a small portion of original JSON and we need a formatted one,
        // so this is just a quick de-ser round.
        // FIXME: But this breaks final email, because educator can only be
        // referenced by id, despite having fullname field.
        let json = get_educator_events_by_id(&http_client, id)?;
        let educator_events_str = serde_json::to_string_pretty(&json)?;
        new_educator_events.insert(id, educator_events_str);
    }
    let new_educator_events = new_educator_events;
    info!("Collected {} educator events", new_educator_events.len());

    for new_educator in new_educators.into_iter() {
        add_new_educator_to_db(
            &conn,
            new_educator,
            new_educator_events.get(&new_educator).unwrap(), // unwrap here must be safe!
        )?;
    }

    for stale_educator in stale_educators.into_iter() {
        remove_educator_from_db(&conn, stale_educator)?;
    }

    let old_educator_events = get_educators_from_db(&conn)?;

    let changed_educators = find_diffs_in_events(&new_educator_events, &old_educator_events);
    info!(
        "Found {} change(s) in educators schedules",
        changed_educators.len()
    );

    let mut pretty_diffs: HashMap<u32, String> = HashMap::new();
    for (changed_educator_id, (changed_educator_str, changed_educator_diff)) in
        changed_educators.into_iter()
    {
        update_educator_events_in_db(&conn, changed_educator_id, changed_educator_str)?;
        pretty_diffs.insert(changed_educator_id, changed_educator_diff);
    }

    let config: Config = Figment::new()
        .merge(Json::file(&args.config_json_path))
        .merge(Env::prefixed("TT_"))
        .extract()?;
    info!(
        "Read config.json from {}",
        std::path::absolute(&args.config_json_path)?.display()
    );

    let sender = SmtpTransport::relay(&config.email_relay)?
        .credentials(Credentials::new(
            config.email_sender_username.to_owned(),
            config.email_sender_password,
        ))
        .authentication(vec![Mechanism::Plain])
        .build();

    for user in users.iter() {
        for educator in user.watch_educators.iter() {
            let Some(diff) = pretty_diffs.get(educator) else {
                continue;
            };
            let email = Message::builder()
                .from(
                    format!(
                        "{} <{}>",
                        config.email_sender_fullname, config.email_sender_username
                    )
                    .parse()?,
                )
                .to(format!("{} <{}>", user.name, user.email).parse()?)
                .subject(format!("Изменилось расписание преподавателя {}!", educator)) // FIXME: Use name instead of id
                .body(format!(
                    "Уважаемый (ая), {}!\nВ расписании преподавателя {} произошли изменения:\n{}",
                    user.name, educator, diff
                ))?;

            let result = sender.send(&email);
            match result {
                Ok(code) => info!("Sent email to {} with response {:?}", user.name, code),
                Err(err) => return Err(Box::new(err)),
            }
        }
    }

    Ok(())
}
