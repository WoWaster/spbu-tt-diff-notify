use std::collections::BTreeMap;
use std::path::PathBuf;

use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Json;
use figment::Figment;
use lettre::address::Envelope;
use lettre::message::header::ContentType;
use lettre::transport::stub::StubTransport;
use lettre::Address;
use lettre::Message;
use lettre::Transport;
use lib::tt_diff::helpers::collect_all_tracked_diffs;
use lib::tt_diff::helpers::generate_email;
use lib::tt_diff::helpers::get_previous_events;
use lib::tt_diff::letter_sender::LetterSender;
use lib::tt_diff::models::educator_model::EducatorEvents;
use lib::tt_diff::models::Args;
use lib::tt_diff::models::{Config, User};
use lib::tt_diff::run_tool::run;
use lib::tt_diff::schedule_getter::ScheduleGetter;
use mailparse::parse_mail;

pub struct TestGetter {
    pub new_schedule_path: String,
}

impl ScheduleGetter for TestGetter {
    async fn get_schedule(&self, _users: &Vec<User>) -> BTreeMap<u32, EducatorEvents> {
        let mock_site = Args {
            users_json_path: PathBuf::from("_"),
            config_json_path: PathBuf::from("_"),
            previous_events_json_path: PathBuf::from(self.new_schedule_path.clone()),
        };
        let new_schedule = get_previous_events(&mock_site).unwrap();
        return new_schedule;
    }
}

pub struct TestSender {
    pub transport: StubTransport,
    // Hashmap uses user mail address as unique identifier, (Envelope, String, Message) = (letter headers, contents of letter, Message to assert bodies equal in readable format)
    // Option is for cases when some educators stay the same, ergo should not have any letter at all
    pub expected: BTreeMap<String, Option<(Envelope, String, Message)>>,
}

// ugly, but does the trick. otherwise Message would be in serialised format, totally unreadable
pub fn assert_emails_eq(fst: &Message, snd: &Message) {
    let fst_raw = fst.formatted();
    let snd_raw = snd.formatted();

    let fst_readable = parse_mail(&fst_raw).unwrap();
    let snd_readable = parse_mail(&snd_raw).unwrap();

    assert_eq!(
        fst_readable.get_body().unwrap(),
        snd_readable.get_body().unwrap()
    );
}

impl LetterSender for TestSender {
    fn form_and_send_letters(
        self,
        users: Vec<User>,
        config: Config,
        ed_changed: BTreeMap<u32, (&EducatorEvents, String)>,
    ) {
        for user in users.iter() {
            let user_id = &user.email;
            let diff = collect_all_tracked_diffs(&ed_changed, user);

            if diff.len() > 0 {
                let email = generate_email(&config, user, &diff).unwrap();
                let _ = self.transport.send(&email);
                let expected_email = self
                    .expected
                    .get(user_id)
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .2
                    .clone();
                // assert body in readable format
                assert_emails_eq(&email, &expected_email);
                // assert headers and serialised letter contents
                assert_eq!(
                    self.transport.messages(),
                    vec![
                        ((
                            self.expected
                                .get(user_id)
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .0
                                .clone(),
                            self.expected
                                .get(user_id)
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .1
                                .clone()
                        ))
                    ],
                )
            } else {
                // indicates that if diff length is 0, then there is no letter in the expected
                assert!(self.expected.get(user_id).unwrap().is_none())
            }
        }
    }
}

#[tokio::test]
async fn test_main() {
    let args = Args {
        users_json_path: PathBuf::from("tests/test.users.json"),
        config_json_path: PathBuf::from("example.config.json"),
        previous_events_json_path: PathBuf::from("tests/test.less_events.json"),
    };
    let config: Config = Figment::new()
        .merge(Json::file(&args.config_json_path))
        .merge(Env::prefixed("TT_"))
        .extract()
        .unwrap();

    // set up test getter from JSON
    let test_getter = TestGetter {
        new_schedule_path: "tests/test.many_events.json".to_string(),
    };

    let test_transport = StubTransport::new_ok();

    let mut test_expected = BTreeMap::new();

    let sender_address = config.email_sender_username.parse::<Address>().unwrap();
    let recipients_addresses = vec!["campbellsoupthebest@gmail.com".parse::<Address>().unwrap()];

    let warhol_envelope = Envelope::new(Some(sender_address), recipients_addresses).unwrap();

    let warhol_email = Message::builder()
    .from(format!("{} <{}>", config.email_sender_fullname, config.email_sender_username).parse().unwrap())
    .to("Энди Уорхол <campbellsoupthebest@gmail.com>".parse().unwrap())
    .subject("Изменилось расписание преподавателя!")
    .header(ContentType::TEXT_HTML)
    .body(String::from("Уважаемый(ая) Энди Уорхол!<br><br> В расписании преподавателя <b>Казимир Малевич</b> произошли изменения:<br><br><b><font size=\"5\">Вторник:</font></b><br><em style=\"color:green;\">Новые события:</em><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 22.12.1915, 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br>    <b>Предмет:</b> Декларация прав художника<br>    <b>Время:</b> 11:00-12:30<br>    <b>Даты:</b> 15.08.1918, 22.08.1918<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 202A<br><br><em style=\"color:red;\">Удалённые события:</em><br>    <b>Предмет:</b> От кубизма к супрематизму<br>    <b>Время:</b> 09:00-10:30<br>    <b>Даты:</b> 29.12.1915<br>    <b>Места:</b> Дворцовая площадь, д. 6/8<br>    <b>Направления:</b> Группа 201A, Группа 201B<br><br><br> <br>В расписании преподавателя <b>Энди Уорхол</b> произошли изменения:<br><br><b><font size=\"5\">Понедельник:</font></b><br><em style=\"color:green;\">Новые события:</em><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963, 08.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A, Группа 101B<br><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 10:15-11:45<br>    <b>Даты:</b> 01.09.1968, 08.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 102B<br><br><em style=\"color:red;\">Удалённые события:</em><br>    <b>Предмет:</b> Как превратить искусство в массовый продукт<br>    <b>Время:</b> 08:30-10:00<br>    <b>Даты:</b> 01.09.1963<br>    <b>Места:</b> 231 East 47th Street<br>    <b>Направления:</b> Группа 101A<br><br><em style=\"color:green;\">Новый день:</em><br><b><font size=\"5\">Среда:</font></b><br>    <b>Предмет:</b> Истоки поп-арта<br>    <b>Время:</b> 13:00-14:30<br>    <b>Даты:</b> 02.09.1968, 10.09.1968<br>    <b>Места:</b> 33 Union Square West<br>    <b>Направления:</b> Группа 103C<br><br> <br> Данное письмо было сгенерировано автоматически, направление ответа не подразумевается.")).unwrap();

    let warhol_contents = String::from_utf8(warhol_email.formatted()).unwrap();

    test_expected.insert(
        "campbellsoupthebest@gmail.com".to_string(),
        Some((warhol_envelope, warhol_contents, warhol_email)),
    );

    // set up test sender, specifically: mock transport, map of expected headers, contents of letters
    let test_sender = TestSender {
        transport: test_transport,
        expected: test_expected,
    };

    let _ = run(test_getter, test_sender, &args, config).await;
}
