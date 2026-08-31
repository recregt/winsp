use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::models::{AppItem, AppTarget};
use winsp_core::search::Engine;

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
            AppItem::new(id, name, AppTarget::Path(format!("app{i}.exe")))
                .with_keywords(vec!["tool".into(), "utility".into()])
        })
        .collect();
    index.set_items(items);
    index
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_at_scale");

    for &size in &[100usize, 1_000, 10_000, 50_000] {
        let index = synthetic_index(size);

        group.bench_with_input(BenchmarkId::new("empty_query", size), &index, |b, idx| {
            b.iter(|| idx.search("", 6));
        });

        group.bench_with_input(BenchmarkId::new("prefix_match", size), &index, |b, idx| {
            b.iter(|| idx.search("Visual Studio", 6));
        });

        group.bench_with_input(BenchmarkId::new("acronym_match", size), &index, |b, idx| {
            b.iter(|| idx.search("as", 6));
        });

        group.bench_with_input(
            BenchmarkId::new("guaranteed_no_match", size),
            &index,
            |b, idx| {
                b.iter(|| idx.search("jj", 6));
            },
        );
    }

    group.finish();
}

const TYPED_QUERY: &str = "visual studio";
const TYPING_INDEX_SIZE: usize = 10_000;

fn bench_search_while_typing(c: &mut Criterion) {
    let index = synthetic_index(TYPING_INDEX_SIZE);
    let prefixes: Vec<&str> = TYPED_QUERY
        .char_indices()
        .map(|(offset, ch)| &TYPED_QUERY[..offset + ch.len_utf8()])
        .collect();

    let mut group = c.benchmark_group("search_while_typing");

    for prefix in &prefixes {
        group.bench_with_input(
            BenchmarkId::new("keystroke", prefix),
            prefix,
            |b, prefix| {
                b.iter(|| index.search(prefix, 6));
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

    group.finish();
}

criterion_group!(benches, bench_search, bench_search_while_typing);
criterion_main!(benches);
