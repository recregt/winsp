use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::calc;

const EXPRESSIONS: &[(&str, &str)] = &[
    ("simple_arithmetic", "128 * 4"),
    ("precedence_chain", "2 + 3 * 4 - 10 / 5 + 7 % 3"),
    ("nested_parentheses", "((12 + 4) * (3 - 1)) / ((2 + 2) * 2)"),
    ("functions", "sqrt(144) + log10(1000) + abs(0 - 7)"),
    ("constants_and_power", "pi * e ^ 2"),
    ("implicit_multiplication", "3(4 + 5)2"),
    ("invalid_expression", "notafunction(12) +"),
    ("plain_text_query", "visual studio code"),
    ("single_word_query", "chrome"),
];

fn bench_calculator(c: &mut Criterion) {
    let mut group = c.benchmark_group("calculator");

    for (name, expression) in EXPRESSIONS {
        group.bench_function(*name, |b| {
            b.iter(|| black_box(calc::eval(black_box(expression))));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_calculator);
criterion_main!(benches);
