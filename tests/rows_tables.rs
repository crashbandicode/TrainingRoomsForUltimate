use std::process::Command;

#[test]
fn every_generated_table_is_what_its_generator_writes() {
    let root = env!("CARGO_MANIFEST_DIR");
    let elf = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/main-13.0.5.elf");
    if !std::path::Path::new(elf).is_file() {
        assert!(
            std::env::var_os("SKIP_OWNED_ELF_FACT").is_some(),
            "owned 13.0.5 ELF missing at {}: link <stage>/../research, or set SKIP_OWNED_ELF_FACT=1 to skip",
            elf
        );
        println!("SKIPPED every_generated_table_is_what_its_generator_writes: {} absent", elf);
        return;
    }
    let out = Command::new("python3")
        .current_dir(root)
        .args(["-I", "tools/check_rows_tables.py", elf])
        .output()
        .expect("the table check needs python3");
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}
