//! Benchmarks for text buffer operations.
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use luminex_buffer::TextBuffer;

/// Generates a large text string for benchmarking.
fn generate_large_text(lines: usize) -> String {
    (0..lines)
        .map(|i| format!("Line {}: This is a sample line of text for benchmarking purposes.\n", i))
        .collect()
}

/// Benchmarks buffer creation.
fn bench_buffer_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_creation");

    for size in [100, 1000, 10000, 100000].iter() {
        let text = generate_large_text(*size);

        group.bench_with_input(
            BenchmarkId::new("from_string", size),
            &text,
            |b, text| {
                b.iter(|| {
                    let buffer = TextBuffer::from(black_box(text.as_str()));
                    black_box(buffer)
                })
            },
        );
    }

    group.finish();
}

/// Benchmarks insertion at various positions.
fn bench_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion");

    let base_text = generate_large_text(10000);

    group.bench_function("insert_at_start", |b| {
        b.iter_with_setup(
            || TextBuffer::from(base_text.as_str()),
            |mut buffer| {
                buffer.insert(0, black_box("inserted text")).unwrap();
                black_box(buffer)
            },
        )
    });

    group.bench_function("insert_at_middle", |b| {
        b.iter_with_setup(
            || TextBuffer::from(base_text.as_str()),
            |mut buffer| {
                let mid = buffer.len_chars() / 2;
                buffer.insert(mid, black_box("inserted text")).unwrap();
                black_box(buffer)
            },
        )
    });

    group.bench_function("insert_at_end", |b| {
        b.iter_with_setup(
            || TextBuffer::from(base_text.as_str()),
            |mut buffer| {
                let end = buffer.len_chars();
                buffer.insert(end, black_box("inserted text")).unwrap();
                black_box(buffer)
            },
        )
    });

    group.finish();
}

/// Benchmarks deletion operations.
fn bench_deletion(c: &mut Criterion) {
    let mut group = c.benchmark_group("deletion");

    let base_text = generate_large_text(10000);

    group.bench_function("delete_at_start", |b| {
        b.iter_with_setup(
            || TextBuffer::from(base_text.as_str()),
            |mut buffer| {
                buffer.delete(0..100).unwrap();
                black_box(buffer)
            },
        )
    });

    group.bench_function("delete_at_middle", |b| {
        b.iter_with_setup(
            || TextBuffer::from(base_text.as_str()),
            |mut buffer| {
                let mid = buffer.len_chars() / 2;
                buffer.delete(mid..mid + 100).unwrap();
                black_box(buffer)
            },
        )
    });

    group.finish();
}

/// Benchmarks undo/redo operations.
fn bench_undo_redo(c: &mut Criterion) {
    let mut group = c.benchmark_group("undo_redo");

    group.bench_function("undo_single", |b| {
        b.iter_with_setup(
            || {
                let mut buffer = TextBuffer::new();
                buffer.insert(0, "test").unwrap();
                buffer
            },
            |mut buffer| {
                buffer.undo().unwrap();
                black_box(buffer)
            },
        )
    });

    group.bench_function("undo_100_operations", |b| {
        b.iter_with_setup(
            || {
                let mut buffer = TextBuffer::new();
                for i in 0..100 {
                    buffer.insert(i * 5, "test ").unwrap();
                    // Add delay to prevent coalescing
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                buffer
            },
            |mut buffer| {
                for _ in 0..100 {
                    buffer.undo().unwrap();
                }
                black_box(buffer)
            },
        )
    });

    group.finish();
}

/// Benchmarks line access.
fn bench_line_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_access");

    let text = generate_large_text(100000);
    let buffer = TextBuffer::from(text.as_str());

    group.bench_function("get_line", |b| {
        b.iter(|| {
            let line = buffer.line(black_box(50000)).unwrap();
            black_box(line)
        })
    });

    group.bench_function("iterate_all_lines", |b| {
        b.iter(|| {
            let mut count = 0;
            for i in 0..buffer.len_lines() {
                let _line = buffer.line(i).unwrap();
                count += 1;
            }
            black_box(count)
        })
    });

    group.finish();
}

/// Benchmarks search operations.
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    let text = generate_large_text(10000);
    let buffer = TextBuffer::from(text.as_str());

    group.bench_function("find_all_occurrences", |b| {
        b.iter(|| {
            let matches = buffer.find_all(black_box("Line"));
            black_box(matches)
        })
    });

    group.bench_function("find_next", |b| {
        b.iter(|| {
            let result = buffer.find_next(black_box("sample"), 0);
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_creation,
    bench_insertion,
    bench_deletion,
    bench_undo_redo,
    bench_line_access,
    bench_search,
);

criterion_main!(benches);
