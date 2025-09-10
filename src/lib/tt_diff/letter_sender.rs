use std::collections::HashMap;

use lettre::{SmtpTransport, Transport};
use log::info;

use crate::tt_diff::helpers::generate_email;

use super::{
    helpers::collect_all_tracked_diffs,
    models::{educator_model::EducatorEvents, Config, User},
};

pub trait LetterSender {
    fn form_and_send_letters(
        self,
        users: Vec<User>,
        config: Config,
        ed_changed: HashMap<u32, (&EducatorEvents, String)>,
    );
}

impl LetterSender for SmtpTransport {
    fn form_and_send_letters(
        self,
        users: Vec<User>,
        config: Config,
        ed_changed: HashMap<u32, (&EducatorEvents, String)>,
    ) {
        for user in users.iter() {
            let diff = collect_all_tracked_diffs(&ed_changed, user);
            if diff.len() > 0 {
                let email = generate_email(&config, user, &diff).unwrap();
                let code = self.send(&email).unwrap();
                info!("Sent email to {} with response {:?}", user.name, code);
            }
        }
    }
}
