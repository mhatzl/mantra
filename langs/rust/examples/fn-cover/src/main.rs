use mantra_rust_macros::{req, reqcov};

use std::io::Write;

#[req(123, 321)]
#[req(my_req.test)]
fn attrb_macro_usage() {
    println!("fn body");

    reqcov!("direct-req".test, 2387192);
}

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} ('{}', '{}')",
                record.level(),
                record.args(),
                record.file().unwrap_or("undefined"),
                record.line().unwrap_or_default()
            )
        })
        .init();

    attrb_macro_usage();
}
