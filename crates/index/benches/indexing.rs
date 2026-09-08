// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_index::{Index, IndexableItem};

const WORDS: &[&str] = &[
    "Advanced", "Cloud", "Digital", "Media", "System", "File", "Network", "Secure", "Quick",
    "Visual",
];

/// Minimal stand-in for a real launcher item, so this bench exercises only
/// what [`Index`] itself needs from one.
#[derive(Clone)]
struct BenchItem {
    name: String,
    keywords: Vec<String>,
}

impl IndexableItem for BenchItem {
    fn name(&self) -> &str {
        &self.name
    }

    fn keywords(&self) -> &[String] {
        &self.keywords
    }

    fn launch_count(&self) -> u32 {
        0
    }
}

fn synthetic_items(size: usize) -> Vec<BenchItem> {
    (0..size)
        .map(|i| {
            let name = format!(
                "{} {} {}",
                WORDS[i % WORDS.len()],
                WORDS[(i / 7) % WORDS.len()],
                i
            );
            BenchItem {
                name,
                keywords: vec!["tool".into(), "utility".into()],
            }
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
                    let mut engine = Index::new();
                    engine.set_items(items);
                    black_box(engine.len())
                },
                criterion::BatchSize::LargeInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("rescan", size), &items, |b, items| {
            let mut engine = Index::new();
            engine.set_items(items.clone());
            b.iter_batched(
                || items.clone(),
                |items| {
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
                    let mut engine = Index::new();
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
