//! Practice-game identity, bounded results and restored-settings precedence. The menu's last On choice wins;
//! snapshots without edit history keep the first enabled row in this registry.

use super::settings::Snapshot;

/// Register each practice game's settings row here once. `stage_tech` is the reserved Stage tech
/// integration key; this reservation creates no settings/menu row. Keep Eggs-ercise before BTT
/// to preserve existing hand-edited-file precedence.
pub const ROWS: &[&str] = &["eggsercise", "btt", "tech_practice", "mashout", "parry_practice", "stage_tech"];

fn enabled(rows: &Snapshot, id: &str) -> bool {
    rows.get(id).is_some_and(|row| row.iter().skip(1).any(|value| *value != 0))
}

/// The effective game, including for directly constructed conflicting snapshots. Missing and
/// Off-only rows do not participate. No allocation: runtime publication can use this.
pub fn active(rows: &Snapshot) -> Option<&'static str> {
    ROWS.iter().copied().find(|id| enabled(rows, id))
}

/// Whether this registered game is the effective winner, rather than merely stored as On.
pub fn is_on(rows: &Snapshot, id: &str) -> bool {
    active(rows) == Some(id)
}

/// Normalize restored settings, preserving the winning option and unrelated rows. Unknown
/// future rows stay untouched until their settings definition exists.
pub fn normalize(rows: &mut Snapshot) {
    let winner = active(rows);
    for &id in ROWS {
        if Some(id) != winner && enabled(rows, id) {
            let mut off = vec![0; rows.get(id).map_or(0, <[u8]>::len)];
            off[0] = 1;
            let _ = rows.set(id, &off);
        }
    }
}

/// A session's minimum elapsed frames for this exact key; vacant slots are separate from zero keys/times.
pub fn best_time<K: Copy + Eq, const N: usize>(pool: &[Option<(K, u32)>; N], key: K) -> Option<u32> {
    pool.iter().flatten().find(|(saved, _)| *saved == key).map(|(_, time)| *time)
}

/// Record a scored attempt. Updates stay in place; a new key evicts the oldest (FIFO, not LRU).
/// The caller owns scoring eligibility. A zero-capacity pool returns the time without retaining it.
pub fn record_time<K: Copy + Eq, const N: usize>(pool: &mut [Option<(K, u32)>; N], key: K, time: u32) -> u32 {
    if let Some((_, best)) = pool.iter_mut().flatten().find(|(saved, _)| *saved == key) {
        *best = (*best).min(time);
        return *best;
    }
    if N > 0 {
        pool.rotate_left(1);
        pool[N - 1] = Some((key, time));
    }
    time
}

/// Advance the result's eligible-frame countdown. Its owner leaves the result phase when this is true.
pub fn result_tick(left: &mut u16, tick: bool) -> bool {
    *left = left.saturating_sub(u16::from(tick));
    *left == 0
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::*;
        use crate::model::{btt, settings::{self, Settings}};

        #[test]
        fn registry_covers_six_games() {
            assert_eq!(ROWS, &["eggsercise", "btt", "tech_practice", "mashout", "parry_practice", "stage_tech"]);
            for (index, id) in ROWS.iter().enumerate() {
                assert!(!ROWS[index + 1..].contains(id));
            }
        }

        #[test]
        fn effective_reads_resolve_direct_snapshot_conflicts() {
            let mut rows = Snapshot::factory();
            assert_eq!(active(&rows), None);
            assert!(!is_on(&rows, "not_a_game"));
            rows.set("btt", &[0, 1]).unwrap();
            assert_eq!(btt::on(&rows), [false, true]);
            rows.set("eggsercise", &[0, 0, 0, 0, 1]).unwrap();
            assert_eq!(active(&rows), Some("eggsercise"));
            assert_eq!(btt::on(&rows), [true, false]);
            assert!(!is_on(&rows, "btt"));
            normalize(&mut rows);
            assert_eq!(rows.get("eggsercise"), Some(&[0, 0, 0, 0, 1][..]));
            assert_eq!(rows.get("btt"), Some(&[1, 0][..]));
            let once = rows.clone();
            normalize(&mut rows);
            assert_eq!(rows, once);
        }

        #[test]
        fn restored_conflicts_follow_registry_order() {
            // Reserved keys join these real parse/serialize probes once their settings row lands.
            let known: Vec<_> = ROWS.iter().copied().filter(|id| settings::def(id).is_some()).collect();
            for (index, &first) in known.iter().enumerate() {
                for &second in &known[index + 1..] {
                    let mut dirty = Settings::factory();
                    for rows in [&mut dirty.menu, &mut dirty.defaults] {
                        rows.set(first, &[0, 1]).unwrap();
                        rows.set(second, &[0, 1]).unwrap();
                    }
                    let loaded = settings::parse(&dirty.to_json());
                    assert!(loaded.issues.is_empty(), "{first}, {second}");
                    for rows in [&loaded.settings.menu, &loaded.settings.defaults] {
                        assert_eq!(active(rows), Some(first));
                        assert!(rows.get(second).unwrap().iter().skip(1).all(|value| *value == 0));
                    }
                    assert_eq!(settings::parse(&loaded.settings.to_json()).settings, loaded.settings);
                }
            }
        }

        #[test]
        fn off_rows_and_unknown_keys_survive_normalization() {
            let mut rows = Snapshot::factory();
            rows.set("eggsercise", &[0, 0, 0, 0, 0]).unwrap();
            rows.set_kept("future_practice_game", serde_json::json!([0, 1])).unwrap();
            let before = rows.clone();
            normalize(&mut rows);
            assert_eq!(rows, before);
            assert_eq!(active(&rows), None);
        }

        #[test]
        fn best_time_pool_keeps_exact_keys_and_fifo() {
            // Keys share their first component; the complete identity must remain distinct.
            let mut pool = [None; 3];
            assert_eq!(best_time(&pool, (0, 0)), None);
            assert_eq!(record_time(&mut pool, (0, 0), 40), 40);
            assert_eq!(record_time(&mut pool, (0, 1), 30), 30);
            assert_eq!(record_time(&mut pool, (0, 2), 20), 20);
            let order = pool.map(|entry| entry.map(|(key, _)| key));
            assert_eq!(record_time(&mut pool, (0, 0), 0), 0);
            assert_eq!(record_time(&mut pool, (0, 0), u32::MAX), 0);
            assert_eq!(pool.map(|entry| entry.map(|(key, _)| key)), order, "updates never refresh eviction order");
            assert_eq!([best_time(&pool, (0, 0)), best_time(&pool, (0, 1)), best_time(&pool, (0, 2))], [Some(0), Some(30), Some(20)]);
            record_time(&mut pool, (0, 3), 10);
            assert_eq!(best_time(&pool, (0, 0)), None, "even an improved key is evicted in insertion order");
            assert_eq!(pool, [Some(((0, 1), 30)), Some(((0, 2), 20)), Some(((0, 3), 10))]);
            let mut empty = [None; 0];
            assert_eq!(record_time(&mut empty, 0u8, 5), 5);
            assert_eq!(best_time(&empty, 0), None);
        }
    }
}
