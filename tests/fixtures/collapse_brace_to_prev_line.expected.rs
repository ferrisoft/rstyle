fn f() {
    if let Some(x) = foo()
    && let Some(y) = bar() {
        use_both(x, y);
    }
}
