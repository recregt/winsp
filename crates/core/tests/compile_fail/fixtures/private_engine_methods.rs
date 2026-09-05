fn main() {
    let index = winsp_core::engine::Engine::new();
    let mut out = Vec::new();
    index.find_into("test", 5, &mut out);
}
