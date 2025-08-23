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
                        /*if let Some(old_event) = old_day.day_study_events.iter().find(|old_ev| {
                            old_ev.time_interval_string == new_event.time_interval_string
                        }) {
                            if !(event_eq(new_event, old_event)) {
                                /*println!("OLD SUBJ {}", old_event.subject);
                                println!("NEW SUBJ {}", new_event.subject);
                                print_contingent_unit_names(&old_event.contingent_unit_names);
                                print_contingent_unit_names(&new_event.contingent_unit_names);*/
                                cur_day_diff.push(format_event_as_string(new_event));
                            }
                        } else {
                            cur_day_diff.push(format_event_as_string(new_event));
                        }*/
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
/*mod tests {
    use std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
    };

    use crate::tt_diff::{
        helpers::{find_diffs_in_events, get_previous_events, get_users},
        models::{
            educator_model::{
                ContingentUnitName, DayStudyEvent, EducatorDay, EducatorEvents, EventLocation,
            },
            Args,
        },
    };

    use super::*;

    fn make_event(
        time: &str,
        subject: &str,
        dates: Vec<&str>,
        locations: Vec<&str>,
        groups: Vec<&str>,
    ) -> DayStudyEvent {
        DayStudyEvent {
            time_interval_string: time.to_string(),
            subject: subject.to_string(),
            dates: dates.into_iter().map(|d| d.to_string()).collect(),
            event_locations: locations
                .into_iter()
                .map(|loc| EventLocation {
                    display_name: loc.to_string(),
                })
                .collect(),
            contingent_unit_names: groups
                .into_iter()
                .map(|g| ContingentUnitName {
                    item1: "Группа".to_string(),
                    item2: g.to_string(),
                })
                .collect(),
        }
    }

    // TODO: what if EducatorEventsDays is empty? seems like timetable API does not allow it, but prob would be cleaner to add this case

    /*#[test]
    fn generate_diff_messages_delete_last_event_of_the_last_day() {
        let args_old = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };
        let args_new = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.delete_last_event_of_the_last_day.json"),
        };

        let old = get_previous_events(&args_old).unwrap();
        let new = get_previous_events(&args_new).unwrap();
        let diff = generate_diff_messages(&old, &new);
        assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br>");
        assert_eq!(diff.get(&1879), None);
    }*/

    /*#[test]
    fn generate_diff_messages_from_less_to_many() {
        let args_less = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };
        let args_many = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.many_events.json"),
        };

        let less = get_previous_events(&args_less).unwrap();
        let many = get_previous_events(&args_many).unwrap();
        let diff = generate_diff_messages(&less, &many);
        assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963, 08.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 10:15-11:45<br>    <b>Даты:</b> 01.09.1968, 08.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 102B<br><br><b><font size=\"5\">Среда:<font size=\"10\"></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br>");
        assert_eq!(diff.get(&1879).unwrap().1, "<b><font size=\"5\">Вторник:</font></b><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 22.12.1915, 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br>    <b>Предмет:</b> Декларация прав художника<br>    <b>Время:</b> 11:00-12:30<br>    <b>Даты:</b> 15.08.1918, 22.08.1918<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 202A<br>");
    }*/
}*/
