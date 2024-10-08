use time::PrimitiveDateTime;

use super::requirements::ReqId;

pub const REVIEW_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
);

time::serde::format_description!(review_date_format, PrimitiveDateTime, REVIEW_DATE_FORMAT);

pub fn date_from_str(date: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    PrimitiveDateTime::parse(date, REVIEW_DATE_FORMAT)
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ReviewSchema {
    pub name: String,
    #[serde(with = "review_date_format")]
    #[schemars(
        with = "String",
        regex(
            pattern = r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2}) (?<hour>\d{2}):(?<minute>\d{2})(?<second>:\d{2}(?<subsecond>\.\d{3})?)?"
        )
    )]
    pub date: PrimitiveDateTime,
    pub reviewer: String,
    pub comment: Option<String>,
    pub requirements: Vec<VerifiedRequirement>,
}

#[derive(
    Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct VerifiedRequirement {
    pub id: ReqId,
    pub comment: Option<String>,
}
