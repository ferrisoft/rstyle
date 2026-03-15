macro_rules! m {
    (
        $(#$meta:tt)*
        pub enum $enum:ident { $(
            $variant:ident
            $({ $($field:ident: $field_ty:ty),*$(,)? })?
        ),*$(,)? }
    ) => {}
}
