//! rc.10 S1 (docs/CODE-REDUCTION-rc10.md): the guard tables' rows live in `data/tables/*.tsv` and `build.rs` turns them
//! into the same Rust tables. These digests were taken from the tables while they were still Rust literals
//! (`f5146184`): every row of every table, in table order, as its `Debug` text. A changed, missing, added or moved
//! row changes a digest.
//!
//! rc.10 S4 took the 15 seam rows of the deleted probes out (`native-unlink-probe` 8, `automation-results-native` 7): 690
//! rows to 675, and the seam digest with them. The other three digests are still the ones of `f5146184`.
//!
//! rc.13 (a′) proper added `rows_module_pins` (6 rows: a kept group's fighter module pinned across its owner's Reload):
//! 675 seam rows to 681, a 24th data file, and the seam digest with them.
//!
//! rc.14 Hide CPU added 7 rows (the natives that hide a CPU body, group `hide_cpu`) to `rows_cpu_level`: 681 seam rows to 688
//! and the seam digest with them.
//!
//! rc.14 Eggs-ercise added `stop_set_hit_stop_frame` to `sites_hitbox` (the hitlag native, V2.1): 258 site rows to 259,
//! and the site digest with them. V2.2 added camera_limit_range (the stage's camera range): 260.
//!
//! rc.14 spots-2 added `attack_power_mul_status` to `sites_hitbox` (the getter of the module's damage multiplier, the smash charge): 261 site rows
//! to 262 (the count the lane's base held was 261), and the site digest with them. spots-3 added `damage_reaction` (the knockback a
//! defender received): 263; and the Relied `damage_record_layout`, `damage_record_hit_ids` (the record's slot): 265; and the Relied
//! `damage_record_dealt` (the hit's damage past the damage cap): 266.
//!
//! rc.14 wind-articles added the four wind-area commands (`hitbox_area_wind`, `_arg10`, `_rad`, `_rad_arg9`, hooked so an
//! article's wind area is drawn) to `sites_hitbox`: 266 site rows to 270, and the site digest with them.
//!
//! rc.14 custom stages added `rows_custom_stages` (9 rows: the game's custom-stage loader and free, its own load around them,
//! the setter's stage-325 byte): 688 seam rows to 697, a 25th data file, and the seam digest with them. After cs-proof-2 the
//! game's own read joined (queue, begin, cleanup, local wireless's request and cleanup windows, the header and record
//! list reads): 704.

//! rc.15 article hurtboxes adds the approved Called `article_get_from_no` row: 270 site rows to 271.
//! rc.15 Mash adds its separately frozen 17 approved reader rows in a 26th data file. The historical digests stay unchanged.
//! rc.15 spacing grader adds the optional grader rows (five hit-pair stores, two hurt-shape radius stores, the hit-handler hook and its entry, the record contact): 272 site rows to 282.
//! rc.15 Juggler adds seven optional stock attack forecast readers to the Edge/Juggler table: 17 to 24 rows.
//! rc.15 1 Frame adds its 13 rows (`rows_speed_frame`) in a 30th data file. The historical digests stay unchanged.
//! F5 adds five label rows in a 31st data file; the historical digests stay unchanged.
//! Scrub moves (SM1) adds the Called `motion_is_anim_resource` row to `sites_pkg2` (the scrub asks whether a fighter has an
//! animation before it starts it): 282 site rows to 283, and the site digest with them.
//! The slide after a release adds two Called rows to `sites_pkg2` (`motion_set_next_no_comp`, `ground_update_shape`: a
//! posed fighter is let go with no blend and its ground shape is made again): 283 site rows to 285, and the site digest
//! with them.

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
    let tables: [&[s::Seam]; 15] = [
        s::rows_a1::ROWS, s::rows_a2::ROWS, s::rows_a3::ROWS, s::rows_a4::ROWS, s::rows_a5::ROWS, s::rows_identity::ROWS,
        s::rows_q15::ROWS, s::rows_lobby_items::ROWS, s::rows_lobby_persist::ROWS, s::rows_cpu_level::ROWS,
        s::rows_res_unload::ROWS, s::rows_game_speed::ROWS, s::rows_lobby_speed::ROWS, s::rows_module_pins::ROWS,
        s::rows_custom_stages::ROWS,
    ];
    assert_eq!(digest(&tables), (SEAM_ROWS, SEAM_DIGEST.to_owned()));
    assert_eq!(digest(&[s::rows_edge_juggle::ROWS]), (EDGE_JUGGLE_ROWS, EDGE_JUGGLE_DIGEST.to_owned()));
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
/// the guard literals in `src/`; the rows are no longer there), and the files hold exactly the tables' rows.
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
    assert_eq!((files, rows), (32, SEAM_ROWS + EDGE_JUGGLE_ROWS + SITE_ROWS + UI_ROWS + COMMON_ROWS
        + crate::engine::seams::native_seams::rows_edge_zoom::ROWS.len()
        + crate::engine::seams::native_seams::rows_speed_frame::ROWS.len()
        + crate::engine::seams::native_seams::rows_speed_label::ROWS.len()
        + crate::engine::seams::native_seams::rows_cut_file::ROWS.len()
        + crate::model::sites_mashout::SITES.len() + crate::model::sites_lockon::CALLEES.len()));
}

const SEAM_ROWS: usize = 704;
const SEAM_DIGEST: &str = "cb2c75e34e6f3258900a157be9b79d08b7ed396c5500f98d132e7627858d6adc";
const EDGE_JUGGLE_ROWS: usize = 24;
const EDGE_JUGGLE_DIGEST: &str = "e2f841548afadaa39460582c2df3cc5ec6d78090b78c388174d947a5f6478fde";
// The shared overlay sampler adds the guarded stock process_end entry.
const SITE_ROWS: usize = 285;
const SITE_DIGEST: &str = "28319671dd2ac630462742a823d106052cbf078a1da7d4d4235cb8e3328fd70f";
const UI_ROWS: usize = 6;
const UI_DIGEST: &str = "35ddb9f164a3ac26ba2e5c45c35066ee738e796589d58e2c697c82e486567b15";
const COMMON_ROWS: usize = 4;
const COMMON_DIGEST: &str = "cf4d2c7c3e7633e8ce303706de7ee67bfc9aff587f30cd6b77f46148117e5b14";
