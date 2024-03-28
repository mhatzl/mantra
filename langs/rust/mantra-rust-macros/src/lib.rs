#![cfg_attr(not(feature = "std"), no_std)]

use coverage::ReqCov;
pub use mantra_rust_procm::req;

pub mod coverage;

#[inline]
pub fn req_print(req: ReqCov) {
    #[cfg(feature = "log")]
    log::trace!("{}", req);

    #[cfg(feature = "defmt")]
    defmt::println!("{}", req);

    #[cfg(feature = "stdout")]
    println!("{}", req);
}

#[macro_export]
macro_rules! reqcov {
    ($($req_id:literal),+) => {
        $(
            $crate::req_print($crate::coverage::ReqCov{id: $req_id, file: file!(), line: line!()});
        )+
    };
}
