use std::path::PathBuf;

use wiki::Wiki;

use crate::references::ReferencesMap;

mod references;
mod req;
mod sync;
mod wiki;

fn main() {
    let start = std::time::Instant::now();

    let wiki = Wiki::try_from(PathBuf::from("../../evident-wiki/5-Requirements")).unwrap();
    let ref_map = ReferencesMap::try_from((&wiki, PathBuf::from("../../evident"))).unwrap();

    let end = std::time::Instant::now();

    dbg!(wiki);
    dbg!(ref_map);

    println!(
        "Took: {}ms",
        end.checked_duration_since(start).unwrap().as_millis()
    );
}
