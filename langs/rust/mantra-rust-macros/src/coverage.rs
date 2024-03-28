pub struct ReqCov {
    pub id: &'static str,
    pub file: &'static str,
    pub line: u32,
}

#[cfg(feature = "defmt")]
impl defmt::Format for ReqCov {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "mantra: req-id='{=str}'; file='{=str}'; line='{}';",
            self.id,
            self.file,
            self.line
        )
    }
}

impl core::fmt::Display for ReqCov {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "mantra: req-id='{}'; file='{}'; line='{}';",
            self.id, self.file, self.line
        )
    }
}

#[cfg(feature = "decode")]
use once_cell::unsync::Lazy;
#[cfg(feature = "decode")]
use regex::bytes::Regex;

#[cfg(feature = "decode")]
pub const REQ_ID_MATCH_NAME: &str = "id";
#[cfg(feature = "decode")]
pub const FILE_MATCH_NAME: &str = "file";
#[cfg(feature = "decode")]
pub const LINE_MATCH_NAME: &str = "line";

#[cfg(feature = "decode")]
thread_local! {
    pub static REQ_COV_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"mantra: req-id='(?<id>.+)'; file='(?<file>.+)'; line='(?<line>\d+)';").unwrap());
}
