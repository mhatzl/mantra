/// The line in a file that was covered by a mantra macro.
#[cfg_attr(feature = "extract", derive(Debug, Clone, PartialEq, Eq))]
pub struct LineCoverage {
    line: u32,
    #[cfg(not(feature = "extract"))]
    file: &'static str,
    #[cfg(feature = "extract")]
    file: String,
}

impl LineCoverage {
    /// Create a new line coverage preferably using `line!()` and `file!()`.
    #[cfg(not(feature = "extract"))]
    pub fn new(line: u32, file: &'static str) -> Self {
        Self { line, file }
    }

    /// Create a new line coverage preferably using `line!()` and `file!()`.
    #[cfg(feature = "extract")]
    pub fn new(line: u32, file: impl Into<String>) -> Self {
        Self {
            line,
            file: file.into(),
        }
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn file(&self) -> &str {
        #[cfg(not(feature = "extract"))]
        return self.file;

        #[cfg(feature = "extract")]
        &self.file
    }

    pub fn log(&self) {
        #[cfg(feature = "defmt")]
        defmt::println!("{}", self);

        #[cfg(feature = "log")]
        log::info!("{}", self);
    }

    /// Returns `true` if the given string starts like a mantra coverage log message.
    ///
    /// `mantra coverage at line='`
    pub fn potential_log(msg: &str) -> bool {
        msg.starts_with("mantra coverage at line='")
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for LineCoverage {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", defmt::Display2Format(self));
    }
}

impl core::fmt::Display for LineCoverage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "mantra coverage at line='{}' in file: {}",
            self.line,
            self.file()
        )
    }
}

#[cfg(feature = "extract")]
static MANTRA_LINE_COV_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"mantra coverage at line='(?<line>\d+)' in file: (?<file>.+)").unwrap()
});

#[cfg(feature = "extract")]
const FILE_MATCH_NAME: &str = "file";
#[cfg(feature = "extract")]
const LINE_MATCH_NAME: &str = "line";

#[cfg(feature = "extract")]
impl core::str::FromStr for LineCoverage {
    type Err = ExtractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match MANTRA_LINE_COV_REGEX.captures(s) {
            Some(captures) => {
                let file_match = captures
                    .name(FILE_MATCH_NAME)
                    .ok_or(ExtractError::BadFile)?;
                let file = file_match.as_str();
                let line_match = captures
                    .name(LINE_MATCH_NAME)
                    .ok_or(ExtractError::BadLine)?;
                let line: u32 = line_match
                    .as_str()
                    .parse()
                    .map_err(|_| ExtractError::BadLine)?;

                Ok(LineCoverage::new(line, file))
            }
            None => Err(ExtractError::Unmatched),
        }
    }
}

#[cfg(feature = "extract")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractError {
    Unmatched,
    BadLine,
    BadFile,
}

#[cfg(all(test, feature = "extract"))]
mod tests {
    use core::{assert_eq, str::FromStr};

    use crate::coverage::{ExtractError, LineCoverage};

    #[test]
    fn extract_valid_coverage() {
        let coverage = LineCoverage::new(1, "some-file");

        let extracted = LineCoverage::from_str(&coverage.to_string()).unwrap();
        assert_eq!(
            coverage, extracted,
            "Extracted (right) from orig (left) differ"
        );
    }

    #[test]
    fn extract_valid_coverage_longer_filepath() {
        let coverage = LineCoverage::new(1, "path/path/path/path/some-longer-nested-filepath");

        let extracted = LineCoverage::from_str(&coverage.to_string()).unwrap();
        assert_eq!(
            coverage, extracted,
            "Extracted (right) from orig (left) differ"
        );
    }

    #[test]
    fn unmatched_msg() {
        let msg = "mantra coverage at line='1";
        let err = LineCoverage::from_str(msg).unwrap_err();

        assert!(
            LineCoverage::potential_log(msg),
            "Message does not start like a mantra coverage log"
        );
        assert_eq!(
            err,
            ExtractError::Unmatched,
            "Message is not a valid mantra coverage log"
        )
    }

    #[test]
    fn bad_line() {
        let msg = "mantra coverage at line='123456789000' in file: some-file";
        let err = LineCoverage::from_str(msg).unwrap_err();

        assert!(
            LineCoverage::potential_log(msg),
            "Message does not start like a mantra coverage log"
        );
        assert_eq!(err, ExtractError::BadLine, "Line number exceeds u32")
    }
}
