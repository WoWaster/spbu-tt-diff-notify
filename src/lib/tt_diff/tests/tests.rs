use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
};

use crate::tt_diff::models::educator_model::{ContingentUnitName, EducatorDay, EventLocation};

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
                dates: BTreeSet::from(["01.09.1963".to_string()]),
                event_locations: BTreeSet::from([EventLocation {
                    display_name: "231 East 47th Street".to_string(),
                }]),
                contingent_unit_names: BTreeSet::from([ContingentUnitName {
                    item1: "Группа".to_string(),
                    item2: "101A".to_string(),
                }]),
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
                dates: BTreeSet::from(["29.12.1915".to_string()]),
                event_locations: BTreeSet::from([EventLocation {
                    display_name: "Дворцовая площадь, д. 6/8".to_string(),
                }]),
                contingent_unit_names: BTreeSet::from([
                    ContingentUnitName {
                        item1: "Группа".to_string(),
                        item2: "201A".to_string(),
                    },
                    ContingentUnitName {
                        item1: "Группа".to_string(),
                        item2: "201B".to_string(),
                    },
                ]),
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
    let expected_1928 = include_str!("exp_find_diffs_in_events1_1928.txt");
    let expected_1879 = include_str!("exp_find_diffs_in_events1_1879.txt");
    assert_eq!(diff.get(&1928).unwrap().1, expected_1928);
    assert_eq!(diff.get(&1879).unwrap().1, expected_1879);
}

#[test]
fn format_event_as_string1() {
    let test_event = DayStudyEvent {
        time_interval_string: "09:30-11.00".to_string(),
        subject: "Матлогика".to_string(),
        dates: BTreeSet::from(["01.09.2025".to_string()]),
        event_locations: BTreeSet::from([EventLocation {
            display_name: "Университетский пр. 28Д".to_string(),
        }]),
        contingent_unit_names: BTreeSet::from([ContingentUnitName {
            item1: "Группа".to_string(),
            item2: "23.Б15-мм".to_string(),
        }]),
    };
    let formatted_event = format_event_as_string(&test_event);
    assert_eq!(formatted_event, "    <b>Предмет:</b> Матлогика<br>    <b>Время:</b> 09:30-11.00<br>    <b>Даты:</b> 01.09.2025<br>    <b>Места:</b> Университетский пр. 28Д<br>    <b>Направления:</b> Группа 23.Б15-мм<br>")
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

#[test]
fn collect_all_tracked_diffs_multiple_diffs() {
    let args_old = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("example.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
    };
    let args_new = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("example.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.many_events.json"),
    };

    let users = get_users(&args_new).unwrap();
    let old = get_previous_events(&args_old).unwrap();
    let new = get_previous_events(&args_new).unwrap();
    let diff_test = generate_diff_messages(&old, &new);
    let diff = collect_all_tracked_diffs(&diff_test, &users[0]);
    // method .iter() of HashSet takes educators in arbitrary order, which is no problem for resulting letter, but pain for testing
    let malevich_first = "В расписании преподавателя <b>Казимир Малевич</b> произошли изменения:<br><br><b><font size=\"5\">Вторник:</font></b><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 22.12.1915, 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br>    <b>Предмет:</b> Декларация прав художника<br>    <b>Время:</b> 11:00-12:30<br>    <b>Даты:</b> 15.08.1918, 22.08.1918<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 202A<br><br><br> <br>В расписании преподавателя <b>Энди Уорхол</b> произошли изменения:<br><br><b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963, 08.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 10:15-11:45<br>    <b>Даты:</b> 01.09.1968, 08.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 102B<br><br><b><font size=\"5\">Среда:<font size=\"10\"></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br><br>";
    let warhol_first = "В расписании преподавателя <b>Энди Уорхол</b> произошли изменения:<br><br><b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963, 08.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 10:15-11:45<br>    <b>Даты:</b> 01.09.1968, 08.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 102B<br><br><b><font size=\"5\">Среда:<font size=\"10\"></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br><br><br> <br>В расписании преподавателя <b>Казимир Малевич</b> произошли изменения:<br><br><b><font size=\"5\">Вторник:</font></b><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 22.12.1915, 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br>    <b>Предмет:</b> Декларация прав художника<br>    <b>Время:</b> 11:00-12:30<br>    <b>Даты:</b> 15.08.1918, 22.08.1918<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 202A<br><br>";
    let diff_valid_mixed_educators_order = diff == malevich_first || diff == warhol_first;
    assert_eq!(diff_valid_mixed_educators_order, true)
}

#[test]
fn collect_all_tracked_diffs_no_diffs() {
    let args_old = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("example.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
    };

    let users = get_users(&args_old).unwrap();
    let old = get_previous_events(&args_old).unwrap();
    let diff_test = generate_diff_messages(&old, &old);
    let diff = collect_all_tracked_diffs(&diff_test, &users[0]);
    assert_eq!(diff, "")
}

/*
diff:   Понедельник:
        Предмет: Как превратить искусство в массовый продукт
        Время: 08:30-10:00
        Даты: 01.09.1963, 08.09.1963
        Места: 231 East 47th Street
        Направления: Группа 101A, Группа 101B

        Предмет: Истоки поп-арта
        Время: 10:15-11:45
        Даты: 01.09.1968, 08.09.1968
        Места: 33 Union Square West
        Направления: Группа 102B

        Вторник:
        Предмет: Как превратить искусство в массовый продукт
        Время: 09:00-10:30
        Даты: 22.12.1915, 29.12.1915
        Места: 231 East 47th Street
        Направления: Группа 201A, Группа 201B

        Среда:
        Предмет: Истоки поп-арта
        Время: 13:00-14:30
        Даты: 02.09.1968, 10.09.1968
        Места: 33 Union Square West
        Направления: Группа 103C */
#[test]
fn generate_diff_messages_many_days() {
    let args_old = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("tests/test.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
    };
    let args_new = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("tests/test.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.many_days.json"),
    };

    let old = get_previous_events(&args_old).unwrap();
    let new = get_previous_events(&args_new).unwrap();
    let diff = generate_diff_messages(&old, &new);
    assert_eq!(diff.get(&1928).unwrap().1, "<b><font size=\"5\">Понедельник:</font></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963, 08.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 10:15-11:45<br>    <b>Даты:</b> 01.09.1968, 08.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 102B<br><br><b><font size=\"5\">Вторник:<font size=\"10\"></b><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 22.12.1915, 29.12.1915<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br><b><font size=\"5\">Среда:<font size=\"10\"></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br>");
    assert_eq!(diff.get(&1879), None);
}
