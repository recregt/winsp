fn main() {
    let index = winsp_core::SearchIndex::new();
    let _ = index.find("test", 5);
}
