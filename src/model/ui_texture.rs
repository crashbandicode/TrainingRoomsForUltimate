//! The pure half of the run-time texture helper (`runtime/ui/texture.rs`, stream u4): the layout facts of the game's
//! UI texture object and its shared pointer that the helper reads, the readiness test, and a fixed-size queue that delays a
//! texture's release by a few frames after it was unbound.
//!
//! How the game shows a game-file texture on a picture pane (owned 13.0.5 `main`, `docs/provenance` rows of
//! `sites_u4.rs`): a file's hash40 gives a resource handle (`ui_tex_find_handle`), `ui_tex_make` wraps the handle in a
//! 0x68-byte UI texture held by a `shared_ptr` and queues the file for loading, `ui_tex_poll` turns the loaded BNTX into a
//! GPU texture view with a registered descriptor slot, and the texture's embedded texture-info (`+0x38`) is what a picture
//! pane's TexMap takes (`ui_tex_bind`). The file is read by the game's own resource service, so ARCropolis's replacement
//! file is what loads (SPEC-core-07 U1-U4: the game's own loader is the route; the plugin never reads or decodes a BNTX).

/// The resource handle value that means "none" (the lookup writes it when the path hash is not in the resource table).
pub const HANDLE_NONE: u32 = 0x00ff_ffff;

/// `main`-relative offsets of the data the helper compares pointers against (the game's own vtable values).
pub const HOLDER_VTABLE: usize = 0x524_14c0;
pub const CONTROL_VTABLE: usize = 0x523_dba0;
/// The file-resource service's global slot (`adrp`/`ldr` words of `ui_tex_service_slot`).
pub const SERVICE_SLOT: usize = 0x533_2f20;

/// The shared pointer's holder object: `{ vtable, ui_texture* }`.
pub const HOLDER_TEXTURE: usize = 8;
/// The control block: `{ vtable, shared_owners - 1, weak_owners, ... }`.
pub const CONTROL_COUNT: usize = 8;
/// The control block's vtable slot of `on_zero_shared` (the game calls `[[control] + 0x10](control)`).
pub const CONTROL_ON_ZERO: usize = 0x10;

/// The UI texture: handle, pending-job pointer, the texture-info the TexMap takes, and the job-state object.
pub const TEXTURE_HANDLE: usize = 0x00;
pub const TEXTURE_JOB: usize = 0x18;
pub const TEXTURE_DESCRIPTOR: usize = 0x38;
/// The embedded texture-info's 8-byte descriptor slot (written by the view registration), its width and height (`u16` each) and the byte that says
/// the slot is registered (the game's destroy releases the slot only then).
pub const TEXTURE_SLOT: usize = 0x40;
pub const TEXTURE_WIDTH: usize = 0x48;
pub const TEXTURE_HEIGHT: usize = 0x4a;
pub const TEXTURE_REGISTERED: usize = 0x5c;
pub const SLOT_UNSET: u64 = u64::MAX;
pub const TEXTURE_STATE: usize = 0x60;
/// `[[tex+0x60]+0x18]`, the state value that means the texture finished.
pub const STATE_FIELD: usize = 0x18;
pub const STATE_LOADED: u32 = 3;

/// What `ui_tex_poll` left of one UI texture (the fields the game's own bind reads).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureFacts {
    pub handle: u32,
    /// `[tex+0x18]`: a pending decode job, 0 when none.
    pub job: u64,
    /// `[tex+0x60]` non-null.
    pub has_state: bool,
    /// `[[tex+0x60]+0x18]`, meaningful only when `has_state`.
    pub state: u32,
}

/// The game's own test (`ui_tex_ready_words`): the texture may be bound when no job is pending, the file handle was consumed
/// (the loaded bytes were copied and the handle released) and the state object, if any, says finished.
pub fn is_ready(facts: TextureFacts) -> bool {
    facts.job == 0 && facts.handle == HANDLE_NONE && (!facts.has_state || facts.state == STATE_LOADED)
}

/// Releases a texture only after `delay` frames: the GPU may still read the last frame's descriptor for a frame or two after
/// the pane stopped using it. A fixed array: the helper pushes and pops in the UI tick and never allocates.
pub struct RetireQueue<T: Copy, const N: usize> {
    slots: [Option<(T, u64)>; N],
    delay: u64,
}

impl<T: Copy, const N: usize> RetireQueue<T, N> {
    pub const fn new(delay: u64) -> Self {
        RetireQueue { slots: [None; N], delay }
    }

    /// Queue `item` at frame `now`; false when the queue is full (the caller keeps the item).
    pub fn push(&mut self, item: T, now: u64) -> bool {
        match self.slots.iter_mut().find(|s| s.is_none()) {
            Some(slot) => {
                *slot = Some((item, now.saturating_add(self.delay)));
                true
            }
            None => false,
        }
    }

    /// The next item whose delay has passed at frame `now`, removed from the queue.
    pub fn pop_due(&mut self, now: u64) -> Option<T> {
        let slot = self.slots.iter_mut().find(|s| matches!(s, Some((_, due)) if *due <= now))?;
        slot.take().map(|(item, _)| item)
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> TextureFacts {
        TextureFacts { handle: HANDLE_NONE, job: 0, has_state: false, state: 0 }
    }

    #[test]
    fn a_texture_is_ready_only_when_the_handle_was_consumed_and_no_job_is_pending() {
        assert!(is_ready(facts()));
        assert!(!is_ready(TextureFacts { handle: 0x1234, ..facts() }), "the file is still loading");
        assert!(!is_ready(TextureFacts { job: 0x7000_0000, ..facts() }), "a decode job is pending");
    }

    #[test]
    fn the_state_object_must_say_finished_when_there_is_one() {
        assert!(is_ready(TextureFacts { has_state: true, state: STATE_LOADED, ..facts() }));
        assert!(!is_ready(TextureFacts { has_state: true, state: 2, ..facts() }));
        assert!(is_ready(TextureFacts { has_state: false, state: 2, ..facts() }), "state is not read without the object");
    }

    #[test]
    fn the_queue_releases_after_the_delay_in_any_order() {
        let mut q: RetireQueue<u32, 4> = RetireQueue::new(8);
        assert!(q.push(1, 10));
        assert!(q.push(2, 12));
        assert_eq!(q.pop_due(17), None);
        assert_eq!(q.pop_due(18), Some(1));
        assert_eq!(q.pop_due(18), None);
        assert_eq!(q.pop_due(30), Some(2));
        assert!(q.is_empty());
    }

    #[test]
    fn a_full_queue_refuses_and_keeps_its_items() {
        let mut q: RetireQueue<u8, 2> = RetireQueue::new(1);
        assert!(q.push(1, 0));
        assert!(q.push(2, 0));
        assert!(!q.push(3, 0));
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop_due(1), Some(1));
        assert!(q.push(3, 1));
    }
}
