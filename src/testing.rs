pub fn repo() -> String {
    repo_raw().replace('\\', "/")
}

pub fn repo_raw() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}
