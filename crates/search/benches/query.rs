use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::index::{Index, Match};
use winsp_core::models::{AppItem, LaunchTarget, SearchResult};

const WORDS: &[&str] = &[
    "Advanced", "Cloud", "Digital", "Media", "System", "File", "Network", "Secure", "Quick",
    "Visual", "Studio", "Editor", "Player", "Manager", "Console",
];

fn synthetic_index(size: usize) -> Index<AppItem> {
    let mut index = Index::new();
    let items: Vec<AppItem> = (0..size)
        .map(|i| {
            let name = format!(
                "{} {} {}",
                WORDS[i % WORDS.len()],
                WORDS[(i / WORDS.len()) % WORDS.len()],
                i
            );
            AppItem::new(
                format!("bench-app-{i}"),
                name,
                LaunchTarget::Path(format!("app{i}.exe")),
            )
        })
        .collect();
    index.set_items(items);
    index
}

fn bench_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");
    let index = synthetic_index(1_000);
    let mut matches: Vec<Match<AppItem>> = Vec::new();
    let mut results: Vec<SearchResult> = Vec::new();

    group.bench_function("app_match", |b| {
        b.iter(|| {
            winsp_search::query(
                &index,
                black_box("visual studio"),
                6,
                &mut matches,
                &mut results,
            );
            black_box(&results);
        });
    });

    group.bench_function("calculation", |b| {
        b.iter(|| {
            winsp_search::query(&index, black_box("12 * 12"), 6, &mut matches, &mut results);
            black_box(&results);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_query);
criterion_main!(benches);
