use std::{collections::HashMap, error::Error, fs::File, io::BufReader};

use lettre::{message::header::ContentType, Message};
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

pub fn get_users(args: &Args) -> Result<Vec<User>, Box<dyn Error>> {
    info!(
        "Reading users.json from {}",
        std::path::absolute(&args.users_json_path)?.display()
    );
    let users_file = BufReader::new(File::open(&args.users_json_path)?);
    let users: Vec<User> = serde_json::from_reader(users_file)?;
    log_all_users(&users);
    Ok(users)
}

pub fn get_previous_events(args: &Args) -> Result<HashMap<u32, EducatorEvents>, Box<dyn Error>> {
    info!(
        "Reading previous events from {}",
        std::path::absolute(&args.previous_events_json_path)?.display()
    );
    if args.previous_events_json_path.exists() {
        let events_file = BufReader::new(File::open(&args.previous_events_json_path)?);
        let events: Vec<EducatorEvents> = serde_json::from_reader(events_file)?;
        let events_hm = events
            .into_iter()
            .map(|educator| (educator.educator_master_id.to_owned(), educator))
            .collect::<HashMap<_, _>>();
        Ok(events_hm)
    } else {
        Ok(HashMap::new())
    }
}

pub async fn get_educator_events_by_id(
    http_client: &Client,
    id: u32,
) -> Result<(u32, EducatorEvents), reqwest::Error> {
    info!("Getting events for educator {}", id);
    let request_url = format!("https://timetable.spbu.ru/api/v1/educators/{}/events", id);
    let response = http_client.get(request_url).send().await?;
    let educator: EducatorEvents = response.json().await?;
    Ok((educator.educator_master_id.to_owned(), educator))
}

pub fn find_diffs_in_events<'a>(
    educators_old: &HashMap<u32, EducatorEvents>,
    educators_new: &'a HashMap<u32, EducatorEvents>,
) -> Result<HashMap<u32, (&'a EducatorEvents, String)>, Box<dyn Error>> {
    let mut educators_changed: HashMap<u32, (&EducatorEvents, String)> = HashMap::new();

    for (id, new_events) in educators_new.iter() {
        if let Some(old_events) = educators_old.get(id) {
            let old_events_json = serde_json::to_string_pretty(old_events)?;
            let new_events_json = serde_json::to_string_pretty(new_events)?;
            let diff = TextDiff::from_lines(&old_events_json, &new_events_json);
            if diff.ratio() != 1.0 {
                let pretty_diff = diff.unified_diff();
                educators_changed.insert(id.to_owned(), (new_events, pretty_diff.to_string()));
            }
        }
    }

    Ok(educators_changed)
}

pub fn generate_email(
    config: &Config,
    user: &User,
    events: &EducatorEvents,
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
        .subject(format!(
            "Изменилось расписание преподавателя {}!",
            events.educator_long_display_text
        ))
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Уважаемый (ая) {}!\nВ расписании преподавателя {} произошли изменения:\n{}",
            user.name, events.educator_long_display_text, diff
        ))?;

    Ok(email)
}

pub fn write_previous_events(
    args: &Args,
    educator_events_new: HashMap<u32, EducatorEvents>,
) -> Result<(), Box<dyn Error>> {
    info!(
        "Writing {} events to a {}",
        educator_events_new.len(),
        std::path::absolute(&args.previous_events_json_path)?.display()
    );

    let events = educator_events_new
        .into_values()
        .collect::<Vec<EducatorEvents>>();

    let events_file = File::create(&args.previous_events_json_path)?;

    Ok(serde_json::to_writer_pretty(events_file, &events)?)
}
