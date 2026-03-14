fn f() {
    let result = items
        .flat_map(|n| {
            vec![n, n + 1]
                .into_iter()
                .filter(|x| x > &0)
        })
        .enumerate()
        .collect::<Vec<_>>();
}
