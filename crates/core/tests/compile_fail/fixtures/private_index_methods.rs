fn main() {
    let index = winsp_core::index::Index::<winsp_core::models::AppItem>::new();
    let mut out = Vec::new();
    index.find_into("test", 5, &mut out);
}
