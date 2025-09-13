use log::info;

use super::{
    helpers::{generate_diff_messages, get_previous_events, get_users, write_previous_events},
    letter_sender::LetterSender,
    models::{Args, Config},
    schedule_getter::ScheduleGetter,
};

pub async fn run<SG: ScheduleGetter, LS: LetterSender>(
    schedule_getter: SG,
    letter_sender: LS,
    args: &Args,
    config: Config,
) -> () {
    let users = get_users(&args).unwrap();
    let educator_events_old = get_previous_events(&args).unwrap();
    info!("Found {} educators in db", educator_events_old.len());
    let educator_events_new = schedule_getter.get_schedule(&users).await;
    let educators_changed = generate_diff_messages(&educator_events_old, &educator_events_new);
    info!(
        "Found {} changed educators schedules",
        educators_changed.len()
    );
    letter_sender.form_and_send_letters(users, config, educators_changed);
    write_previous_events(&args, educator_events_new).unwrap();
}
