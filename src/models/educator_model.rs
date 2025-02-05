//! Module with educator model compatible with timetable.spbu.ru's REST API
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventLocation {
    pub display_name: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DayStudyEvent {
    pub time_interval_string: String,
    pub subject: String,
    pub dates: Vec<String>,
    pub event_locations: Vec<EventLocation>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EducatorDay {
    pub day_string: String,
    pub day_study_events_count: u8,
    pub day_study_events: Vec<DayStudyEvent>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EducatorEvents {
    pub educator_long_display_text: String,
    pub educator_master_id: i64,
    pub educator_events_days: Vec<EducatorDay>,
}
