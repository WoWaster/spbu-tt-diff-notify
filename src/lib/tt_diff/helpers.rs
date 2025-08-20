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
                        if !old_day.day_study_events.iter().any(|old_ev| {
                            event_eq(new_event, old_ev)
                        }) {
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
        .header(ContentType::TEXT_HTML)
        .body(format!(
            "Уважаемый(ая) {}!<br><br>В расписании преподавателя <b>{}</b> произошли изменения:<br><br>{}<br>Данное письмо было сгенерировано автоматически, направление ответа не подразумевается.",
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

#[cfg(test)]
mod tests {
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

    #[test]
    fn get_users_valid_json() {
        let args = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };

        let users = get_users(&args).unwrap();
        let watch_ed_ref = HashSet::from([1928, 1879]);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Энди Уорхол");
        assert_eq!(users[0].watch_educators, watch_ed_ref);
        assert!(users[0].watch_groups.is_empty());
        assert_eq!(users[0].email, "campbellsoupthebest@gmail.com");
    }

    #[test]
    fn get_prev_events_correct_json() {
        let args = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };

        let prev_ev = get_previous_events(&args).unwrap();

        let mut prev_ev_ref: HashMap<u32, EducatorEvents> = HashMap::new();

        let warhol = EducatorEvents {
            educator_long_display_text: "Энди Уорхол".to_string(),
            educator_master_id: 1928,
            educator_events_days: vec![EducatorDay {
                day_string: "Понедельник".to_string(),
                day_study_events_count: 1,
                day_study_events: vec![DayStudyEvent {
                    time_interval_string: "08:30-10:00".to_string(),
                    subject: "Как превратить искусство в массовый продукт".to_string(),
                    dates: vec!["01.09.1963".to_string()],
                    event_locations: vec![EventLocation {
                        display_name: "231 East 47th Street".to_string(),
                    }],
                    contingent_unit_names: vec![ContingentUnitName {
                        item1: "Группа".to_string(),
                        item2: "101A".to_string(),
                    }],
                }],
            }],
        };
        prev_ev_ref.insert(1928, warhol);

        let malevich = EducatorEvents {
            educator_long_display_text: "Казимир Малевич".to_string(),
            educator_master_id: 1879,
            educator_events_days: vec![EducatorDay {
                day_string: "Вторник".to_string(),
                day_study_events_count: 1,
                day_study_events: vec![DayStudyEvent {
                    time_interval_string: "09:00-10:30".to_string(),
                    subject: "От кубизма к супрематизму".to_string(),
                    dates: vec!["29.12.1915".to_string()],
                    event_locations: vec![EventLocation {
                        display_name: "Дворцовая площадь, д. 6/8".to_string(),
                    }],
                    contingent_unit_names: vec![
                        ContingentUnitName {
                            item1: "Группа".to_string(),
                            item2: "201A".to_string(),
                        },
                        ContingentUnitName {
                            item1: "Группа".to_string(),
                            item2: "201B".to_string(),
                        },
                    ],
                }],
            }],
        };
        prev_ev_ref.insert(1879, malevich);
        assert_eq!(prev_ev, prev_ev_ref);
    }

    #[test]
    fn find_diffs_in_events1() {
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
        let educators_old = get_previous_events(&args_less).unwrap();
        let educators_new = get_previous_events(&args_many).unwrap();
        let diff = find_diffs_in_events(&educators_old, &educators_new).unwrap();
        // TODO: unreadable? prob insert it after formatting tests
        assert_eq!(diff.get(&1928).unwrap().1, "@@ -4,13 +4,14 @@\n   \"EducatorEventsDays\": [\n     {\n       \"DayString\": \"Понедельник\",\n-      \"DayStudyEventsCount\": 1,\n+      \"DayStudyEventsCount\": 2,\n       \"DayStudyEvents\": [\n         {\n           \"TimeIntervalString\": \"08:30-10:00\",\n           \"Subject\": \"Как превратить искусство в массовый продукт\",\n           \"Dates\": [\n-            \"01.09.1963\"\n+            \"01.09.1963\",\n+            \"08.09.1963\"\n           ],\n           \"EventLocations\": [\n             {\n@@ -21,6 +22,54 @@\n             {\n               \"Item1\": \"Группа\",\n               \"Item2\": \"101A\"\n+            },\n+            {\n+              \"Item1\": \"Группа\",\n+              \"Item2\": \"101B\"\n+            }\n+          ]\n+        },\n+        {\n+          \"TimeIntervalString\": \"10:15-11:45\",\n+          \"Subject\": \"Истоки поп-арта\",\n+          \"Dates\": [\n+            \"01.09.1968\",\n+            \"08.09.1968\"\n+          ],\n+          \"EventLocations\": [\n+            {\n+              \"DisplayName\": \"33 Union Square West\"\n+            }\n+          ],\n+          \"ContingentUnitNames\": [\n+            {\n+              \"Item1\": \"Группа\",\n+              \"Item2\": \"102B\"\n+            }\n+          ]\n+        }\n+      ]\n+    },\n+    {\n+      \"DayString\": \"Среда\",\n+      \"DayStudyEventsCount\": 1,\n+      \"DayStudyEvents\": [\n+        {\n+          \"TimeIntervalString\": \"13:00-14:30\",\n+          \"Subject\": \"Истоки поп-арта\",\n+          \"Dates\": [\n+            \"02.09.1968\",\n+            \"10.09.1968\"\n+          ],\n+          \"EventLocations\": [\n+            {\n+              \"DisplayName\": \"33 Union Square West\"\n+            }\n+          ],\n+          \"ContingentUnitNames\": [\n+            {\n+              \"Item1\": \"Группа\",\n+              \"Item2\": \"103C\"\n             }\n           ]\n         }\n");
        assert_eq!(diff.get(&1879).unwrap().1, "@@ -4,12 +4,13 @@\n   \"EducatorEventsDays\": [\n     {\n       \"DayString\": \"Вторник\",\n-      \"DayStudyEventsCount\": 1,\n+      \"DayStudyEventsCount\": 2,\n       \"DayStudyEvents\": [\n         {\n           \"TimeIntervalString\": \"09:00-10:30\",\n           \"Subject\": \"От кубизма к супрематизму\",\n           \"Dates\": [\n+            \"22.12.1915\",\n             \"29.12.1915\"\n           ],\n           \"EventLocations\": [\n@@ -27,6 +28,25 @@\n               \"Item2\": \"201B\"\n             }\n           ]\n+        },\n+        {\n+          \"TimeIntervalString\": \"11:00-12:30\",\n+          \"Subject\": \"Декларация прав художника\",\n+          \"Dates\": [\n+            \"15.08.1918\",\n+            \"22.08.1918\"\n+          ],\n+          \"EventLocations\": [\n+            {\n+              \"DisplayName\": \"Дворцовая площадь, д. 6/8\"\n+            }\n+          ],\n+          \"ContingentUnitNames\": [\n+            {\n+              \"Item1\": \"Группа\",\n+              \"Item2\": \"202A\"\n+            }\n+          ]\n         }\n       ]\n     }\n");
    }

    #[test]
    fn format_event_as_string1() {
        let test_event = DayStudyEvent {
            time_interval_string: "09:30-11.00".to_string(),
            subject: "Матлогика".to_string(),
            dates: vec!["01.09.2025".to_string()],
            event_locations: vec![EventLocation {
                display_name: "Университетский пр. 28Д".to_string(),
            }],
            contingent_unit_names: vec![ContingentUnitName {
                item1: "Группа".to_string(),
                item2: "23.Б15-мм".to_string(),
            }],
        };
        let formatted_event = format_event_as_string(&test_event);
        assert_eq!(formatted_event, "    <b>Предмет:</b> Матлогика<br>    <b>Время:</b> 09:30-11.00<br>    <b>Даты:</b> 01.09.2025<br>    <b>Места:</b> Университетский пр. 28Д<br>    <b>Направления:</b> Группа 23.Б15-мм<br>")
    }

    #[test]
    fn vec_eq_unordered_equal() {
        let fst = [1, 2, 3];
        assert!(vec_eq_unordered(&fst, &fst))
    }

    #[test]
    fn vec_eq_unordered_mixed_order() {
        let fst = [3, 1, 2];
        let snd = [1, 2, 3];
        assert!(vec_eq_unordered(&fst, &snd))
    }

    #[test]
    fn vec_eq_unordered_unequal() {
        let fst = [3, 3, 2];
        let snd = [1, 2, 3];
        assert!(!vec_eq_unordered(&fst, &snd))
    }

    #[test]
    fn vec_eq_unordered_dif_length() {
        let fst = [3, 3];
        let snd = [1, 2, 3];
        assert!(!vec_eq_unordered(&fst, &snd))
    }

    #[test]
    fn event_eq_equal() {
        let test_event = make_event(
            "09:30-11.00",
            "Матлогика",
            vec!["01.09.2025", "03.09.2025"],
            vec!["Университетский пр. 28Д", "Менделеевская л. 2"],
            vec!["23.Б15-мм", "23.Б11-мм"],
        );
        assert!(event_eq(&test_event, &test_event))
    }

    #[test]
    fn event_eq_mixed_order() {
        let old = make_event(
            "09:30-11.00",
            "Матлогика",
            vec!["01.09.2025", "03.09.2025"],
            vec!["Университетский пр. 28Д", "Менделеевская л. 2"],
            vec!["23.Б15-мм", "23.Б11-мм"],
        );
        let new = make_event(
            "09:30-11.00",
            "Матлогика",
            vec!["03.09.2025", "01.09.2025"],
            vec!["Менделеевская л. 2", "Университетский пр. 28Д"],
            vec!["23.Б11-мм", "23.Б15-мм"],
        );
        assert!(event_eq(&old, &new))
    }

    #[test]
    fn event_eq_unequal() {
        let old = make_event(
            "09:30-11.00",
            "Матлогика",
            vec!["01.09.2025", "03.09.2025"],
            vec!["Университетский пр. 28Д", "Менделеевская л. 2"],
            vec!["23.Б15-мм", "23.Б11-мм"],
        );
        let new = make_event(
            "09:30-11.00",
            "Матлогика",
            vec!["03.09.2025", "01.09.2025"],
            vec!["Менделеевская л. 2", "Университетский пр. 28Д"],
            vec!["23.Б11-мм"],
        );
        assert!(!event_eq(&old, &new))
    }

    /* Cases
    only addition:
    1. new day
    2. old day, new event
    3. old day, old event, new group
    4. new educator
    only deletion:
    1. last event of the day
    2. one of many events of the day
    mixed:
    1. same day, one addition, one deletion
    2. different days, one addition, one deletion*/

    /*
    diff:   Среда:
            Предмет: Истоки поп-арта
            Время: 13:00-14:30
            Даты: 02.09.1968, 10.09.1968
            Места: 33 Union Square West
            Направления: Группа 103C */
    #[test]
    fn generate_diff_messages_new_day() {
        let args_old = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };
        let args_new = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.new_day.json"),
        };

        let old = get_previous_events(&args_old).unwrap();
        let new = get_previous_events(&args_new).unwrap();
        let diff = generate_diff_messages(&old, &new);
        assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Среда:<font size=\"10\"></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br>");
        assert_eq!(diff.get(&1879), None);
    }

    /*
    diff:   Понедельник:
            Предмет: Истоки поп-арта
            Время: 13:00-14:30
            Даты: 02.09.1968, 10.09.1968
            Места: 33 Union Square West
            Направления: Группа 103C */
    #[test]
    fn generate_diff_messages_old_day_new_event() {
        let args_old = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };
        let args_new = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.old_day_new_event.json"),
        };

        let old = get_previous_events(&args_old).unwrap();
        let new = get_previous_events(&args_new).unwrap();
        let diff = generate_diff_messages(&old, &new);
        assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br>");
        assert_eq!(diff.get(&1879), None);
    }
    
    /*
    diff:   Понедельник:
            Предмет: Как превратить искусство в массовый продукт
            Время: 08:30-10:00
            Даты: 01.09.1963
            Места: 231 East 47th Street
            Направления: Группа 101A, Группа 101B */
    #[test]
    fn generate_diff_messages_old_day_old_event_new_group() {
        let args_old = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };
        let args_new = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.old_day_old_event_new_group.json"),
        };

        let old = get_previous_events(&args_old).unwrap();
        let new = get_previous_events(&args_new).unwrap();
        let diff = generate_diff_messages(&old, &new);
        assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br>");
        assert_eq!(diff.get(&1879), None);
    }
    
    /*
    diff:   Вторник:
            Предмет: От кубизма к супрематизму
            Время: 09:00-10:30
            Даты: 29.12.1915
            Места: Дворцовая площадь, д. 6/8
            Направления: Группа 201A, Группа 201B */
    #[test]
    fn generate_diff_messages_new_educator() {
        let args_old = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.only_warhol.json"),
        };
        let args_new = Args {
            users_json_path: PathBuf::from("tests/test.users.json"),
            config_json_path: PathBuf::from("tests/test.config.json"),
            previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
        };

        let old = get_previous_events(&args_old).unwrap();
        let new = get_previous_events(&args_new).unwrap();
        let diff = generate_diff_messages(&old, &new);
        assert_eq!(diff.get(&1928), None);
        assert_eq!(diff.get(&1879).unwrap().1, "<b><font size=\"5\">Вторник:<font size=\"10\"></b><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br>");
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
}
