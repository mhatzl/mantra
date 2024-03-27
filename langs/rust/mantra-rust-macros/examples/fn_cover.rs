use mantra_rust_macros::req;

#[req(123, 321)]
#[req(my_req.test)]
fn attrb_macro_usage() {
    println!("fn body");
}

pub fn main() {
    attrb_macro_usage();
}
