use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;


// ==============================
// === collect_fixture_source ===
// ==============================

fn collect_fixture_source() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("failed to read fixtures dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_string_lossy().ends_with(".input.rs"))
        .collect();
    entries.sort_by_key(|e| e.path());
    let mut source = String::new();
    for entry in entries {
        source.push_str(&fs::read_to_string(entry.path()).expect("failed to read fixture"));
        source.push('\n');
    }
    source
}


// =========================
// === bench_format_all ===
// =========================

fn bench_format_all(c: &mut Criterion) {
    let source = collect_fixture_source();
    let mut group = c.benchmark_group("format_all_fixtures");

    group.bench_function("rstyle", |b| {
        b.iter(|| black_box(rstyle::formatter::format_source(black_box(&source))))
    });

    group.bench_function("rustfmt", |b| {
        b.iter(|| {
            let mut child = Command::new("rustfmt")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("failed to spawn rustfmt");
            child
                .stdin
                .take()
                .expect("failed to open rustfmt stdin")
                .write_all(source.as_bytes())
                .expect("failed to write to rustfmt stdin");
            black_box(child.wait_with_output().expect("rustfmt failed"))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_format_all);
criterion_main!(benches);
