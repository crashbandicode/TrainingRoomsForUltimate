//! Names of modules observed at the game's load site.

pub fn module_name(field: &[u8]) -> Option<&str> {
    let end = field.iter().position(|&byte| byte == 0)?;
    if end == 0 {
        return None;
    }
    core::str::from_utf8(&field[..end]).ok()
}

pub fn is_common(name: &str) -> bool {
    name == "common" || name == "lua2cpp_common" || name.ends_with("lua2cpp_common.nro")
}
