#![cfg_attr(not(feature = "std"), no_std)]

pub use mantra_rust_procm::req;
pub use mantra_rust_procm::reqcov;

#[cfg(feature = "extract")]
pub mod extract;

#[inline]
#[allow(unused)]
pub fn req_print(req: ReqCovStatic) {
    #[cfg(feature = "log")]
    log::trace!("{}", req);

    #[cfg(feature = "defmt")]
    defmt::println!("{}", req);

    #[cfg(feature = "stdout")]
    println!("{}", req);
}

#[macro_export]
macro_rules! mr_reqcov {
    ($($req_id:literal),+) => {
        $(
            $crate::req_print($crate::ReqCovStatic{id: $req_id, file: file!(), line: line!()});
        )+
    };
}

#[doc(hidden)]
pub struct ReqCovStatic {
    pub id: &'static str,
    pub file: &'static str,
    pub line: u32,
}

#[cfg(feature = "defmt")]
impl defmt::Format for ReqCovStatic {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "mantra: req-id='{=str}'; file='{=str}'; line='{}';",
            self.id,
            self.file,
            self.line
        )
    }
}

impl core::fmt::Display for ReqCovStatic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "mantra: req-id='{}'; file='{}'; line='{}';",
            self.id, self.file, self.line
        )
    }
}
