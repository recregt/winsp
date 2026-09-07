use criterion::{BatchSize, Bencher, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use winsp_core::index::{Index, Match};
use winsp_core::models::{AppItem, LaunchTarget, SearchResult};

const WORDS: &[&str] = &[
    "Advanced", "Cloud", "Digital", "Media", "System", "File", "Network", "Secure", "Quick",
    "Visual", "Studio", "Editor", "Player", "Manager", "Console",
];

/// How many results the launcher window shows, and therefore how many of them
/// a keystroke builds.
const MAX_RESULTS: usize = 6;

/// The number of entries a real Start menu holds, where what a keystroke costs
/// besides scanning the index is a visible share of the search.
const INSTALLED_INDEX_SIZE: usize = 300;

const SCALE_INDEX_SIZE: usize = 1_000;

/// A query that shares no prefix with the measured ones, so the engine cannot
/// narrow the measured search with the match set it kept.
const UNRELATED_QUERY: &str = "zq";

const TYPED_QUERY: &str = "visual studio";
const TYPED_EXPRESSION: &str = "1234*5678";

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
            .with_description(format!("C:\\Program Files\\bench\\app{i}.exe"))
            .with_keywords(vec!["tool".into(), "utility".into()])
        })
        .collect();
    index.set_items(items);
    index
}

/// The buffers the UI keeps alive for the whole session and hands to
/// [`winsp_search::query`] on every keystroke.
#[derive(Default)]
struct Buffers {
    matches: Vec<Match<AppItem>>,
    results: Vec<SearchResult>,
}

impl Buffers {
    fn query(&mut self, index: &Index<AppItem>, input: &str) {
        winsp_search::query(
            index,
            input,
            MAX_RESULTS,
            &mut self.matches,
            &mut self.results,
        );
    }
}

/// Measures `input` as the first keystroke of a session, scanning the whole
/// index. The untimed setup leaves an unrelated query behind, because repeating
/// one query in a loop would otherwise measure the narrowed rescan.
///
/// [`BatchSize::PerIteration`] is what makes that setup do its job under a
/// wall-clock run, and is also what CodSpeed measures, so both runs measure the
/// same keystroke.
fn bench_cold_query(b: &mut Bencher, index: &Index<AppItem>, input: &str) {
    let mut buffers = Buffers::default();
    b.iter_batched_ref(
        || {
            let mut setup = Buffers::default();
            setup.query(index, UNRELATED_QUERY);
            black_box(&setup.results);
            setup
        },
        |_| {
            buffers.query(index, black_box(input));
            black_box(&buffers.results);
        },
        BatchSize::PerIteration,
    );
}

/// Every prefix of `query`, in the order the keystrokes that type it arrive.
fn keystrokes(query: &str) -> Vec<&str> {
    query
        .char_indices()
        .map(|(offset, ch)| &query[..offset + ch.len_utf8()])
        .collect()
}

fn bench_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");
    let index = synthetic_index(SCALE_INDEX_SIZE);

    group.bench_function("app_match", |b| {
        bench_cold_query(b, &index, "visual studio");
    });

    group.bench_function("calculation", |b| {
        bench_cold_query(b, &index, "12 * 12");
    });

    group.bench_function("empty_query", |b| {
        let mut buffers = Buffers::default();
        b.iter(|| {
            buffers.query(&index, black_box(""));
            black_box(&buffers.results);
        });
    });

    // What the launcher actually runs: a query typed one keystroke at a time
    // against an index the size of an installed machine's Start menu, reusing
    // the buffers the UI holds for the session.
    let installed = synthetic_index(INSTALLED_INDEX_SIZE);
    let typed = keystrokes(TYPED_QUERY);
    let expression = keystrokes(TYPED_EXPRESSION);

    group.bench_function("typing_session", |b| {
        let mut buffers = Buffers::default();
        b.iter(|| {
            for input in &typed {
                buffers.query(&installed, input);
                black_box(&buffers.results);
            }
        });
    });

    // The same session for an expression, where every keystroke also builds the
    // calculation the launcher answers with.
    group.bench_function("calculation_session", |b| {
        let mut buffers = Buffers::default();
        b.iter(|| {
            for input in &expression {
                buffers.query(&installed, input);
                black_box(&buffers.results);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_query);
criterion_main!(benches);
