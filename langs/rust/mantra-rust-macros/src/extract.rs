use std::path::PathBuf;

use once_cell::unsync::Lazy;
use regex::bytes::Regex;

const REQ_ID_MATCH_NAME: &str = "id";
const FILE_MATCH_NAME: &str = "file";
const LINE_MATCH_NAME: &str = "line";

thread_local! {
    static REQ_COV_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"mantra: req-id='(?<id>.+)'; file='(?<file>.+)'; line='(?<line>\d+)';").unwrap());
}

#[derive(Debug)]
pub struct CoveredReq {
    pub id: Vec<String>,
    pub file: PathBuf,
    pub line: u32,
}

pub fn extract_covered_reqs(content: &[u8]) -> Option<Vec<CoveredReq>> {
    REQ_COV_REGEX.with(|re| {
        let mut reqs = Vec::new();
        let captures = re.captures_iter(content);

        for cap in captures {
            let combined_id = String::from_utf8(cap[REQ_ID_MATCH_NAME].to_vec()).ok()?;
            let file = PathBuf::from(String::from_utf8(cap[FILE_MATCH_NAME].to_vec()).ok()?);
            let line: u32 = String::from_utf8(cap[LINE_MATCH_NAME].to_vec())
                .ok()?
                .parse()
                .ok()?;

            reqs.push(CoveredReq {
                id: combined_id.split('.').map(|s| s.to_string()).collect(),
                file,
                line,
            })
        }
        Some(reqs)
    })
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::ReqCovStatic;

    use super::extract_covered_reqs;

    #[test]
    fn extract_root_req() {
        let id = "my_id";
        let file = file!();
        let line = line!();

        let intern_req_cov = ReqCovStatic { id, file, line };
        let displayed_req_cov = intern_req_cov.to_string();

        let extracted_reqs = extract_covered_reqs(displayed_req_cov.as_bytes()).unwrap();

        assert_eq!(extracted_reqs.len(), 1, "Single requirement not extracted.");

        let root_req = extracted_reqs.first().unwrap();

        assert_eq!(root_req.id.len(), 1, "Root ID was split.");
        assert_eq!(
            root_req.id.first().unwrap(),
            id,
            "Extracted ID differs from original."
        );

        assert_eq!(
            root_req.file,
            PathBuf::from(file),
            "Extracted file differs from original."
        );
        assert_eq!(
            root_req.line, line,
            "Extracted line number differs from original."
        );
    }
}
