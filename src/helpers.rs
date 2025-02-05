use std::collections::HashMap;

use log::{debug, info};
use reqwest::Client;
use similar::TextDiff;

use crate::models::{educator_model::EducatorEvents, User};

pub fn log_all_users(users: &[User]) -> () {
    for user in users.iter() {
        debug!(
            "Serving {}, who is watching for educators {:?} and groups {:?}",
            user.name, user.watch_educators, user.watch_groups
        );
    }
}

pub async fn get_educator_events_by_id(
    http_client: &Client,
    id: u32,
) -> Result<EducatorEvents, reqwest::Error> {
    info!("Getting events for educator {}", id);
    let request_url = format!("https://timetable.spbu.ru/api/v1/educators/{}/events", id);
    let response = http_client.get(request_url).send().await?;
    response.json().await
}

// Note to myself: this is probably the first time I have done some weird magic
// TODO: read about lifetimes
pub fn find_diffs_in_events<'a>(
    new_events: &'a HashMap<u32, String>,
    old_events: &HashMap<u32, String>,
) -> HashMap<u32, (&'a str, String)> {
    let mut out_map: HashMap<u32, (&str, String)> = HashMap::new();

    for (new_event_id, new_event_str) in new_events.iter() {
        let old_event_str = old_events.get(new_event_id).unwrap(); // unwrap here must be safe!
        let diff = TextDiff::from_lines(old_event_str, new_event_str);
        if diff.ratio() != 1.0 {
            let pretty_diff = diff.unified_diff();
            debug!("Changes for {}: {}", new_event_id, pretty_diff);
            out_map.insert(
                *new_event_id,
                (new_event_str.as_str(), pretty_diff.to_string()),
            );
        }
    }

    out_map
}
