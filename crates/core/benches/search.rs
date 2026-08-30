use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use winsp_core::{AppItem, AppTarget, SearchIndex, search};

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

fn synthetic_index(size: usize) -> SearchIndex {
    let mut index = SearchIndex::new();
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
            b.iter(|| search(idx, "", 6));
        });

        group.bench_with_input(BenchmarkId::new("prefix_match", size), &index, |b, idx| {
            b.iter(|| search(idx, "Visual Studio", 6));
        });

        group.bench_with_input(BenchmarkId::new("acronym_match", size), &index, |b, idx| {
            b.iter(|| search(idx, "as", 6));
        });

        group.bench_with_input(
            BenchmarkId::new("guaranteed_no_match", size),
            &index,
            |b, idx| {
                b.iter(|| search(idx, "jj", 6));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
