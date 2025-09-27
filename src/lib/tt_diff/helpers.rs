use std::{collections::BTreeMap, error::Error, fs::File, io::BufReader};

use itertools::Itertools;
use lettre::{message::header::ContentType, Message};
use log::{debug, info};
use reqwest::Client;

use super::models::{
    educator_model::{DayStudyEvent, EducatorDay, EducatorEvents},
    Args, Config, User,
};

/// Logs info about all users that are being served in current Geraltt launch.
pub fn log_all_users(users: &[User]) {
    for user in users.iter() {
        debug!(
            "Serving {}, who is watching for educators {:?} and groups {:?}",
            user.name, user.watch_educators, user.watch_groups
        );
    }
}

/// Extracts users from ARGS structure into Vec.
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

/// Extracts the state of schedule, saved during the previous Geraltt launch,
/// from ARGS structure into map with {educator ID: their old schedule} structure.
pub fn get_previous_events(args: &Args) -> Result<BTreeMap<u32, EducatorEvents>, Box<dyn Error>> {
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
            .collect::<BTreeMap<_, _>>();
        Ok(events_hm)
    } else {
        Ok(BTreeMap::new())
    }
}

/// Requests schedule of given educator from TimeTable resourse by their ID.
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

/// Builds string of information about changed event in HTML format.
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

/// If current day isn't empty, formats it into HTML string with "New day:" description.
fn add_day_to_diff(cur_educator_diff: &mut Vec<String>, educator_day: &EducatorDay) {
    if educator_day.day_study_events_count != 0 {
        cur_educator_diff.push("<em style=\"color:green;\">Новый день:</em>".to_string());
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

/// Finds differences between old and new events of the same day and sormats them into strings.
/// The differences are divided into deleted and added events, both with corresponding message.
fn diff_educator_day(old_day: &EducatorDay, new_day: &EducatorDay) -> (Vec<String>, Vec<String>) {
    let old_events = &old_day.day_study_events;
    let new_events = &new_day.day_study_events;

    let added_events = new_events.difference(old_events);
    let removed_events = old_events.difference(new_events);

    let mut removed_acc = Vec::new();
    for event in removed_events {
        removed_acc.push(format_event_as_string(event));
    }
    if !removed_acc.is_empty() {
        removed_acc.insert(
            0,
            "<em style=\"color:red;\">Удалённые события:</em>".to_string(),
        )
    }
    let mut added_acc = Vec::new();
    for event in added_events {
        added_acc.push(format_event_as_string(event));
    }
    if !added_acc.is_empty() {
        added_acc.insert(
            0,
            "<em style=\"color:green;\">Новые события:</em>".to_string(),
        )
    }
    (removed_acc, added_acc)
}

/// Find all changes of a certain educator, that was previously tracked (meaning that their old events need to be compared with new ones).
/// Returns Vec<String>, that contains all
/// information about change days (both added and deleted events with corresponding messages).
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
                if new_day.day_study_events_count != 0 {
                    add_day_to_diff(&mut cur_educator_diff, new_day)
                }
            }
            _ => {
                let (removed, added) = diff_educator_day(old_day, new_day);
                let mut combined = added.clone();
                combined.extend(removed);
                if !combined.is_empty() {
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

/// If an educator was untracked before, meaning that there are no previous events for them provided,
/// Geraltt adds all of their current events to difference.
fn add_untracked_educator_to_diff(educator_events: &EducatorEvents) -> Vec<String> {
    let mut cur_educator_diff = Vec::new();

    for new_day in &educator_events.educator_events_days {
        add_day_to_diff(&mut cur_educator_diff, new_day);
    }

    cur_educator_diff
}

/// Builds map with {educator ID: (their changed events, info about their changed events formatted into HTML string)}.
pub fn generate_diff_messages<'a>(
    educators_old: &'a BTreeMap<u32, EducatorEvents>,
    educators_new: &'a BTreeMap<u32, EducatorEvents>,
) -> BTreeMap<u32, (&'a EducatorEvents, String)> {
    let mut educators_new_w_messages = BTreeMap::new();

    for (&educator_id, new_events) in educators_new {
        let educator_diff = match educators_old.get(&educator_id) {
            Some(old_events) => add_tracked_educator_to_diff(old_events, new_events),
            None => add_untracked_educator_to_diff(new_events),
        };
        if !educator_diff.is_empty() {
            educators_new_w_messages.insert(educator_id, (new_events, educator_diff.join("<br>")));
        }
    }

    educators_new_w_messages
}

/// Finds all educators with changed schedules, that certain user watches,
/// and concats info about their changes into one HTML string.
pub fn collect_all_tracked_diffs(
    educators_changed: &BTreeMap<u32, (&EducatorEvents, String)>,
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
    acc.join("<br> <br>")
}

/// Builds an email from sender email info, user email info and string of difference of watched educators for said user.
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
        .subject("Изменилось расписание преподавателя!".to_string())
        .header(ContentType::TEXT_HTML)
        .body(format!("Уважаемый(ая) {}!<br><br> {} <br> Данное письмо было сгенерировано автоматически, направление ответа не подразумевается.", user.name, diff))?;

    Ok(email)
}

/// Updates previous_events.json, so that during the next Geraltt launch,
/// it will contain information about the most recent state of the schedule.
pub fn write_previous_events(
    args: &Args,
    educator_events_new: BTreeMap<u32, EducatorEvents>,
) -> Result<(), Box<dyn Error>> {
    let is_test = &args
        .previous_events_json_path
        .to_str()
        .unwrap()
        .starts_with("tests/test.");
    if *is_test {
        return Ok(());
    }
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
