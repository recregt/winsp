fn main() {
    embed_resource::compile("resources/winsp.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
