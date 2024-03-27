pub use mantra_rust_procm::req;

#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            println!(
                "mantra: '{}'; file='{}'; line='{}'",
                $req_id,
                file!(),
                line!()
            );
        )+
    };
}
