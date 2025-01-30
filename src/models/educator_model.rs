use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct EventLocation {
    display_name: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct DayStudyEvent {
    time_interval_string: String,
    subject: String,
    dates: Vec<String>,
    event_locations: Vec<EventLocation>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct EducatorDay {
    day_string: String,
    day_study_events_count: u8,
    day_study_events: Vec<DayStudyEvent>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EducatorEvents {
    educator_long_display_text: String,
    educator_events_days: Vec<EducatorDay>,
}
