use std::{collections::HashMap, error::Error, fs::File, io::BufReader};

use itertools::Itertools;
use lettre::{message::header::ContentType, Message};
use log::{debug, info};
use reqwest::Client;

use crate::tt_diff::models::{
    educator_model::{DayStudyEvent, EducatorDay, EducatorEvents},
    Args, Config, User,
};

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

/* form string of information about changed event */
fn format_event_as_string(event: &DayStudyEvent) -> String {
    format!(
        "    <b>Предмет:</b> {}<br>    <b>Время:</b> {}<br>    <b>Даты:</b> {}<br>    <b>Места:</b> {}<br>    <b>Направления:</b> {}<br>",
        event.subject,
        event.time_interval_string,
        event.dates.iter().join(", "),
        event
            .event_locations
            .iter()
            .map(|loc| loc.display_name.clone())
            .collect::<Vec<_>>()
            .join(", "),
        event
            .contingent_unit_names
            .iter()
            .map(|c| format!("{} {}", c.item1, c.item2))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn add_day_to_diff(cur_educator_diff: &mut Vec<String>, educator_day: &EducatorDay) {
    if educator_day.day_study_events_count != 0 {
        cur_educator_diff.push(format!("<em style=\"color:green;\">Новый день:</em>"));
        cur_educator_diff.push(format!(
            "<b><font size=\"5\">{}:</font></b><br>{}",
            educator_day.day_string,
            educator_day
                .day_study_events
                .iter()
                .map(format_event_as_string)
                .collect::<Vec<_>>()
                .join("<br>")
        ));
    }
}

fn diff_educator_day(
    old_day: &EducatorDay,
    new_day: &EducatorDay,
) -> (Vec<String>, Vec<String>) {
    let old_events = &old_day.day_study_events;
    let new_events = &new_day.day_study_events;

    let added_events = new_events.difference(old_events);
    let removed_events = old_events.difference(new_events);

    let mut removed_acc = Vec::new();
    for event in removed_events {
        removed_acc.push(format_event_as_string(event));
    }
    if removed_acc.len() > 0 {
        removed_acc.insert(0, "<em style=\"color:red;\">Удалённые события:</em>".to_string())
    }
    let mut added_acc = Vec::new();
    for event in added_events {
        added_acc.push(format_event_as_string(event));
    };
    if added_acc.len() > 0 {
        added_acc.insert(0, "<em style=\"color:green;\">Новые события:</em>".to_string())
    }
    return (removed_acc, added_acc)
}

fn add_tracked_educator_to_diff<'a>(
    educator_old_events: &'a EducatorEvents,
    educator_new_events: &'a EducatorEvents,
) -> Vec<String> {
    let mut cur_educator_diff = Vec::new();

    for day in 0..6 {
        let old_day = &educator_old_events.educator_events_days[day];
        let new_day = &educator_new_events.educator_events_days[day];

        match old_day.day_study_events_count {
            0 => {
                if !(new_day.day_study_events_count == 0) {
                    add_day_to_diff(&mut cur_educator_diff, new_day)
                }},
            _ => {
                let (removed, added) = diff_educator_day(old_day, new_day);
                let mut combined = added.clone();
                combined.extend(removed);
                if combined.len() > 0 {
                cur_educator_diff.push(format!(
                    "<b><font size=\"5\">{}:</font></b>",
                    new_day.day_string,
                )); 
                cur_educator_diff.extend(combined);
                }
            }
        }
    }
    cur_educator_diff
}

fn add_untracked_educator_to_diff<'a>(educator_events: &'a EducatorEvents) -> Vec<String> {
    let mut cur_educator_diff = Vec::new();

    for new_day in &educator_events.educator_events_days {
        add_day_to_diff(&mut cur_educator_diff, new_day);
    }

    cur_educator_diff
}

pub fn generate_diff_messages<'a>(
    educators_old: &'a HashMap<u32, EducatorEvents>,
    educators_new: &'a HashMap<u32, EducatorEvents>,
) -> HashMap<u32, (&'a EducatorEvents, String)> {
    let mut educators_new_w_messages = HashMap::new();

    /* for every found educator look for their old events,
    if there are none, insert all their events into the diff */
    for (&educator_id, new_events) in educators_new {
        let educator_diff = match educators_old.get(&educator_id) {
            Some(old_events) => add_tracked_educator_to_diff(old_events, new_events),
            None => add_untracked_educator_to_diff(new_events),
        };
        if educator_diff.len() > 0 {
            educators_new_w_messages.insert(educator_id, (new_events, educator_diff.join("<br>")));
        }
    }

    educators_new_w_messages
}

pub fn collect_all_tracked_diffs(
    educators_changed: &HashMap<u32, (&EducatorEvents, String)>,
    user: &User,
) -> String {
    let mut acc: Vec<String> = Vec::new();
    for educator in user.watch_educators.iter() {
        if let Some((events, diff)) = educators_changed.get(educator) {
            let cur_ed_diff = format!(
                "В расписании преподавателя <b>{}</b> произошли изменения:<br><br>{}<br>",
                events.educator_long_display_text, diff
            );
            acc.push(cur_ed_diff);
        }
    }
    return acc.join("<br> <br>");
}

pub fn generate_email(config: &Config, user: &User, diff: &str) -> Result<Message, Box<dyn Error>> {
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
            "Изменилось расписание преподавателя!"
        ))
        .header(ContentType::TEXT_HTML)
        .body(format!("Уважаемый(ая) {}!<br><br> {} <br> Данное письмо было сгенерировано автоматически, направление ответа не подразумевается.", user.name, diff))?;

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

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;
