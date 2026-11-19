//! Tracks the native item for each live target. The practice games own scoring.

#[derive(Clone, Copy)]
pub struct Target {
    pub key: u64,
    pub at: [f32; 2],
}

pub trait ItemHost {
    type Handle: Copy;

    fn valid(&mut self, item: Self::Handle) -> bool;
    fn create(&mut self, at: [f32; 2]) -> Option<Self::Handle>;
    fn place(&mut self, item: Self::Handle, at: [f32; 2]);
    fn remove(&mut self, item: Self::Handle);
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
            let mut keep = slot.take().filter(|owned| host.valid(owned.item));
            if let Some(old) = keep.filter(|owned| target.is_none_or(|new| new.key != owned.key)) {
                host.remove(old.item);
                keep = None;
            }
            let Some(target) = target else { continue };
            if let Some(item) = keep
                .map(|owned| owned.item)
                .or_else(|| host.create(target.at))
            {
                host.place(item, target.at);
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
