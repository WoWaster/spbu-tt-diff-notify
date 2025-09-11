use std::collections::HashMap;

use log::info;

use crate::tt_diff::helpers::{generate_diff_messages, get_previous_events, get_users};

use super::{
    letter_sender::LetterSender,
    models::{educator_model::EducatorEvents, Args, Config},
    schedule_getter::ScheduleGetter,
};

pub async fn run<SG: ScheduleGetter, LS: LetterSender>(
    schedule_getter: SG,
    letter_sender: LS,
    args: &Args,
    config: Config,
) -> HashMap<u32, EducatorEvents> {
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
    return educator_events_new;
}
