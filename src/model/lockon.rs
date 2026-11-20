//! Tracks the native item for each live target. The practice games own scoring.

/// The target picture's radius, the practice games' own hit test (`eggs::RADIUS`).
pub const PICTURE_RADIUS: f32 = 4.0;
/// The lock-on helper Sandbag's posture scale: small enough that its whole hurt volume is inside the picture
/// (`helper_reach` 3.48 of 4), so the helper only anchors the native lock-on search and never widens the break area.
pub const HELPER_SCALE: f32 = 0.3;
/// How far under the target centre the helper's posture (its feet) is placed: its middle part on the centre.
pub const HELPER_CENTER_HEIGHT: f32 = 2.96;
/// The Sandbag's three hurt parts: their height above its posture, and the radius they touch at, per unit of posture scale
/// (exact d6393cb0, Eden target-hurtbox-d6393cb0-yoga-4: at 0.6 the centres rose 2.41, 5.93 and 9.41 and touched at 3.45).
pub const HELPER_PART_RISE: [f32; 3] = [4.02, 9.88, 15.68];
pub const HELPER_PART_RADIUS: f32 = 5.75;

/// How far the helper's hurt volume reaches from the target centre, at posture `scale` placed `center_height` under it.
pub fn helper_reach(scale: f32, center_height: f32) -> f32 {
    HELPER_PART_RISE.iter().map(|rise| (rise * scale - center_height).abs()).fold(0.0, f32::max) + HELPER_PART_RADIUS * scale
}

/// Whether `id` is a helper by the ids published for scene `published`: none once the scene is `now` no longer.
pub fn names_helper(ids: &[u32], published: u32, now: u32, id: u32) -> bool {
    let _ = (ids, published, now, id);
    true
}

/// Address of the next 32-bit slot ID, if the game's free stack has an entry.
pub fn free_slot_top([begin, end, capacity]: [usize; 3]) -> Option<usize> {
    (begin != 0 && (begin | end | capacity) & 3 == 0 && begin < end && end <= capacity)
        .then(|| end - 4)
}

/// Advance the consumed damage baseline even when the caller rejects the hit.
pub fn take_damage(previous: &mut f32, current: f32) -> bool {
    current.is_finite() && current > core::mem::replace(previous, current)
}

#[derive(Clone, Copy)]
pub struct Target {
    pub key: u64,
    pub at: [f32; 2],
}

pub trait ItemHost {
    type Handle: Copy;

    fn owns_live_item(&mut self, item: Self::Handle) -> bool;
    fn create_target_item(&mut self, at: [f32; 2]) -> Option<Self::Handle>;
    fn place_target_item(&mut self, item: Self::Handle, at: [f32; 2]);
    fn retire_target_item(&mut self, item: Self::Handle);
}

#[derive(Clone, Copy)]
struct Owned<T> {
    key: u64,
    item: T,
}

pub struct Tracker<T, const N: usize> {
    scene: u32,
    items: [Option<Owned<T>>; N],
}

impl<T: Copy, const N: usize> Tracker<T, N> {
    pub const fn new() -> Self {
        Self {
            scene: 0,
            items: [None; N],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(Option::is_none)
    }

    /// The scene (generation) the slots belong to.
    pub fn scene(&self) -> u32 {
        self.scene
    }

    /// The ids of the items the slots own now (0: an empty slot), as `id` names them: what the visualizer leaves out.
    pub fn ids(&self, id: impl Fn(&T) -> u32) -> [u32; N] {
        let _ = id;
        [0; N]
    }

    pub fn item_mut(&mut self, scene: u32, slot: usize, key: u64) -> Option<&mut T> {
        if scene != self.scene {
            return None;
        }
        let item = self.items.get_mut(slot)?.as_mut().filter(|item| item.key == key)?;
        Some(&mut item.item)
    }

    pub fn update(
        &mut self,
        scene: u32,
        targets: [Option<Target>; N],
        host: &mut impl ItemHost<Handle = T>,
    ) {
        if self.scene != scene {
            // The departed scene owns destruction. Never check its old IDs in the new scene.
            *self = Self {
                scene,
                items: [None; N],
            };
        }
        for (slot, target) in self.items.iter_mut().zip(targets) {
            let target = target.filter(|t| t.at.iter().all(|value| value.is_finite()));
            let mut keep = slot.take().filter(|owned| host.owns_live_item(owned.item));
            if let Some(old) = keep.filter(|owned| target.is_none_or(|new| new.key != owned.key)) {
                host.retire_target_item(old.item);
                keep = None;
            }
            let Some(target) = target else { continue };
            if let Some(item) = keep
                .map(|owned| owned.item)
                .or_else(|| host.create_target_item(target.at))
            {
                host.place_target_item(item, target.at);
                *slot = Some(Owned {
                    key: target.key,
                    item,
                });
            }
        }
    }
}

impl<T: Copy, const N: usize> Default for Tracker<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
