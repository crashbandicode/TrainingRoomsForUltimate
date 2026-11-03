//! Value-only admission for diagnostic completion observations.
//! Reset on a new segment-zero record; no capture-policy state is changed.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Event { Stop, Publish, Serializer, ActiveCopy, Segment, State, Sample, Input }

impl Event {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stop => "stop", Self::Publish => "publish",
            Self::Serializer => "serializer", Self::ActiveCopy => "active_copy",
            Self::Segment => "segment", Self::State => "state",
            Self::Sample => "sample", Self::Input => "input",
        }
    }
}

#[derive(Default)]
pub struct CompletionTrace { record: Option<(u64, u32)>, seen: u128 }
impl CompletionTrace {
    pub const fn new() -> Self { Self { record: None, seen: 0 } }
    pub fn clear(&mut self) { self.record = None; self.seen = 0; }
    pub fn begin(&mut self, generation: u64, index: u32) {
        self.record = Some((generation, index)); self.seen = 0;
    }
    pub const fn record(&self) -> Option<(u64, u32)> { self.record }
    /// At most 8 kinds * 9 entry labels per record, regardless of callback count.
    /// Invalid entries are refused without consuming another entry's observation.
    pub fn take(&mut self, event: Event, entry: Option<u8>) -> bool {
        if self.record.is_none() || entry.is_some_and(|value| value >= 8) { return false; }
        let bit = 1u128 << (event as u32 * 9 + u32::from(entry.unwrap_or(8)));
        if self.seen & bit != 0 { return false; }
        self.seen |= bit; true
    }
}
