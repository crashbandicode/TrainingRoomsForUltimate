//! The names of the modules TrainingRooms sees at the game's load site (`runtime::nro_load`).

/// The name in a loader `Module`'s fixed name field: the bytes before the first NUL, as UTF-8. `None` for an empty, unterminated or non-UTF-8 field. Reads in place: no allocation, so the load hook can call it.
pub fn module_name(field: &[u8]) -> Option<&str> {
    let end = field.iter().position(|&byte| byte == 0)?;
    if end == 0 {
        return None;
    }
    core::str::from_utf8(&field[..end]).ok()
}

/// Whether a loaded module is the `common` script module (`lua2cpp_common.nro`), by the names the loader gives it.
pub fn is_common(name: &str) -> bool {
    name == "common" || name == "lua2cpp_common" || name.ends_with("lua2cpp_common.nro")
}
