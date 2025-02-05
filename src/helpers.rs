use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
};

use lettre::Message;
use log::{debug, info};
use reqwest::Client;
use similar::TextDiff;

use crate::models::{educator_model::EducatorEvents, Args, Config, User};

pub fn log_all_users(users: &[User]) -> () {
    for user in users.iter() {
        debug!(
            "Serving {}, who is watching for educators {:?} and groups {:?}",
            user.name, user.watch_educators, user.watch_groups
        );
    }
}

pub async fn get_educator_events_by_id(
    http_client: &Client,
    id: i64,
) -> Result<EducatorEvents, reqwest::Error> {
    info!("Getting events for educator {}", id);
    let request_url = format!("https://timetable.spbu.ru/api/v1/educators/{}/events", id);
    let response = http_client.get(request_url).send().await?;
    response.json().await
}

// Note to myself: this is probably the first time I have done some weird magic
// TODO: read about lifetimes
pub fn find_diffs_in_events<'a>(
    new_events: &'a HashMap<i64, String>,
    old_events: &HashMap<i64, String>,
) -> HashMap<i64, (&'a str, String)> {
    let mut out_map: HashMap<i64, (&str, String)> = HashMap::new();

    for (new_event_id, new_event_str) in new_events.iter() {
        let old_event_str = old_events.get(new_event_id).unwrap(); // unwrap here must be safe!
        let diff = TextDiff::from_lines(old_event_str, new_event_str);
        if diff.ratio() != 1.0 {
            let pretty_diff = diff.unified_diff();
            debug!("Changes for {}: {}", new_event_id, pretty_diff);
            out_map.insert(
                *new_event_id,
                (new_event_str.as_str(), pretty_diff.to_string()),
            );
        }
    }

    out_map
}

pub fn get_users(args: &Args) -> Result<Vec<User>, Box<dyn Error>> {
    info!(
        "Reading users.json from {}",
        std::path::absolute(&args.users_json_path)?.display()
    );
    let users_file = fs::File::open(&args.users_json_path)?;
    let users: Vec<User> = serde_json::from_reader(users_file)?;
    log_all_users(&users);
    Ok(users)
}

pub fn generate_sorts_of_educators(
    watched_educators: &HashSet<i64>,
    educators_in_db: &HashSet<i64>,
) -> Result<(HashSet<i64>, HashSet<i64>, HashSet<i64>), Box<dyn Error>> {
    let new_educators = watched_educators
        .difference(educators_in_db)
        .cloned()
        .collect::<HashSet<_>>();
    info!("Going to add {} new educators to db", new_educators.len());
    let stable_educators = watched_educators
        .intersection(educators_in_db)
        .cloned()
        .collect::<HashSet<_>>();
    info!("Going to diff {} educators", stable_educators.len());
    let stale_educators = educators_in_db
        .difference(watched_educators)
        .cloned()
        .collect::<HashSet<_>>();
    info!(
        "Going to remove {} educators from db",
        stale_educators.len()
    );

    Ok((new_educators, stale_educators, stable_educators))
}

pub fn generate_email(
    config: &Config,
    user: &User,
    educator: &i64,
    diff: &str,
) -> Result<Message, Box<dyn Error>> {
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
            "Уважаемый (ая) {}!\nВ расписании преподавателя {} произошли изменения:\n{}",
            user.name, educator, diff
        ))?;

    Ok(email)
}
