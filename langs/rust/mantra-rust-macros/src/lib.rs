// pub use mantra_rust_procm::req;
// pub use mantra_rust_procm::req_link;
// pub use mantra_rust_procm::req_note;
// pub use mantra_rust_procm::req_satisfied;
// pub use mantra_rust_procm::req_test;
// pub use mantra_rust_procm::req_verified;

pub use mantra_rust_procm::*;

#[macro_export]
macro_rules! satisfy_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[macro_export]
macro_rules! impl_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[macro_export]
macro_rules! verify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[macro_export]
macro_rules! clarify_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

#[macro_export]
macro_rules! link_req {
    ($($id:literal),+ => $code:expr) => {
        {
            $(const _: &str = $id;)+
            $code
        }
    };
}

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

#[cfg(test)]
mod tests {

    #[test]
    fn single_expr_macros() {
        let res = satisfy_req!("ID-1" => true);
        assert!(res);
        let res = satisfy_req!("ID-1", "ID-2" => 1 + 1 == 2);
        assert!(res);

        let res = impl_req!("ID-1" => true);
        assert!(res);
        let res = impl_req!("ID-1", "ID-2" => 1 + 1 == 2);
        assert!(res);

        let res = verify_req!("ID-1" => true);
        assert!(res);
        let res = verify_req!("ID-1", "ID-2" => 1 + 1 == 2);
        assert!(res);

        let res = clarify_req!("ID-1" => true);
        assert!(res);
        let res = clarify_req!("ID-1", "ID-2" => 1 + 1 == 2);
        assert!(res);

        let res = link_req!("ID-1" => true);
        assert!(res);
        let res = link_req!("ID-1", "ID-2" => 1 + 1 == 2);
        assert!(res);
    }

    #[test]
    #[should_panic]
    fn assert_macro() {
        assert_req!("ID-1" => true);
        assert_req!("ID-1" => false);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn assert_macro_with_static_msg() {
        assert_req!("ID-1" => true, "Passing");
        assert_req!("ID-1" => false, "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn assert_macro_with_param_msg() {
        assert_req!("ID-1" => true, "Passing {}: {}", "value", 1);
        assert_req!("ID-1" => false, "Failing assert {}: {}", "value", 0);
    }

    #[test]
    #[should_panic]
    fn debug_assert_macro() {
        debug_assert_req!("ID-1" => true);
        debug_assert_req!("ID-1" => false);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn debug_assert_macro_with_static_msg() {
        debug_assert_req!("ID-1" => true, "Passing");
        debug_assert_req!("ID-1" => false, "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn debug_assert_macro_with_param_msg() {
        debug_assert_req!("ID-1" => true, "Passing {}: {}", "value", 1);
        debug_assert_req!("ID-1" => false, "Failing assert {}: {}", "value", 0);
    }

    #[test]
    #[should_panic]
    fn assert_eq_macro() {
        assert_eq_req!("ID-1" => 1, 1);
        assert_eq_req!("ID-1" => 0, 1);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn assert_eq_macro_with_static_msg() {
        assert_eq_req!("ID-1" => 'a', 'a', "Passing");
        assert_eq_req!("ID-1" => 'a', 'b', "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn assert_eq_macro_with_param_msg() {
        assert_eq_req!("ID-1" => "hello", "hello", "Passing {}: {}", "value", 1);
        assert_eq_req!("ID-1" => "hello", "world", "Failing assert {}: {}", "value", 0);
    }

    #[test]
    #[should_panic]
    fn debug_assert_eq_macro() {
        debug_assert_eq_req!("ID-1" => 1, 1);
        debug_assert_eq_req!("ID-1" => 0, 1);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn debug_assert_eq_macro_with_static_msg() {
        debug_assert_eq_req!("ID-1" => 'a', 'a', "Passing");
        debug_assert_eq_req!("ID-1" => 'a', 'b', "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn debug_assert_eq_macro_with_param_msg() {
        debug_assert_eq_req!("ID-1" => "hello", "hello", "Passing {}: {}", "value", 1);
        debug_assert_eq_req!("ID-1" => "hello", "world", "Failing assert {}: {}", "value", 0);
    }

    #[test]
    #[should_panic]
    fn assert_ne_macro() {
        assert_ne_req!("ID-1" => 1, 0);
        assert_ne_req!("ID-1" => 1, 1);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn assert_ne_macro_with_static_msg() {
        assert_ne_req!("ID-1" => 'a', 'b', "Passing");
        assert_ne_req!("ID-1" => 'a', 'a', "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn assert_ne_macro_with_param_msg() {
        assert_ne_req!("ID-1" => "hello", "world", "Passing {}: {}", "value", 1);
        assert_ne_req!("ID-1" => "hello", "hello", "Failing assert {}: {}", "value", 0);
    }

    #[test]
    #[should_panic]
    fn debug_assert_ne_macro() {
        debug_assert_ne_req!("ID-1" => 1, 0);
        debug_assert_ne_req!("ID-1" => 1, 1);
    }

    #[test]
    #[should_panic(expected = "Failing assert")]
    fn debug_assert_ne_macro_with_static_msg() {
        debug_assert_ne_req!("ID-1" => 'a', 'b', "Passing");
        debug_assert_ne_req!("ID-1" => 'a', 'a', "Failing assert");
    }

    #[test]
    #[should_panic(expected = "Failing assert value: 0")]
    fn debug_assert_ne_macro_with_param_msg() {
        debug_assert_ne_req!("ID-1" => "hello", "world", "Passing {}: {}", "value", 1);
        debug_assert_ne_req!("ID-1" => "hello", "hello", "Failing assert {}: {}", "value", 0);
    }
}
