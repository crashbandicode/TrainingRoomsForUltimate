//! rc.10 S1 (docs/CODE-REDUCTION-rc10.md): the guard tables' rows live in `data/tables/*.tsv` and `build.rs` turns them
//! into the same Rust tables. These digests were taken from the tables while they were still Rust literals
//! (`f5146184`): every row of every table, in table order, as its `Debug` text. A changed, missing, added or moved
//! row changes a digest.

use crate::model::sha256::sha256_hex;
use std::fmt::Write;

fn digest<T: std::fmt::Debug>(tables: &[&[T]]) -> (usize, String) {
    let (mut text, mut rows) = (String::new(), 0);
    for table in tables {
        for row in *table {
            writeln!(text, "{row:?}").unwrap();
            rows += 1;
        }
        text.push('\n');
    }
    (rows, sha256_hex(text.as_bytes()))
}

#[test]
fn every_seam_row_is_the_row_it_was() {
    use crate::engine::seams::native_seams as s;
    let tables: [&[s::Seam]; 13] = [
        s::rows_a1::ROWS, s::rows_a2::ROWS, s::rows_a3::ROWS, s::rows_a4::ROWS, s::rows_a5::ROWS, s::rows_identity::ROWS,
        s::rows_q15::ROWS, s::rows_lobby_items::ROWS, s::rows_lobby_persist::ROWS, s::rows_cpu_level::ROWS,
        s::rows_res_unload::ROWS, s::rows_game_speed::ROWS, s::rows_lobby_speed::ROWS,
    ];
    assert_eq!(digest(&tables), (SEAM_ROWS, SEAM_DIGEST.to_owned()));
}

#[test]
fn every_site_row_is_the_row_it_was() {
    use crate::model as m;
    let tables: [&[m::sites::Site]; 8] = [
        &m::sites::BASE_SITES, &m::sites_pkg2::SITES_PKG2, &m::sites_pkg3::STATE_SITES, &m::sites_pkg4::SITES,
        &m::sites_pkg6::SITES, &m::sites_hitbox::SITES, &m::sites_u4::SITES, &m::sites_u4b::SITES,
    ];
    assert_eq!(digest(&tables), (SITE_ROWS, SITE_DIGEST.to_owned()));
    let rest = (digest(&[&m::sites_ui::UI_SITES[..]]), digest(&[&m::sites_pkg2::COMMON_SITES[..]]));
    assert_eq!(rest, ((UI_ROWS, UI_DIGEST.to_owned()), (COMMON_ROWS, COMMON_DIGEST.to_owned())));
}

/// Every row of every data file carries a 64-byte guard (the playbook's rule, which its compliance check reads from
/// `guard: [..]` literals in `src/`; the rows are no longer there), and the files hold exactly the tables' rows.
#[test]
fn every_data_row_has_a_64_byte_guard() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/tables");
    let (mut files, mut rows) = (0, 0);
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "tsv") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        let fields: Vec<&str> = lines.next().unwrap().split('\t').collect();
        let guard = fields.iter().position(|f| *f == "guard").unwrap_or_else(|| panic!("{path:?}: no guard field")) - 1;
        for line in lines.filter(|l| !l.starts_with("# ") && !l.is_empty()) {
            let cell = line.split('\t').nth(guard).unwrap();
            assert!(cell.len() == 128 && cell.bytes().all(|b| b.is_ascii_hexdigit()), "{path:?}: {line}");
            rows += 1;
        }
        files += 1;
    }
    assert_eq!((files, rows), (23, SEAM_ROWS + SITE_ROWS + UI_ROWS + COMMON_ROWS));
}

const SEAM_ROWS: usize = 690;
const SEAM_DIGEST: &str = "5846c7ca5a38c59b4f0780b1c791bc2b0014c4a2dd980baf30c7d7de26f26170";
const SITE_ROWS: usize = 258;
const SITE_DIGEST: &str = "f86958e64401e05a6acac006e94b5be53b062f03a5da5bdbf027197436f2ea4d";
const UI_ROWS: usize = 6;
const UI_DIGEST: &str = "35ddb9f164a3ac26ba2e5c45c35066ee738e796589d58e2c697c82e486567b15";
const COMMON_ROWS: usize = 4;
const COMMON_DIGEST: &str = "cf4d2c7c3e7633e8ce303706de7ee67bfc9aff587f30cd6b77f46148117e5b14";
