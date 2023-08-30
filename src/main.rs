use std::path::PathBuf;

use references::ReferencesMap;
use wiki::Wiki;

mod references;
mod req;
mod sync;
mod wiki;

fn main() {
    let start = std::time::Instant::now();

    // let wiki = Wiki::try_from(PathBuf::from("../../evident-wiki/5-Requirements")).unwrap();
    // let ref_map = ReferencesMap::try_from((&wiki, PathBuf::from("../../evident"))).unwrap();

    let _ = sync::sync(sync::SyncParameter {
        branch_name: "main".to_string(),
        proj_folder: PathBuf::from("../../evident"),
        req_folder: PathBuf::from("../../evident-wiki/5-Requirements"),
        wiki_url_prefix: String::new(),
    });

    let end = std::time::Instant::now();

    // dbg!(wiki.sub_reqs(&format!("subs")));
    // dbg!(wiki);
    // dbg!(ref_map);

    println!(
        "Took: {}ms",
        end.checked_duration_since(start).unwrap().as_millis()
    );
}
