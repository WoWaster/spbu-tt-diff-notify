use std::collections::{BTreeMap, BTreeSet};

use futures::future;
use log::info;
use reqwest::Client;

use super::{
    helpers::get_educator_events_by_id,
    models::{educator_model::EducatorEvents, User},
};

/// A trait, necessary for every entity that will be used for getting actual schedule.
#[allow(async_fn_in_trait)]
pub trait ScheduleGetter {
    async fn get_schedule(&self, users: &[User]) -> BTreeMap<u32, EducatorEvents>;
}

/// Allows to use Client for getting actual schedule via requests to TimeTable resource.
impl ScheduleGetter for Client {
    async fn get_schedule(&self, users: &[User]) -> BTreeMap<u32, EducatorEvents> {
        let watched_educators = users
            .iter()
            .flat_map(|user| &user.watch_educators)
            .cloned()
            .collect::<BTreeSet<_>>();
        /* Collect new info from timetable about all watched educators */
        let educator_events_new = future::join_all(
            watched_educators
                .into_iter()
                .map(|id| get_educator_events_by_id(self, id)),
        )
        .await
        .into_iter()
        .collect::<Result<BTreeMap<_, _>, _>>()
        .unwrap();
        info!("Collected {} educator events", educator_events_new.len());
        educator_events_new
    }
}
