use std::collections::{BTreeMap, BTreeSet};

use futures::future;
use log::info;
use reqwest::Client;

use super::{
    helpers::get_educator_events_by_id,
    models::{educator_model::EducatorEvents, User},
};

// probably do smth about this warning later
#[allow(async_fn_in_trait)]
pub trait ScheduleGetter {
    async fn get_schedule(&self, users: &Vec<User>) -> BTreeMap<u32, EducatorEvents>;
}

impl ScheduleGetter for Client {
    async fn get_schedule(&self, users: &Vec<User>) -> BTreeMap<u32, EducatorEvents> {
        let watched_educators = users
            .iter()
            .flat_map(|user| &user.watch_educators)
            .cloned()
            .collect::<BTreeSet<_>>();
        /* Collect new info from timetable about all watched educators */
        let educator_events_new = future::join_all(
            watched_educators
                .into_iter()
                .map(|id| get_educator_events_by_id(&self, id)),
        )
        .await
        .into_iter()
        .collect::<Result<BTreeMap<_, _>, _>>()
        .unwrap();
        info!("Collected {} educator events", educator_events_new.len());
        return educator_events_new;
    }
}
