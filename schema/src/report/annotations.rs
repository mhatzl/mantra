use relative_path::RelativePathBuf;

use crate::{Line, annotations::TraceKind, report::Aggregated};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TraceReference {
    #[schemars(with = "String")]
    pub path: RelativePathBuf,
    pub line: Option<Line>,
    pub kind: TraceKind,
}

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
pub struct TracesSummary {
    pub total: i64,
    pub satisfies: Aggregated,
    pub verifies: Aggregated,
    pub clarifies: Aggregated,
    pub links: Aggregated,
}

impl TracesSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.satisfies.cnt += other.satisfies.cnt;
        self.verifies.cnt += other.verifies.cnt;
        self.clarifies.cnt += other.clarifies.cnt;
        self.links.cnt += other.links.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.satisfies.update_percentage(self.total);
        self.verifies.update_percentage(self.total);
        self.clarifies.update_percentage(self.total);
        self.links.update_percentage(self.total);
    }
}
