use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-changed=../mantra.db");

    let db = include_bytes!("../mantra.db");

    let out_path = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or(dirs::home_dir().expect("Home directory must exist to build mantra."))
        .join("mantra.db");

    std::fs::write(&out_path, db).expect("Prepared database written to build directory.");

    std::env::set_var("DATABASE_URL", out_path.as_os_str());
}
