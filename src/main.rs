use std::path::PathBuf;

use wiki::Wiki;

mod req;
mod sync;
mod trace;
mod wiki;

fn main() {
    let wiki = Wiki::try_from(PathBuf::from("../../evident-wiki/5-Requirements")).unwrap();

    dbg!(wiki);
}
