//! Owned, in-session UI suspension across native scene teardown.
//!
//! Adapters capture values, release their native UI, and reconstruct it later.
//! This layer neither saves feature settings nor confirms provisional edits.
//! The caller supplies a lobby-session identity, never a native object address.
//! There is one outstanding interaction; a snapshot survives a failed restore.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumeTicket {
    session: u64,
    serial: u64,
}

/// Facts from the current native callback, not facts cached before the match.
#[derive(Clone, Copy, Debug)]
pub struct ResumeReadiness {
    pub session: u64,
    pub previous_ui_released: bool,
    pub interruption_resolved: bool,
    pub practice_ready: bool,
    pub another_ui_active: bool,
}

impl ResumeReadiness {
    fn permits(self, ticket: ResumeTicket) -> bool {
        self.session == ticket.session
            && self.previous_ui_released
            && self.interruption_resolved
            && self.practice_ready
            && !self.another_ui_active
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Suspended,
    Restoring,
}

struct Entry<T> {
    ticket: ResumeTicket,
    snapshot: T,
    phase: Phase,
}

/// Stores value snapshots only. The feature adapter validates their contents.
/// No serialization is implied: this is scene continuation in the same process.
pub struct ResumeSlot<T> {
    serial: u64,
    entry: Option<Entry<T>>,
}

impl<T> Default for ResumeSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ResumeSlot<T> {
    pub const fn new() -> Self {
        Self {
            serial: 0,
            entry: None,
        }
    }

    /// Reject a competing snapshot rather than overwrite the unfinished work.
    /// On refusal, return ownership of the supplied value to its caller.
    pub fn suspend(&mut self, session: u64, snapshot: T) -> Result<ResumeTicket, T> {
        if self.entry.is_some() {
            return Err(snapshot);
        }
        let Some(serial) = self.serial.checked_add(1) else {
            return Err(snapshot);
        };
        self.serial = serial;
        let ticket = ResumeTicket { session, serial };
        self.entry = Some(Entry {
            ticket,
            snapshot,
            phase: Phase::Suspended,
        });
        Ok(ticket)
    }

    pub fn ticket(&self) -> Option<ResumeTicket> {
        self.entry.as_ref().map(|entry| entry.ticket)
    }

    /// Begin one attempt only when the adapter has observed an eligible return.
    /// The reference cannot outlive this slot borrow; copy owned values before
    /// invoking native code or crossing a thread boundary.
    pub fn begin_restore(
        &mut self,
        ticket: ResumeTicket,
        ready: ResumeReadiness,
    ) -> Option<&T> {
        let entry = self.entry.as_mut()?;
        if entry.ticket != ticket || entry.phase != Phase::Suspended || !ready.permits(ticket) {
            return None;
        }
        entry.phase = Phase::Restoring;
        Some(&entry.snapshot)
    }

    /// Refine an owned deferred value during its exact active reconstruction.
    /// The adapter must validate the refinement before writing it; this does not
    /// acknowledge restoration or change the ticket/readiness lifecycle.
    pub fn restoring_snapshot_mut(&mut self, ticket: ResumeTicket) -> Option<&mut T> {
        let entry = self.entry.as_mut()?;
        (entry.ticket == ticket && entry.phase == Phase::Restoring).then_some(&mut entry.snapshot)
    }

    /// Keep the checkpoint after a failed reconstruction. The adapter decides
    /// when there is new evidence to retry; this method does not start a loop.
    pub fn restore_failed(&mut self, ticket: ResumeTicket) -> bool {
        let Some(entry) = self.entry.as_mut() else {
            return false;
        };
        if entry.ticket != ticket || entry.phase != Phase::Restoring {
            return false;
        }
        entry.phase = Phase::Suspended;
        true
    }

    /// A later interruption preserves the unacknowledged checkpoint, whether
    /// reconstruction is still running or has already failed. Only its owner
    /// may suspend it again; this never replaces the original pending edits.
    pub fn interrupt_restore(&mut self, ticket: ResumeTicket) -> bool {
        let Some(entry) = self.entry.as_mut() else {
            return false;
        };
        if entry.ticket != ticket {
            return false;
        }
        entry.phase = Phase::Suspended;
        true
    }

    /// Acknowledge only after the requested UI, including pending edits, exists.
    /// This removes the checkpoint; it does not save or confirm those edits.
    pub fn restored(&mut self, ticket: ResumeTicket) -> bool {
        if !self
            .entry
            .as_ref()
            .is_some_and(|entry| entry.ticket == ticket && entry.phase == Phase::Restoring)
        {
            return false;
        }
        self.entry = None;
        true
    }

    /// Explicit cancellation or leaving the owning lobby discards the snapshot.
    /// A stale callback cannot discard a newer interaction.
    pub fn discard(&mut self, ticket: ResumeTicket) -> bool {
        if !self.entry.as_ref().is_some_and(|entry| entry.ticket == ticket) {
            return false;
        }
        self.entry = None;
        true
    }
}

