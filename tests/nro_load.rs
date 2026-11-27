use crate::common;
use crate::model::{nro_load, sites, sites_pkg2};
use crate::pause_helper_sites as owned_elf;

#[test]
fn nro_load_module_name_reads_to_the_first_nul() {
    assert_eq!(nro_load::module_name(b"common\0\0\0"), Some("common"));
    assert_eq!(nro_load::module_name(b"item\0"), Some("item"));
    assert_eq!(nro_load::module_name(b"item\0junk\0"), Some("item"));
    assert_eq!(nro_load::module_name(&[b'x'; 256]), None);
    assert_eq!(nro_load::module_name(b"\0"), None);
    assert_eq!(nro_load::module_name(b"\xff\0"), None);
    for name in ["common", "lua2cpp_common", "x/lua2cpp_common.nro"] {
        assert!(nro_load::is_common(name), "{name}");
    }
    for name in ["item", "commonx", ""] {
        assert!(!nro_load::is_common(name), "{name}");
    }
}

#[test]
fn common_install_does_not_write_the_event_file_on_the_loader_thread() {
    // Switch std's metadata/open construct an owned CString even on a successful append.
    let mut graph = crate::callgraph::Graph::scan(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
    graph.precise_imports = true;
    let [install] = graph.find("cpu_common::install")[..] else { panic!("missing common installer") };
    let [writer] = graph.find("install::write_text")[..] else { panic!("missing event writer") };
    assert!(!graph.callees(install).contains(&writer), "common install calls the allocating synchronous event writer");
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn import_at_slot(bytes: &[u8], slot: u64) -> &str {
    let shoff = u64_at(bytes, 0x28) as usize;
    let entsize = u16_at(bytes, 0x3a) as usize;
    let count = u16_at(bytes, 0x3c) as usize;
    let sections: Vec<_> = (0..count).map(|i| {
        let h = shoff + i * entsize;
        (u32_at(bytes, h + 4), u64_at(bytes, h + 0x10), u64_at(bytes, h + 0x18), u64_at(bytes, h + 0x20))
    }).collect();
    let file_offset = |address: u64| {
        let (_, va, off, _) = sections.iter()
            .find(|(kind, va, _, size)| *kind != 8 && *va <= address && address - *va < *size)
            .unwrap_or_else(|| panic!("no file-backed section for {address:#x}"));
        (off + address - va) as usize
    };
    let mod0 = u32_at(bytes, file_offset(4)) as u64;
    assert_eq!(&bytes[file_offset(mod0)..file_offset(mod0) + 4], b"MOD0");
    let dynamic = mod0.wrapping_add_signed(u32_at(bytes, file_offset(mod0 + 4)) as i32 as i64);
    let mut tags = std::collections::BTreeMap::new();
    let mut cursor = file_offset(dynamic);
    loop {
        let tag = u64_at(bytes, cursor);
        let value = u64_at(bytes, cursor + 8);
        cursor += 16;
        if tag == 0 { break; }
        tags.insert(tag, value);
    }
    let (strtab, symtab, jmprel, pltrelsz) = (tags[&5], tags[&6], tags[&23], tags[&2]);
    for i in 0..pltrelsz / 24 {
        let entry = file_offset(jmprel + i * 24);
        if u64_at(bytes, entry) != slot { continue; }
        let index = u64_at(bytes, entry + 8) >> 32;
        let name_offset = u32_at(bytes, file_offset(symtab + index * 24)) as u64;
        let start = file_offset(strtab + name_offset);
        let end = start + bytes[start..].iter().position(|&byte| byte == 0).unwrap();
        return std::str::from_utf8(&bytes[start..end]).unwrap();
    }
    panic!("no PLT relocation for {slot:#x}");
}

#[test]
fn nro_load_site_follows_the_games_only_load_module_call() {
    let Some(owned) = common::owned_main("nro_load_site_follows_the_games_only_load_module_call") else { return };
    let site = sites_pkg2::NRO_LOADED_OFFSET as u64;
    let row = sites::find("nro_loaded").unwrap();
    assert_eq!(row.feature, sites::Feature::NroLoad);
    assert_eq!(row.offset, site as usize);
    assert_eq!(row.guard, owned.window(site as usize));
    assert!(!sites::feature_usable(sites::Feature::NroLoad, |s| s.name != "nro_loaded"));
    assert!(sites::feature_usable(sites::Feature::NroLoad, |s| s.name != "cpu_on_frame"));

    let word = |at| owned_elf::word(&owned.bytes, at);
    assert_eq!(word(site), 0x9104cb28);
    assert_eq!(word(site - 0x2c), 0xaa1903e0);
    let cbz = word(site - 0x24);
    assert_eq!(cbz & 0xff00001f, 0x34000000);
    assert_eq!(owned_elf::target(site - 0x24, cbz), Some(site));
    let bl = word(site - 0x28);
    assert_eq!(bl & 0xfc000000, 0x94000000);
    let stub = owned_elf::target(site - 0x28, bl).unwrap();
    let (adrp, ldr, add, br) = (word(stub), word(stub + 4), word(stub + 8), word(stub + 12));
    assert_eq!(adrp & 0x9f00001f, 0x90000010);
    assert_eq!(ldr & 0xffc003ff, 0xf9400211);
    assert_eq!(add & 0xffc003ff, 0x91000210);
    assert_eq!(br, 0xd61f0220);
    let imm = (((adrp >> 5) & 0x7ffff) << 2) | ((adrp >> 29) & 3);
    let page_delta = ((imm << 11) as i32 >> 11) as i64 * 4096;
    let slot = (stub & !0xfff).wrapping_add_signed(page_delta) + (((ldr >> 10) & 0xfff) as u64 * 8);
    assert_eq!(import_at_slot(&owned.bytes, slot), "_ZN2nn2ro10LoadModuleEPNS0_6ModuleEPKvPvmi");

    let mut stub_branches = Vec::new();
    let mut site_branches = Vec::new();
    for (i, instruction) in owned.text_words().into_iter().enumerate() {
        let pc = owned.text.addr as u64 + i as u64 * 4;
        if let Some(target) = owned_elf::target(pc, instruction) {
            if target == site { site_branches.push(pc); }
            if target == stub && instruction & 0x7c000000 == 0x14000000 {
                stub_branches.push(pc);
            }
        }
    }
    assert_eq!(stub_branches, [site - 0x28]);
    assert_eq!(site_branches, [site - 0x24]);
}
