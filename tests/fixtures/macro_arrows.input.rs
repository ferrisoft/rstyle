macro_rules! m {
    () => {
        fn f() -> i32 {
            match x {
                1 => 2,
            }
        }
    }
}
