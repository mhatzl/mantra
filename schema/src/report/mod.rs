pub mod annotations;
pub mod evidence_matrix;
pub mod nav;
//pub mod overview;
pub mod product;
pub mod products;
pub mod requirement;
pub mod requirements;
pub mod review;
pub mod reviews;
pub mod source_file;
pub mod source_folder;
pub mod sources;
pub mod test_case;
pub mod test_run;
pub mod test_runs;
pub mod tests;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct Aggregated {
    pub cnt: i64,
    pub percentage: f64,
}

impl Aggregated {
    pub fn update_percentage(&mut self, total: i64) {
        if total == 0 || self.cnt == 0 {
            self.percentage = 0.0;
        } else {
            self.percentage = (self.cnt as f64 / total as f64) * 100.0;
        }
    }
}

fn encode_utc_date(utc_date: &time::OffsetDateTime) -> String {
    format!(
        "{}-{:02}-{:02}_{:02}-{:02}-{:02}",
        utc_date.year(),
        utc_date.month() as u8,
        utc_date.day(),
        utc_date.hour(),
        utc_date.minute(),
        utc_date.second()
    )
}
