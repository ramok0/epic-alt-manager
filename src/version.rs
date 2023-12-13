

pub fn get_program_version() -> &'static str {
    const CURRENT_VERSION_STR: &str = env!("CARGO_PKG_VERSION");
    CURRENT_VERSION_STR
}