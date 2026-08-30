fn main() {
    let index = winsp_core::search::Engine::new();
    let _ = index.find("test", 5);
}
