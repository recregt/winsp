use criterion::{BatchSize, Bencher, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::engine::Engine;
use winsp_core::models::{AppItem, LaunchTarget};

const WORDS: &[&str] = &[
    "Advanced",
    "Cloud",
    "Digital",
    "Media",
    "System",
    "File",
    "Network",
    "Secure",
    "Quick",
    "Visual",
    "Studio",
    "Editor",
    "Player",
    "Manager",
    "Console",
    "Terminal",
    "Browser",
    "Calculator",
    "Explorer",
    "Monitor",
    "Analyzer",
    "Designer",
    "Recorder",
    "Viewer",
    "Scanner",
    "Launcher",
    "Builder",
    "Compiler",
    "Debugger",
    "Inspector",
];

fn synthetic_index(size: usize) -> Engine {
    let mut index = Engine::new();
    let items: Vec<AppItem> = (0..size)
        .map(|i| {
            let name = format!(
                "{} {} {}",
                WORDS[i % WORDS.len()],
                WORDS[(i / WORDS.len()) % WORDS.len()],
                i
            );
            let id = format!("bench-app-{i}");
            AppItem::new(id, name, LaunchTarget::Path(format!("app{i}.exe")))
                .with_keywords(vec!["tool".into(), "utility".into()])
        })
        .collect();
    index.set_items(items);
    index
}

/// A query that shares no prefix with the measured ones, so the engine cannot
/// narrow the measured search with the match set it kept.
const UNRELATED_QUERY: &str = "zq";

/// Measures `query` as the first keystroke of a session, scanning the whole
/// index. The untimed setup leaves an unrelated query behind, because repeating
/// one query in a loop would otherwise measure the narrowed rescan.
fn bench_cold_query(b: &mut Bencher, index: &Engine, query: &str) {
    b.iter_batched(
        || {
            black_box(index.search(UNRELATED_QUERY, 6));
        },
        |()| index.search(query, 6),
        BatchSize::SmallInput,
    );
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_at_scale");

    for &size in &[100usize, 1_000, 10_000, 50_000] {
        let index = synthetic_index(size);

        group.bench_with_input(BenchmarkId::new("empty_query", size), &index, |b, idx| {
            b.iter(|| idx.search("", 6));
        });

        group.bench_with_input(BenchmarkId::new("prefix_match", size), &index, |b, idx| {
            bench_cold_query(b, idx, "Visual Studio");
        });

        group.bench_with_input(BenchmarkId::new("acronym_match", size), &index, |b, idx| {
            bench_cold_query(b, idx, "as");
        });

        group.bench_with_input(
            BenchmarkId::new("guaranteed_no_match", size),
            &index,
            |b, idx| {
                bench_cold_query(b, idx, "jj");
            },
        );
    }

    group.finish();
}

const TYPED_QUERY: &str = "visual studio";
const TYPING_INDEX_SIZE: usize = 10_000;
/// The number of entries a real Start menu holds, where the per-keystroke
/// fixed costs are a visible share of a search instead of rounding error.
const INSTALLED_INDEX_SIZE: usize = 300;

fn bench_search_while_typing(c: &mut Criterion) {
    let index = synthetic_index(TYPING_INDEX_SIZE);
    let prefixes: Vec<&str> = TYPED_QUERY
        .char_indices()
        .map(|(offset, ch)| &TYPED_QUERY[..offset + ch.len_utf8()])
        .collect();

    let mut group = c.benchmark_group("search_while_typing");

    // A keystroke starts from the result of the previous one, so each query is
    // measured after its own predecessor is searched, outside the measurement.
    for (nth, prefix) in prefixes.iter().enumerate() {
        let previous = if nth == 0 {
            UNRELATED_QUERY
        } else {
            prefixes[nth - 1]
        };
        group.bench_with_input(
            BenchmarkId::new("keystroke", prefix),
            prefix,
            |b, prefix| {
                b.iter_batched(
                    || {
                        black_box(index.search(previous, 6));
                    },
                    |()| index.search(prefix, 6),
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.bench_function("full_session", |b| {
        b.iter(|| {
            for prefix in &prefixes {
                black_box(index.search(prefix, 6));
            }
        });
    });

    // The UI keeps a single result buffer alive for the whole session and hands
    // it to `search_into` on every keystroke. Paired with `full_session`, this
    // is what makes the per-keystroke allocation visible.
    group.bench_function("full_session_reused_buffer", |b| {
        let mut results = Vec::new();
        b.iter(|| {
            for prefix in &prefixes {
                index.search_into(prefix, 6, &mut results);
                black_box(&results);
            }
        });
    });

    // A mistyped query: the user types it, then deletes it one keystroke at a
    // time, so the buffer has to shrink and grow again within one session.
    let edit_session = edit_session_queries(&prefixes);

    group.bench_function("edit_session", |b| {
        b.iter(|| {
            for query in &edit_session {
                black_box(index.search(query, 6));
            }
        });
    });

    group.bench_function("edit_session_reused_buffer", |b| {
        let mut results = Vec::new();
        b.iter(|| {
            for query in &edit_session {
                index.search_into(query, 6, &mut results);
                black_box(&results);
            }
        });
    });

    // The same session against an index the size of an installed machine's
    // Start menu, where the scan no longer hides what a keystroke costs
    // besides matching.
    let installed = synthetic_index(INSTALLED_INDEX_SIZE);

    group.bench_function("installed_index_session", |b| {
        b.iter(|| {
            for query in &edit_session {
                black_box(installed.search(query, 6));
            }
        });
    });

    group.bench_function("installed_index_session_reused_buffer", |b| {
        let mut results = Vec::new();
        b.iter(|| {
            for query in &edit_session {
                installed.search_into(query, 6, &mut results);
                black_box(&results);
            }
        });
    });

    group.finish();
}

/// Types the query out one keystroke at a time, then backspaces it away. The
/// empty query at the end is the only one the UI still searches after a
/// deletion, since further backspaces on an empty query are no-ops.
fn edit_session_queries<'a>(prefixes: &[&'a str]) -> Vec<&'a str> {
    let mut queries = prefixes.to_vec();
    queries.extend(prefixes.iter().rev().skip(1).copied());
    queries.push("");
    queries
}

criterion_group!(benches, bench_search, bench_search_while_typing);
criterion_main!(benches);
