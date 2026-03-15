fn f() {
    fn g() {
        fn h() {
            fn i() {
                fn j() {
                    fn k() {
                        fn l() {
                            fn m() {
                                let long_result = if is_empty { default_value } else { compute_long_alternative_value() };
                            }
                        }
                    }
                }
            }
        }
    }
}
