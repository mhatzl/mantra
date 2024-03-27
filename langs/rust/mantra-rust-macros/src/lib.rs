pub use mantra_rust_procm::req;

#[cfg(all(any(feature = "stdout", feature = "log"), feature = "defmt"))]
compile_error!("The 'defmt' feature may not be used together with features 'log' or 'stdout'.");

#[cfg(feature = "log")]
pub use log;

#[cfg(feature = "defmt")]
pub use defmt;

#[cfg(all(feature = "log", not(any(feature = "defmt", feature = "stdout"))))]
#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            $crate::log::trace!("mantra: req-id='{}'", $req_id);
        )+
    };
}

#[cfg(all(feature = "defmt", not(any(feature = "log", feature = "stdout"))))]
#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            $crate::defmt::println!("mantra: req-id='{}'", $req_id);
        )+
    };
}

#[cfg(all(feature = "stdout", not(any(feature = "log", feature = "defmt"))))]
#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            println!("mantra: req-id='{}'; file='{}'; line='{}'", $req_id, file!(), line!());
        )+
    };
}

#[cfg(all(feature = "stdout", feature = "log", not(feature = "defmt")))]
#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            $crate::log::trace!("mantra: req-id='{}'", $req_id);
            println!("mantra: req-id='{}'; file='{}'; line='{}'", $req_id, file!(), line!());
        )+
    };
}

#[cfg(not(any(feature = "stdout", feature = "log", feature = "defmt")))]
#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
        )+
    };
}
