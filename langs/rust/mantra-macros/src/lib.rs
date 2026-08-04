#![cfg_attr(not(feature = "extract"), no_std)]

pub use mantra_procm::*;

pub mod coverage;

#[cfg(feature = "coverage")]
#[doc(hidden)]
#[macro_export]
macro_rules! _line_coverage {
    () => {
        $crate::coverage::LineCoverage::new(line!(), file!()).log();
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! satisfy_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! satisfy_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! impl_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! impl_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! verify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! verify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! clarify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! clarify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! link_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! link_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            $code
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! assert_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            core::assert!($code)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::assert!($code, $msg)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::assert!($code, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! assert_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert!($code)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert!($code, $msg)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert!($code, $msg, $($param),+)
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! debug_assert_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert!($code)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert!($code, $msg)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert!($code, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! debug_assert_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert!($code)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert!($code, $msg)
        }
    };

    ($($id:literal),+ => $code:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert!($code, $msg, $($param),+)
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! assert_eq_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $(const _: &str = $id;)+
            core::assert_eq!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::assert_eq!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::assert_eq!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! assert_eq_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_eq!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_eq!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_eq!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! debug_assert_eq_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! debug_assert_eq_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_eq!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! assert_ne_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $(const _: &str = $id;)+
            core::assert_ne!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::assert_ne!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::assert_ne!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! assert_ne_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_ne!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_ne!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::assert_ne!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(not(feature = "coverage"))]
#[macro_export]
macro_rules! debug_assert_ne_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right, $msg, $($param),+)
        }
    };
}

#[cfg(feature = "coverage")]
#[macro_export]
macro_rules! debug_assert_ne_req {
    ($($id:literal),+ => $left:expr, $right:expr) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right, $msg)
        }
    };

    ($($id:literal),+ => $left:expr, $right:expr, $msg:literal, $($param:expr),+$(,)?) => {
        {
            $crate::_line_coverage!();

            $(const _: &str = $id;)+
            core::debug_assert_ne!($left, $right, $msg, $($param),+)
        }
    };
}
