//! Hitlag read-only native calls, pinned to owned 13.0.5. See S0-hitlag-resolver/native-audit.
use super::{guard::SeamKind, sites::{Feature, Site}};
#[path = "../runtime/features/tech_hitlag_guards.rs"]
pub mod guards;
pub const PAIR_OFFSET: usize = 0x3bd350;
pub const FLOAT_OFFSET: usize = 0x3a4a30;
pub const INT_OFFSET: usize = 0x3bce20;
pub const K_OFFSET: usize = 0x3caf60;
const fn sites() -> [Site; 5] {
    let mut result = [Site { name: "", feature: Feature::TechPractice, offset: 0,
        kind: SeamKind::Called, guard: [0;64], note: "Live typed hitlag reader" }; 5];
    let mut i=0;
    while i<result.len() {
        let (name,offset,guard)=guards::HITLAG_GUARDS[i];
        result[i]=Site { name, feature: Feature::TechPractice, offset,
            kind: if i==4 {SeamKind::Relied} else {SeamKind::Called}, guard,
            note: "Audited live object/pair/slot identity" };
        i+=1;
    }
    result
}
pub const SITES: [Site;5] = sites();
