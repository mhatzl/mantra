use mantra_rust_macros::{req, reqcov};

use std::io::Write;

#[req(123, 321)]
#[req(my_req.test)]
fn attrb_macro_usage() {
    println!("fn body");

    reqcov!("direct-req".test, 42);
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

    Test::me();

    Other::me();
}

#[req(on_const)]
pub const MY_REQ: usize = 1;

#[req(on_type)]
pub type MyType = bool;

#[req(on_enum)]
pub enum MyEnum {
    /// [req(variant_req)]
    /// Attribute macros cannot be set for variants directly.
    First,
}

#[req(123)]
pub struct Test {
    /// [req(field_req)]
    /// Attribute macros cannot be set for fields directly.
    pub my_field: bool,
}

#[req(test_req)]
mod reqs {}

#[req(some_req)]
trait Something {
    #[req(trait_type)]
    type A;

    #[req(works)]
    fn me() {}
}

#[req(impl_req)]
impl Something for Test {
    #[req(set_type)]
    type A = bool;

    #[req(still_works)]
    fn me() {}
}

struct Other;

impl Something for Other {
    type A = String;
}
