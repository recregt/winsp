use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::models::{AppItem, LaunchTarget};
use winsp_core::search::Engine;

const WORDS: &[&str] = &[
    "Advanced", "Cloud", "Digital", "Media", "System", "File", "Network", "Secure", "Quick",
    "Visual",
];

fn synthetic_items(size: usize) -> Vec<AppItem> {
    (0..size)
        .map(|i| {
            let name = format!(
                "{} {} {}",
                WORDS[i % WORDS.len()],
                WORDS[(i / 7) % WORDS.len()],
                i
            );
            AppItem::new(
                format!("bench-app-{i}"),
                name,
                LaunchTarget::Path(format!("app{i}.exe")),
            )
            .with_keywords(vec!["tool".into(), "Utility".into()])
        })
        .collect()
}

fn bench_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");

    for &size in &[1_000usize, 10_000] {
        let items = synthetic_items(size);

        group.bench_with_input(BenchmarkId::new("set_items", size), &items, |b, items| {
            b.iter_batched(
                || items.clone(),
                |items| {
                    let mut engine = Engine::new();
                    engine.set_items(items);
                    black_box(engine.len())
                },
                criterion::BatchSize::LargeInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("add_item", size), &items, |b, items| {
            b.iter_batched(
                || items.clone(),
                |items| {
                    let mut engine = Engine::new();
                    for item in items {
                        engine.add_item(item);
                    }
                    black_box(engine.len())
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_indexing);
criterion_main!(benches);
