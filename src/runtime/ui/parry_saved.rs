//! Parry practice's saved practices (`artifacts/rc15-parry/DESIGN.md` §5) on the shared saved-file list (`layouts::SavedList`):
//! `parry/<id>.json` through the `Store`, references only. Use wires the CPU's active macro and Loop and turns Parry practice
//! On; a practice whose macro or start save was deleted is listed unusable and refuses Use.

use std::path::Path;

use super::layouts::SavedList;
use crate::engine::library::LoopCount;
use crate::engine::parry_practice::{check, Practice, Usable, DIR};
use crate::engine::rr_host::{parry_check, parry_use, parry_wiring, PARRY_MACRO_MISSING, PARRY_NO_RECORDING, PARRY_START_SAVE_MISSING};
use crate::model::menu::Menu;
use crate::model::recording_menu::{hot_five, ordered};
use crate::model::store::{lowest_free, Store};

fn store(root: &Path) -> Store<Practice> {
    Store::new(root.join(DIR))
}

fn all(root: &Path) -> (Vec<(String, Practice)>, usize) {
    match root.join(DIR).exists() {
        true => store(root).list().map_or((Vec::new(), 1), |l| (l.entries, l.skipped.len())),
        false => (Vec::new(), 0),
    }
}

/// One more than the folder's highest use: the hot five's order (Break the Targets layouts' convention).
fn next_use(root: &Path) -> u64 {
    all(root).0.iter().map(|e| e.1.used).max().unwrap_or(0) + 1
}

fn refusal(usable: Usable) -> Option<&'static str> {
    match usable {
        Usable::Yes => None,
        Usable::MacroMissing => Some(PARRY_MACRO_MISSING),
        Usable::StartSaveMissing => Some(PARRY_START_SAVE_MISSING),
    }
}

#[derive(Default)]
pub struct ParryPractices {
    rows: Vec<(String, Practice, Usable)>,
}

impl ParryPractices {
    fn practice(&self, id: &str) -> Result<(Practice, Usable), String> {
        self.rows.iter().find(|r| r.0 == id).map(|r| (r.1.clone(), r.2)).ok_or_else(|| format!("No practice {id}"))
    }

    fn write(&self, root: &Path, id: Option<&str>, mut p: Practice, done: String) -> Result<String, String> {
        p.used = next_use(root);
        match id {
            Some(id) => store(root).overwrite(id, &p),
            None => store(root).save_new(&p).map(drop),
        }
        .map(|()| done)
        .map_err(|e| e.to_string())
    }
}

impl SavedList for ParryPractices {
    fn title(&self) -> &'static str {
        "Saved parry practices"
    }

    fn naming(&self) -> &'static str {
        "Name practice"
    }

    fn saved_rows(&mut self, root: &Path) -> (Vec<(String, String, String)>, usize, usize) {
        let (files, skipped) = all(root);
        let hot = hot_five(files.iter().map(|e| (e.0.as_str(), e.1.used)));
        let listed = ordered(&files, &hot, |e| &e.0, |e| &e.1.name).into_iter().cloned();
        self.rows = listed.map(|(id, p)| { let (opens, start) = parry_check(root, &p.macro_id); (id, p, check(opens, start)) }).collect();
        let rows = self.rows.iter().map(|(id, p, u)| {
            let loops = match p.loop_count { LoopCount::Infinite => "∞".to_owned(), LoopCount::Finite(n) => n.to_string() };
            let detail = refusal(*u).map_or_else(|| format!("{} · Loop {loops}", p.macro_id), |why| format!("unusable · {why}"));
            (id.clone(), p.name.clone(), detail)
        });
        (rows.collect(), hot.len(), skipped)
    }

    fn footer(&self, cursor: usize, skipped: usize) -> String {
        let first = self.rows.get(cursor).map_or("No saved practices yet: Save this parry practice saves one", |r| refusal(r.2).unwrap_or(&r.1.name));
        format!("{first}\nA open   B back{}", if skipped > 0 { format!("   {skipped} skipped") } else { String::new() })
    }

    fn saved_rename(&mut self, root: &Path, id: &str, name: &str) -> Result<String, String> {
        let p = Practice { name: name.trim().to_owned(), ..self.practice(id)?.0 };
        let done = format!("Renamed to {}", p.name);
        self.write(root, Some(id), p, done)
    }

    fn saved_copy(&mut self, root: &Path, id: &str) -> Result<String, String> {
        let p = self.practice(id)?.0;
        let name = lowest_free(|n| format!("{} {n}", p.name.trim()), all(root).0.iter().map(|e| e.1.name.as_str()));
        self.write(root, None, Practice { name, ..p }, "Duplicated: the copy is at the top".to_owned())
    }

    fn saved_remove(&mut self, root: &Path, id: &str) -> Result<String, String> {
        let name = self.practice(id)?.0.name;
        store(root).delete(id).map(|()| format!("Deleted {name}")).map_err(|e| e.to_string())
    }

    /// Use: the CPU's active macro and Loop become the practice's, and Parry practice turns On with its set.
    fn saved_use(&mut self, root: &Path, id: &str, menu: Option<&mut Menu>) -> Result<String, String> {
        let (p, usable) = self.practice(id)?;
        if let Some(why) = refusal(usable) {
            return Err(why.to_owned());
        }
        parry_use(root, &p.macro_id, p.loop_count)?;
        if let Some(menu) = menu {
            menu.parry_on(usize::from(p.set));
        }
        let done = format!("Loaded {}: close the menu and press ZR+Up", p.name);
        self.write(root, Some(id), p, done)
    }

    /// Save this parry practice: the CPU's active macro and Loop, and the session row's set.
    fn saved_new(&mut self, root: &Path, name: &str) -> Option<Result<String, String>> {
        let Some((macro_id, loop_count)) = parry_wiring(root) else { return Some(Err(PARRY_NO_RECORDING.to_owned())) };
        let set = if crate::runtime::features::parry::free() { 2 } else { 1 };
        let p = Practice { schema_version: 1, name: name.trim().to_owned(), macro_id, loop_count, set, used: 0 };
        let done = format!("Saved {}", p.name);
        Some(self.write(root, None, p, done))
    }

    fn new_name(&mut self, root: &Path) -> String {
        lowest_free(|n| format!("Parry practice {n}"), all(root).0.iter().map(|e| e.1.name.as_str()))
    }
}
