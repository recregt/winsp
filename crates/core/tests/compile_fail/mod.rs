#[test]
fn encapsulation() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/fixtures/*.rs");
}
