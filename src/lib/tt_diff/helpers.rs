use std::{collections::HashMap, error::Error, fs::File, hash::Hash, io::BufReader};

use lettre::{message::header::ContentType, Message};
use log::{debug, info};
use reqwest::Client;
use similar::TextDiff;

use crate::tt_diff::models::{
    /*educator_model::ContingentUnitName,*/ educator_model::DayStudyEvent,
    educator_model::EducatorEvents, Args, Config, User,
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
                /*println!("NEW DIFF:\n {}", pretty_diff);*/
                educators_changed.insert(id.to_owned(), (new_events, pretty_diff.to_string()));
            }
        }
    }

    Ok(educators_changed)
}

/* form string of information about changed event */
fn format_event_as_string(event: &DayStudyEvent) -> String {
    format!(
        "    <b>Предмет:</b> {}<br>    <b>Время:</b> {}<br>    <b>Даты:</b> {}<br>    <b>Места:</b> {}<br>    <b>Направления:</b> {}<br>",
        event.subject,
        event.time_interval_string,
        event.dates.join(", "),
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

/* compare two Vec's with allowence of mixed order */
pub fn vec_eq_unordered<T>(fst: &[T], snd: &[T]) -> bool
where
    T: Eq + Hash,
{
    if fst.len() != snd.len() {
        return false;
    }

    let mut map_fst = HashMap::new();
    for item in fst {
        /* find int value corresponding to key "item" (or insert 0 if none), increment it */
        *map_fst.entry(item).or_insert(0) += 1;
    }

    let mut map_snd = HashMap::new();
    for item in snd {
        *map_snd.entry(item).or_insert(0) += 1;
    }

    map_fst == map_snd
}

pub fn event_eq(new: &DayStudyEvent, old: &DayStudyEvent) -> bool {
    new.time_interval_string == old.time_interval_string
        && new.subject == old.subject
        && vec_eq_unordered(&new.dates, &old.dates)
        && vec_eq_unordered(&new.event_locations, &old.event_locations)
        && vec_eq_unordered(&new.contingent_unit_names, &old.contingent_unit_names)
}

/* DEBUG, to delete later
fn print_contingent_unit_names(units: &Vec<ContingentUnitName>) {
    for (i, unit) in units.iter().enumerate() {
        println!("{}. Item1: {}, Item2: {}", i + 1, unit.item1, unit.item2);
    }
}*/

/* take old and new events hashmaps, for every educator for every day form  */
pub fn generate_diff_messages<'a>(
    educators_old: &'a HashMap<u32, EducatorEvents>,
    educators_new: &'a HashMap<u32, EducatorEvents>,
) -> HashMap<u32, (&'a EducatorEvents, String)> {
    let mut educators_new_w_messages = HashMap::new();

    for (&educator_id, new_events) in educators_new {
        if let Some(old_events) = educators_old.get(&educator_id) {
            let mut cur_educator_diff = Vec::new();

            for new_day in &new_events.educator_events_days {
                if let Some(old_day) = old_events
                    .educator_events_days
                    .iter()
                    .find(|old_day| old_day.day_string == new_day.day_string)
                {
                    let mut cur_day_diff = Vec::new();

                    for new_event in &new_day.day_study_events {
                        if !old_day
                            .day_study_events
                            .iter()
                            .any(|old_ev| event_eq(new_event, old_ev))
                        {
                            cur_day_diff.push(format_event_as_string(new_event));
                        }
                    }

                    if !cur_day_diff.is_empty() {
                        cur_educator_diff.push(format!(
                            "<b><font size=\"5\">{}:</font></b><br>{}",
                            new_day.day_string,
                            cur_day_diff.join("<br>")
                        ));
                    }
                } else {
                    cur_educator_diff.push(format!(
                        "<b><font size=\"5\">{}:<font size=\"10\"></b><br>{}",
                        new_day.day_string,
                        new_day
                            .day_study_events
                            .iter()
                            .map(format_event_as_string)
                            .collect::<Vec<_>>()
                            .join("<br>")
                    ));
                }
            }

            if !cur_educator_diff.is_empty() {
                educators_new_w_messages.insert(
                    educator_id,
                    (new_events, format!("{}", cur_educator_diff.join("<br>"))),
                );
            }
        } else {
            let mut cur_educator_diff = Vec::new();
            for new_day in &new_events.educator_events_days {
                cur_educator_diff.push(format!(
                    "<b><font size=\"5\">{}:<font size=\"10\"></b><br>{}",
                    new_day.day_string,
                    new_day
                        .day_study_events
                        .iter()
                        .map(format_event_as_string)
                        .collect::<Vec<_>>()
                        .join("<br>")
                ));
            }
            educators_new_w_messages.insert(
                educator_id,
                (new_events, format!("{}", cur_educator_diff.join("<br>"))),
            );
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
