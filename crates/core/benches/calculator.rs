use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::models::{AppItem, LaunchTarget};
use winsp_core::search::Engine;

const EXPRESSIONS: &[(&str, &str)] = &[
    ("simple_arithmetic", "128 * 4"),
    ("precedence_chain", "2 + 3 * 4 - 10 / 5 + 7 % 3"),
    ("nested_parentheses", "((12 + 4) * (3 - 1)) / ((2 + 2) * 2)"),
    ("functions", "sqrt(144) + log10(1000) + abs(0 - 7)"),
    ("constants_and_power", "pi * e ^ 2"),
    ("implicit_multiplication", "3(4 + 5)2"),
    ("invalid_expression", "notafunction(12) +"),
];

const POPULATED_INDEX_SIZE: usize = 10_000;

fn populated_engine(size: usize) -> Engine {
    let mut engine = Engine::new();
    let items: Vec<AppItem> = (0..size)
        .map(|i| {
            let id = format!("bench-app-{i}");
            let name = format!("Bench App {i}");
            AppItem::new(id, name, LaunchTarget::Path(format!("app{i}.exe")))
        })
        .collect();
    engine.set_items(items);
    engine
}

fn bench_calculator(c: &mut Criterion) {
    let engine = Engine::new();
    let mut group = c.benchmark_group("calculator");

    for (name, expression) in EXPRESSIONS {
        group.bench_function(*name, |b| {
            b.iter(|| black_box(engine.search(black_box(expression), 6)));
        });
    }

    group.finish();
}

fn bench_calculator_with_populated_index(c: &mut Criterion) {
    let engine = populated_engine(POPULATED_INDEX_SIZE);
    let mut group = c.benchmark_group("calculator_with_populated_index");

    for (name, expression) in EXPRESSIONS {
        group.bench_function(*name, |b| {
            b.iter(|| black_box(engine.search(black_box(expression), 6)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_calculator,
    bench_calculator_with_populated_index
);
criterion_main!(benches);
