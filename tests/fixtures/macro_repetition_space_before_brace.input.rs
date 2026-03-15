macro_rules! m {
    () => {
        pub enum Tag { $($variant),* }
    }
}
