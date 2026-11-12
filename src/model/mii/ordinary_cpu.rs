//! Ordinary CPU1's native selector assignment. Not a free-slot allocator.
use {crate::engine::mii::lifetime::{Binding, Configuration}, crate::engine::mii::ordinary_player::Owner};

pub const OWNER: usize = 1;
pub const DESTINATION: u16 = 1;

/// Exact native CPU descriptor receipts qualify all three classes. Runtime adapter acceptance is separate.
pub fn supported_kind(kind: u32) -> bool { matches!(kind, 0x48..=0x4a) }

/// Shared fresh-versus-retained decision for every present row.
/// `Ok(false)` keeps the retained path. `Ok(true)` is the generic fresh-O
/// constructor, which reads entry id `FE+0xe0` in `0..8` and that row's own
/// descriptor. Training, LDN, and Quickplay use `layout.owners()` (0/1/2/3 or
/// 0/4/5/6). Match modes never become a layout. Brawler, Swordfighter and
/// Gunner are not special-cased here.
pub fn headgear_reload(layout:crate::engine::mii::backend::Layout,owner:usize,before:u16,target:u16)
    ->Result<bool,&'static str> {
    if before==target {return Ok(false);}
    if target==u16::MAX {return Err("invalid headgear row");}
    if owner>=8 || !layout.owners().contains(&owner) {
        return Err("headgear row is not on this layout");
    }
    Ok(true)
}

/// CPU wrapper around [`headgear_reload`] for the layout's physical CPU.
/// Training passes owner 1. Waiting passes owner 4. Any other owner keeps the
/// historical refusal and does not borrow the constant Training slot.
pub fn cpu_hat_route(layout:crate::engine::mii::backend::Layout,owner:usize,before:u16,target:u16)->Result<bool,&'static str> {
    if owner!=layout.visible()[1] as usize {return Err("CPU unchanged hat required");}
    match headgear_reload(layout,owner,before,target) {
        Ok(changed)=>Ok(changed),
        Err(_) if target==u16::MAX=>Err("CPU invalid headgear row"),
        Err(reason)=>Err(reason),
    }
}

/// A changed headgear row cannot reuse the retained O's kind*42+3 resource key.
/// This is branch selection only; SameClass still validates all of its old proofs.
pub fn retained_same_class_candidate(before:Configuration,after:Configuration)->bool {
    before.canonical().class==after.canonical().class && before.tokens().hat==after.tokens().hat
}

/// Copied phase facts, not native write authority. The shared publisher additionally
/// holds G and proves the executing ticket, protected union, task and source registry.
#[derive(Clone, Copy)]
pub struct HatPublication<'a> {
    pub layout:crate::engine::mii::backend::Layout,
    pub owner:u32,
    pub source:bool,
    pub opt_in:bool,
    pub initialized:bool,
    pub fingerprints:bool,
    pub before:u16,
    pub target:u16,
    /// Selector record `Binding.record_index` actually written by publication.
    /// Saved payloads must publish this row. It is not the active `E+0x1a8`.
    pub destination_index:u16,
    /// Immutable origin `E+0x1a8` captured with the owner. Origin restore
    /// publishes this row and does not copy CharInfo onto the selector record.
    pub original_index:u16,
    pub descriptor:&'a [u8;0x70],
    pub payload:Option<Configuration>,
}
impl HatPublication<'_> {
    pub fn validate(&self)->Result<(),&'static str> {
        // Preserve the equal-hat contract for every existing publisher.
        if self.before==self.target && !self.opt_in {return Ok(());}
        // A saved payload publishes the selector record that `record_address`
        // writes. An empty payload restores the immutable origin index. The
        // active snapshot index is neither value once a saved row is live.
        // A source token is only the helper lease.
        let helper=self.layout.helper(self.owner as usize).is_some();
        let index=u16::from_le_bytes(self.descriptor[0x46..0x48].try_into().unwrap());
        let fresh=headgear_reload(self.layout,self.owner as usize,self.before,self.target)?;
        if self.owner>=8 || !self.opt_in || self.initialized || !self.fingerprints
            || (self.source && !helper) || !fresh {
            return Err("unqualified headgear publication");
        }
        if let Some(c)=self.payload {
            if !supported_kind(c.canonical().class as u32) {return Err("Mii class not qualified");}
        }
        if u16::from_le_bytes(self.descriptor[0x4a..0x4c].try_into().unwrap())!=self.target {
            return Err("headgear descriptor/scalar mismatch");
        }
        if let Some(c)=self.payload {
            if c.tokens().hat!=self.target
                || u32::from_le_bytes(self.descriptor[0x18..0x1c].try_into().unwrap())!=c.canonical().class as u32
                || index!=self.destination_index
                || self.destination_index!=self.owner as u16 {
                return Err("headgear payload/row mismatch");
            }
        } else if index!=self.original_index {
            return Err("headgear retained origin index mismatch");
        }
        // None restores the retained origin record: the adapter supplies
        // origin scalars and an origin-based descriptor, not a fabricated payload.
        Ok(())
    }

    /// [`HatPublication::validate`] for a Mii one-member group switch
    /// (MII-GROUPS-DESIGN §5.2). The shared resource owner is the live window's
    /// and therefore initialized, so the fresh-O requirement is replaced by the
    /// group's own proof: the group's prefetch registered the hat key
    /// (`kind*42+3`, built from `R+0x70`) for `group_hat`, which must be the hat
    /// this publication names. A saved payload is mandatory (a Mii group always
    /// carries a configuration); every other check of `validate` applies.
    pub fn validate_bank(&self,group_hat:u16)->Result<(),&'static str> {
        let Some(c)=self.payload else {return Err("Mii group publication without a payload");};
        if group_hat!=self.target || c.tokens().hat!=self.target {
            return Err("Mii group hat differs from the published hat");
        }
        if self.before==self.target && !self.opt_in {return Ok(());}
        let helper=self.layout.helper(self.owner as usize).is_some();
        let index=u16::from_le_bytes(self.descriptor[0x46..0x48].try_into().unwrap());
        let fresh=headgear_reload(self.layout,self.owner as usize,self.before,self.target)?;
        if self.owner>=8 || !self.opt_in || !self.fingerprints || (self.source && !helper) || !fresh {
            return Err("unqualified headgear publication");
        }
        if !supported_kind(c.canonical().class as u32) {return Err("Mii class not qualified");}
        if u16::from_le_bytes(self.descriptor[0x4a..0x4c].try_into().unwrap())!=self.target {
            return Err("headgear descriptor/scalar mismatch");
        }
        if u32::from_le_bytes(self.descriptor[0x18..0x1c].try_into().unwrap())!=c.canonical().class as u32
            || index!=self.destination_index || self.destination_index!=self.owner as u16 {
            return Err("headgear payload/row mismatch");
        }
        Ok(())
    }
}

// Native FE+37 mirrors the AI rank setter, whose supported domain is 0..=100.
// This byte is live native state; the captured origin remains immutable.
const NATIVE_RANK: usize = 0x37;

pub fn native_rank(descriptor:&[u8;0x70])->Result<u8,&'static str> {
    let rank=descriptor[NATIVE_RANK];
    if rank>100 {return Err("CPU native rank outside supported range");}
    Ok(rank)
}

/// Construct an owned descriptor value carrying the qualified current rank.
/// This grants no native access, recapture permission or transaction authority.
pub fn with_native_rank(mut original:[u8;0x70],current:&[u8;0x70])->Result<[u8;0x70],&'static str> {
    native_rank(&original)?;
    original[NATIVE_RANK]=native_rank(current)?;
    Ok(original)
}

// FE+0x35 is the AI control type paired with rank. `main+0x6b82b0` stores its
// argument at actor AI+0xc08, then indexes a 16-byte behavior row only when the
// sign-extended byte is <= 0x27. The live FE writers call that setter and then
// store the same byte: `main+0x151be88`, `main+0x151c4b4`, and `main+0x151c520`.
// Fighter control select `main+0x64fc68` reads FE+0x35 into that setter, then
// reads FE+0x37 into the rank setter `main+0x6b8400`. An unchanged byte keeps
// any value. A change is qualified only inside 0..=0x27.
const NATIVE_CONTROL: usize = 0x35;
const NATIVE_CONTROL_MAX: u8 = 0x27;

pub fn native_control(descriptor:&[u8;0x70])->Result<u8,&'static str> {
    let control=descriptor[NATIVE_CONTROL];
    if control>NATIVE_CONTROL_MAX {return Err("CPU native control type outside supported range");}
    Ok(control)
}

fn copy_changed_control(expected:&[u8;0x70],current:&[u8;0x70],descriptor:&mut [u8;0x70])
    ->Result<(),&'static str> {
    if expected[NATIVE_CONTROL]==current[NATIVE_CONTROL] {return Ok(());}
    native_control(expected)?;
    descriptor[NATIVE_CONTROL]=native_control(current)?;
    Ok(())
}

// Native fighter control select `main+0x64fb70` also copies FE+0x34 into the
// controller's operation byte (`main+0x64fc1c`, controller+0x70; 1 takes the CPU
// path at `main+0x64fc40`) and FE+0x36 into AI+0xc10 (`main+0x64fc9c`). Neither
// indexes a native table. With FE+0x35 and FE+0x37 they are the CPU's live
// behavior (Training stop, walk, jump, attack, level or Control), never Mii
// identity; the generic roster loader carries all four unchanged into its install.
const NATIVE_OPERATION: usize = 0x34;
const NATIVE_AI_C10: usize = 0x36;

// FE+0x10..0x14 is the fighter's controller index, a sign-extended 32-bit word.
// The native Mii preview configuration shares this descriptor prefix (0x43 flag,
// 0x46 index, 0x48/0x4a/0x4c/0x4e tokens) and its native writer stores the selected
// controller there, mapping "none" (-1) to the native fallback 8
// (`src/mii_editor_bridge/native.rs`, SelectType path). Native pad lookups of such
// a word sign-extend it and refuse anything not below 8 before indexing the
// eight-slot pad table at `[0x52b94f8]` (`main+0x64f590`, `main+0x65ae14`:
// `ldrsw ..,#0x10; cmp #8; b.hs`). The CPU origin carries -1. Training
// Behavior=Control assigns a pad to the CPU: C104 runtime (cycle-regression172
// #336..#342) changed FE+0x10 ffffffff>00000001 together with operation 01>00,
// type 01>24 and rank 100>21 while index, scalars, records, owner and binding
// stayed equal. It is live behavior, never Mii identity; the generic roster loader
// carries it unchanged into its install. An unchanged word keeps any value. A
// change is qualified only when both sides are inside -1..=8.
const NATIVE_CONTROLLER: core::ops::Range<usize> = 0x10..0x14;
const NATIVE_CONTROLLER_NONE: i32 = -1;
const NATIVE_CONTROLLER_MAX: i32 = 8;

pub fn native_controller(descriptor:&[u8;0x70])->Result<i32,&'static str> {
    let controller=i32::from_le_bytes(descriptor[NATIVE_CONTROLLER].try_into().unwrap());
    if !(NATIVE_CONTROLLER_NONE..=NATIVE_CONTROLLER_MAX).contains(&controller) {
        return Err("CPU native controller outside supported range");
    }
    Ok(controller)
}

fn copy_changed_controller(expected:&[u8;0x70],current:&[u8;0x70],descriptor:&mut [u8;0x70])
    ->Result<(),&'static str> {
    if expected[NATIVE_CONTROLLER]==current[NATIVE_CONTROLLER] {return Ok(());}
    native_controller(expected)?;
    let controller=native_controller(current)?;
    descriptor[NATIVE_CONTROLLER].copy_from_slice(&controller.to_le_bytes());
    Ok(())
}

/// The qualified AI control type and controller assignment plus the live
/// operation and AI+0xc10 bytes.
fn copy_live_control(expected:&[u8;0x70],current:&[u8;0x70],descriptor:&mut [u8;0x70])
    ->Result<(),&'static str> {
    copy_changed_control(expected,current,descriptor)?;
    copy_changed_controller(expected,current,descriptor)?;
    descriptor[NATIVE_OPERATION]=current[NATIVE_OPERATION];
    descriptor[NATIVE_AI_C10]=current[NATIVE_AI_C10];
    Ok(())
}

/// Origin value plus the CPU's live behavior: the qualified current rank, control
/// type and controller assignment, and the current operation and AI+0xc10 bytes.
/// Kind, tokens, records and index stay the origin's.
pub fn with_native_live_metadata(original:[u8;0x70],current:&[u8;0x70])->Result<[u8;0x70],&'static str> {
    let mut descriptor=with_native_rank(original,current)?;
    copy_live_control(&original,current,&mut descriptor)?;
    Ok(descriptor)
}

/// Descriptor rule of a bank group receipt whose entry, owner, epoch, shared
/// resource header and bodies are proven separately: `current` may differ from
/// `expected` only in the five live behavior bytes above (controller word
/// FE+0x10..0x14, operation FE+0x34, control type FE+0x35, AI+0xc10 FE+0x36 and
/// rank FE+0x37), each changed inside its native domain. Native Training CPU
/// Behavior (Stop, Attack, level, Control) writes exactly those bytes on the live
/// entry (C104 #336..#342: ffffffff>00000001, 01>00, 01>24, 100>21). Every other
/// byte, including the group words, the 0x43 flag, the index and the tokens,
/// must match exactly. An unchanged byte keeps any value, so the rule is
/// role-neutral and grants nothing beyond the receipt it qualifies.
pub fn live_behavior_only_change(expected:&[u8;0x70],current:&[u8;0x70])->bool {
    let mut descriptor=*expected;
    descriptor[NATIVE_OPERATION]=current[NATIVE_OPERATION];
    descriptor[NATIVE_AI_C10]=current[NATIVE_AI_C10];
    if expected[NATIVE_RANK]!=current[NATIVE_RANK] {
        if native_rank(expected).is_err() || native_rank(current).is_err() {return false;}
        descriptor[NATIVE_RANK]=current[NATIVE_RANK];
    }
    copy_changed_control(expected,current,&mut descriptor).is_ok()
        && copy_changed_controller(expected,current,&mut descriptor).is_ok()
        && descriptor==*current
}

/// Idle menu/preflight recapture's descriptor-only policy. The native adapter must still run
/// every non-descriptor ownership check and recheck state/provenance at commit.
/// Transactions continue comparing their frozen descriptors byte-for-byte.
/// Only the live behavior bytes are adopted: the controller word FE+0x10..0x14
/// and the control type FE+0x35 when changed inside their native domains, rank
/// FE+0x37 inside its domain, operation FE+0x34 and AI+0xc10 FE+0x36 as written.
pub fn recapture_rank(layout:crate::engine::mii::backend::Layout,expected:Binding,current:Binding,
    expected_descriptor:&[u8;0x70],current_descriptor:&[u8;0x70])->Result<[u8;0x70],&'static str> {
    if expected!=current || current.epoch==0 || current.owner!=layout.visible()[1] {
        return Err("CPU rank recapture owner/lifetime changed");
    }
    let mut descriptor=with_native_rank(*expected_descriptor,current_descriptor)?;
    copy_live_control(expected_descriptor,current_descriptor,&mut descriptor)
        .map_err(|_| "CPU rank recapture changed non-rank descriptor bytes")?;
    if descriptor!=*current_descriptor {return Err("CPU rank recapture changed non-rank descriptor bytes");}
    Ok(descriptor)
}

/// Preflight refresh before a ticket exists. Rank must already match. Only a
/// qualified FE+0x35 change is copied. Any other byte, including rank and the
/// controller word, refuses.
pub fn refresh_control_type(layout:crate::engine::mii::backend::Layout,expected:Binding,current:Binding,
    expected_descriptor:&[u8;0x70],current_descriptor:&[u8;0x70])->Result<[u8;0x70],&'static str> {
    if expected!=current || current.epoch==0 || current.owner!=layout.visible()[1] {
        return Err("CPU control refresh owner/lifetime changed");
    }
    let mut descriptor=*expected_descriptor;
    copy_changed_control(expected_descriptor,current_descriptor,&mut descriptor)?;
    if descriptor!=*current_descriptor {return Err("CPU control refresh changed other descriptor bytes");}
    Ok(descriptor)
}

/// Offsets inside the 0x70 descriptor, excluding native rank, where the frozen
/// and live bytes differ. Unused tail pairs stay zero. This grants no recapture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonRankDiff {
    pub count: u8,
    pub pairs: [(u8, u8, u8); 0x70],
}
pub fn non_rank_descriptor_diff(expected: &[u8; 0x70], current: &[u8; 0x70]) -> NonRankDiff {
    let mut diff = NonRankDiff { count: 0, pairs: [(0, 0, 0); 0x70] };
    for offset in 0..0x70 {
        if offset == NATIVE_RANK || expected[offset] == current[offset] { continue; }
        diff.pairs[diff.count as usize] = (offset as u8, expected[offset], current[offset]);
        diff.count += 1;
    }
    diff
}

/// Field flags sampled beside a refused non-rank descriptor. Indices are the
/// captured and live E+1a8 values, not a physical-slot assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecaptureFlags {
    pub index_eq: bool,
    pub scalars_eq: bool,
    pub selector_record_eq: bool,
    pub current_record_eq: bool,
    pub captured_index: u16,
    pub live_index: u16,
}

/// Same owner epoch and the same byte tuple plus flags have already been logged.
pub fn recapture_observation_seen(
    seen: Option<(u64, NonRankDiff, RecaptureFlags)>,
    epoch: u64,
    diff: NonRankDiff,
    flags: RecaptureFlags,
) -> bool {
    seen == Some((epoch, diff, flags))
}

/// One stack line fits every non-rank pair. `None` means `out` was too small.
pub const RECAPTURE_DIAGNOSTIC_BYTES: usize = 1400;
pub fn write_recapture_diagnostic(out: &mut [u8], diff: &NonRankDiff, flags: &RecaptureFlags) -> Option<usize> {
    fn push(out: &mut [u8], n: &mut usize, bytes: &[u8]) -> bool {
        if *n + bytes.len() > out.len() { return false; }
        out[*n..*n + bytes.len()].copy_from_slice(bytes);
        *n += bytes.len();
        true
    }
    fn push_str(out: &mut [u8], n: &mut usize, text: &str) -> bool { push(out, n, text.as_bytes()) }
    fn push_dec(out: &mut [u8], n: &mut usize, mut value: u16) -> bool {
        let mut buf = [0u8; 5];
        let mut i = 5;
        loop {
            i -= 1;
            buf[i] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 { break; }
        }
        push(out, n, &buf[i..])
    }
    fn push_hex(out: &mut [u8], n: &mut usize, value: u8) -> bool {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        push(out, n, &[HEX[(value >> 4) as usize], HEX[(value & 0xf) as usize]])
    }
    fn push_flag(out: &mut [u8], n: &mut usize, label: &str, value: bool) -> bool {
        push_str(out, n, label) && push(out, n, if value { b"1" } else { b"0" })
    }
    let mut n = 0usize;
    if !push_str(out, &mut n, "recapture non-rank n=")
        || !push_dec(out, &mut n, u16::from(diff.count))
        || !push_flag(out, &mut n, " index_eq=", flags.index_eq)
        || !push_flag(out, &mut n, " scalars_eq=", flags.scalars_eq)
        || !push_flag(out, &mut n, " selector_record_eq=", flags.selector_record_eq)
        || !push_flag(out, &mut n, " current_record_eq=", flags.current_record_eq)
        || !push_str(out, &mut n, " captured_index=")
        || !push_dec(out, &mut n, flags.captured_index)
        || !push_str(out, &mut n, " live_index=")
        || !push_dec(out, &mut n, flags.live_index)
    {
        return None;
    }
    for pair in diff.pairs.iter().take(usize::from(diff.count)) {
        if !push(out, &mut n, b" ")
            || !push_hex(out, &mut n, pair.0)
            || !push(out, &mut n, b":")
            || !push_hex(out, &mut n, pair.1)
            || !push(out, &mut n, b">")
            || !push_hex(out, &mut n, pair.2)
        {
            return None;
        }
    }
    Some(n)
}

/// Frozen native target; an ordinary numbered fighter is not the immutable origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind { Original, Fighter(u32), Saved }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {kind:TargetKind,configuration:Option<Configuration>}
impl Target {
    pub const ORIGINAL:Self=Self {kind:TargetKind::Original,configuration:None};
    pub fn fighter(kind:u32)->Self {Self {kind:TargetKind::Fighter(kind),configuration:None}}
    pub fn mii(c:Configuration)->Self {Self {kind:TargetKind::Saved,configuration:Some(c)}}
    pub fn kind(self)->TargetKind {self.kind}
    pub fn saved(self)->Option<Configuration> {self.configuration}
    /// Commit identity is independent of the payload published for an Original return.
    pub fn publish(self, original:Option<Configuration>)->Option<Configuration> {
        if self.kind==TargetKind::Original {original} else {self.configuration}
    }
    pub fn index(self)->u16 {if self.configuration.is_some() {DESTINATION} else {0}}
}
/// Prepared before INSTALL. The owner commits this value only after Gate::can_complete
/// and the full current native comparison; a refused ticket cannot advance the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Destination {pub target:Target,pub cursor:Option<usize>}

/// Epoch-bound same-class menu intent. CPU original identity remains outside SavedRole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {selection:Option<MenuSelection>,replay:Option<crate::engine::mii::replay::Selection>,receipt_only:bool,direct:Option<(u64,u32)>,refresh_partners:bool}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MenuSelection {epoch:u64,active:crate::engine::mii::inventory::Canonical,choice:crate::engine::mii::selection::Choice}
impl Request {
    pub const CYCLE:Self=Self {selection:None,replay:None,receipt_only:false,direct:None,refresh_partners:false};
    pub const REFRESH_PARTNERS:Self=Self {refresh_partners:true,..Self::CYCLE};
    pub fn refresh_partners(self)->bool {self.refresh_partners}
    pub fn direct(epoch:u64,kind:u32)->Option<Self> {
        (epoch!=0).then_some(Self {direct:Some((epoch,kind)),..Self::CYCLE})
    }
    pub fn direct_kind(self)->Option<u32> {self.direct.map(|(_,kind)|kind)}
    pub fn from_replay(selection:crate::engine::mii::replay::Selection)->Option<Self> {
        if selection.target.configuration().is_none()
            && crate::engine::mii::inventory::MiiClass::from_kind(selection.target.kind()).is_some() {return None;}
        selection.current(selection.generation,selection.revision)
            .then_some(Self {replay:Some(selection),..Self::CYCLE})
    }
    /// Renew an already-active exact Mii receipt; this intent never authorizes INSTALL.
    pub fn acknowledge_replay(selection:crate::engine::mii::replay::Selection)->Option<Self> {
        selection.target.configuration()?;
        let mut request=Self::from_replay(selection)?;
        request.receipt_only=true;
        Some(request)
    }
    pub fn can_install(self)->bool {!self.receipt_only}
    pub fn replay(self)->Option<crate::engine::mii::replay::Selection> {self.replay}
    pub fn from_menu(epoch:u64,context_epoch:u64,
        live:Option<(crate::engine::mii::inventory::MiiClass,crate::engine::mii::inventory::Canonical)>,
        current:Option<crate::engine::mii::inventory::Canonical>,action:crate::engine::mii::selection::Action)->Option<Self> {
        let (Some((class,canonical)),Some(active),crate::engine::mii::selection::Action::ReloadCpu(choice))=
            (live,current,action) else {return None};
        (epoch!=0 && epoch==context_epoch && canonical==active && class==active.class
            && choice.canonical.class==class && choice.canonical!=active)
            .then_some(Self {selection:Some(MenuSelection {epoch,active,choice}),..Self::CYCLE})
    }
    pub fn current(self,epoch:u64,active:Option<crate::engine::mii::inventory::Canonical>)->bool {
        self.direct.is_none_or(|(opening,_)|opening==epoch)
            && self.selection.is_none_or(|s|epoch==s.epoch && active==Some(s.active))
    }
    pub fn choice(self)->Option<crate::engine::mii::selection::Choice> {self.selection.map(|s|s.choice)}
}

/// Catalog capture is read-only. Exclusive retention begins only with an actual CPU ticket.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reservation(bool);
impl Reservation {
    pub const fn empty() -> Self { Self(false) }
    pub fn held(self) -> bool { self.0 }
    /// A retained CPU payload blocks CPU ownership and every claim touching record1.
    pub fn conflicts(self, role:usize, record:u16)->bool {
        self.0 && (role==OWNER || record==DESTINATION)
    }
    pub fn routes(self, requested: bool, active: bool) -> bool { self.0 || requested || active }
    pub fn acquire(&mut self, install_owned: bool) -> bool {
        if !install_owned { return false; }
        self.0 = true; true
    }
    pub fn retire(&mut self, exact_owner: bool) { if exact_owner { self.0 = false; } }
}

/// Identity publication after the generic loader's native completion proof.
/// This does not grant INSTALL, reservation, or menu qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericOrdinaryFacts {
    pub reservation_held: bool,
    pub origin_is_saved: bool,
    pub live_is_saved: bool,
    pub transaction_busy: bool,
    pub kind: u32,
    pub origin_kind: u32,
    pub published_kind: u32,
    pub index: u16,
    pub descriptor_equals_origin: bool,
    pub roster_cursor: Option<usize>,
}

pub fn generic_ordinary_commit(f: GenericOrdinaryFacts) -> Result<Destination, &'static str> {
    generic_ordinary_commit_in(crate::engine::mii::backend::Layout::Training,0,f)
}
pub fn generic_ordinary_commit_in(layout:crate::engine::mii::backend::Layout,original_index:u16,f:GenericOrdinaryFacts)->Result<Destination,&'static str> {
    if f.transaction_busy { return Err("generic ordinary CPU transaction busy"); }
    if f.reservation_held { return Err("generic ordinary CPU destination reserved"); }
    if f.origin_is_saved || f.live_is_saved { return Err("saved CPU stays on the owned adapter"); }
    if f.kind != f.published_kind { return Err("generic ordinary CPU published kind differs"); }
    if crate::engine::mii::inventory::MiiClass::from_kind(f.kind).is_some() {
        return Err("Mii CPU transition is not a generic ordinary commit");
    }
    if f.index != if layout==crate::engine::mii::backend::Layout::Training {0}else{original_index} { return Err("generic ordinary CPU changed native original index"); }
    if f.kind == f.origin_kind && f.descriptor_equals_origin {
        Ok(Destination { target: Target::ORIGINAL, cursor: None })
    } else {
        Ok(Destination { target: Target::fighter(f.kind), cursor: f.roster_cursor })
    }
}

#[cfg(test)]
mod generic_completion_tests {
    lib_unit_tests! {
    use super::*;

    fn loaded(kind: u32) -> GenericOrdinaryFacts {
        GenericOrdinaryFacts { reservation_held:false,origin_is_saved:false,live_is_saved:false,
            transaction_busy:false,kind,origin_kind:0x1e,published_kind:kind,index:0,
            descriptor_equals_origin:false,roster_cursor:Some(0) }
    }

    fn ordinary_cycle_commits_kirby_mario_and_exact_origin() {
        assert!(!Reservation::empty().routes(false,false));
        for (kind,cursor) in [(0x6,0),(0x0,1)] {
            let f=GenericOrdinaryFacts {roster_cursor:Some(cursor),..loaded(kind)};
            assert_eq!(generic_ordinary_commit(f),Ok(Destination {target:Target::fighter(kind),cursor:Some(cursor)}));
        }
        for kind in [0x1e,0x0] {
            let f=GenericOrdinaryFacts {origin_kind:kind,descriptor_equals_origin:true,..loaded(kind)};
            assert_eq!(generic_ordinary_commit(f),Ok(Destination {target:Target::ORIGINAL,cursor:None}));
            assert_eq!(generic_ordinary_commit(GenericOrdinaryFacts {descriptor_equals_origin:false,..f}),
                Ok(Destination {target:Target::fighter(kind),cursor:Some(0)}));
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("ordinary_cycle_commits_kirby_mario_and_exact_origin", ordinary_cycle_commits_kirby_mario_and_exact_origin as fn()),
        ]);
    }
    }
}

/// Native Training's four present rows include inactive helpers. Their current index AND
/// selector protect records. Repeated original index0 is permitted; destination1 is not shared.
pub fn assignment(owners: &[Option<Owner>; 8]) -> Option<[Option<u16>; 8]> {
    assignment_for(OWNER, owners)
}

/// One proved selector destination; callers supply live ownership and phase authority.
pub fn assignment_for(destination: usize, owners: &[Option<Owner>; 8]) -> Option<[Option<u16>; 8]> {
    assignment_in(crate::engine::mii::backend::Layout::Training,destination,owners)
}
pub fn assignment_in(layout:crate::engine::mii::backend::Layout,destination:usize,owners:&[Option<Owner>;8])->Option<[Option<u16>;8]> {
    assignment_with(layout,crate::engine::mii::capacity::Extras::NONE,destination,owners)
}
/// `assignment_in` with exactly the registered created entries present. An extra is
/// never a destination; its current index and selector join the protected records,
/// and an extra index/selector equal to the destination refuses. A protected set
/// that cannot fit the fixed record table refuses instead of truncating.
pub fn assignment_with(layout:crate::engine::mii::backend::Layout,extras:crate::engine::mii::capacity::Extras,
    destination:usize,owners:&[Option<Owner>;8])->Option<[Option<u16>;8]> {
    if !layout.selector(destination) || extras.entry(destination).is_some() {return None;}
    protected_assignment(layout, extras, destination, owners)
}

/// Same record-protection walk for an independently qualified destination.
/// Callers must establish either the native layout selector or CreatedDestination.
fn protected_assignment(layout:crate::engine::mii::backend::Layout,
    extras:crate::engine::mii::capacity::Extras, destination:usize,
    owners:&[Option<Owner>;8])->Option<[Option<u16>;8]> {
    if !layout.shape_with(&owners.map(|o|o.map_or(0,|o|o.entry)),extras) {
        return None;
    }
    let mut protected = [None; 8];
    let mut used = 0;
    for id in layout.owners().into_iter().chain(extras.slots().map(|(slot,_)|slot)) {
        let owner=owners[id]?;
        if owner.selector != id as u8 || owner.alias != 0
            || [owner.entry, owner.row, owner.information, owner.object].iter()
                .any(|p| *p == 0 || !p.is_multiple_of(8)) {
            return None;
        }
        for other in owners[..id].iter().flatten() {
            if owner.entry == other.entry || owner.row == other.row
                || owner.information == other.information || owner.object == other.object {
                return None;
            }
        }
        if id == destination {
            if layout==crate::engine::mii::backend::Layout::Training && owner.index != 0 && owner.index != destination as u16 { return None; }
        } else {
            if owner.index == destination as u16 || usize::from(owner.selector) == destination {
                return None;
            }
            for index in [owner.index,u16::from(owner.selector)] {
                if !protected[..used].contains(&Some(index)) {
                    if used == protected.len() {return None;}
                    protected[used]=Some(index);used+=1;
                }
            }
        }
    }
    // Original record0 remains protected on forward, return, and reuse.
    if !protected[..used].contains(&Some(0)) {
        if used == protected.len() {return None;}
        protected[used]=Some(0);
    }
    Some(protected)
}

/// Captured ordinary created-entry destination, separate from Layout::selector.
/// Values only: this never grants native access or proves a loaded Mii. The
/// adapter must retain its INSTALL/SourceBook ticket and recheck the full native
/// map, G bounds, descriptor and protected record bytes before publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreatedDestination {
    owner: Owner,
    extras: crate::engine::mii::capacity::Extras,
    creation_selector: u32,
    original_kind: u32,
    record: u16,
    protected: [Option<u16>; 8],
}
impl CreatedDestination {
    pub fn capture(
        layout: crate::engine::mii::backend::Layout,
        extras: crate::engine::mii::capacity::Extras,
        slot: usize,
        owners: &[Option<Owner>; 8],
        kinds: &[Option<u32>; 8],
        facts: crate::engine::mii::capacity::ExtraFacts,
        record_count: usize,
    ) -> Option<Self> {
        // F9's replacement destinations belong to native Training. Waiting
        // retains its existing two-helper contract.
        if layout != crate::engine::mii::backend::Layout::Training { return None; }
        let owner = *owners.get(slot)?.as_ref()?;
        extras.admit(slot, &owner, facts).ok()?;
        if kinds[slot] != Some(facts.entry_kind) { return None; }
        crate::engine::mii::role_owner::claims_in_with(layout, extras, owners, kinds)?;
        let protected = protected_assignment(layout, extras, slot, owners)?;
        if slot >= record_count || protected.iter().flatten().any(|i| usize::from(*i) >= record_count) {
            return None;
        }
        Some(Self { owner, extras, creation_selector:facts.descriptor_selector,
            original_kind:facts.entry_kind, record: slot as u16, protected })
    }
    pub fn owner(self) -> Owner { self.owner }
    pub fn record(self) -> u16 { self.record }
    pub fn protected(self) -> [Option<u16>; 8] { self.protected }


    /// Revalidate the exact creation registration, never just a physical slot.
    pub fn registered_in(self, layout:crate::engine::mii::backend::Layout,
        extras:crate::engine::mii::capacity::Extras, slot:usize, entry:usize)->bool {
        layout==crate::engine::mii::backend::Layout::Training && self.extras.same_registration(extras,slot)
            && slot==usize::from(self.record) && entry==self.owner.entry
            && extras.entry(slot)==Some(entry)
    }

    /// Rewalk every current/selector claim in each phase. The caller retains
    /// the protected bytes, not merely these indices, through publication.
    pub fn protected_now(self, layout:crate::engine::mii::backend::Layout,
        extras:crate::engine::mii::capacity::Extras,
        owners:&[Option<Owner>;8], kinds:&[Option<u32>;8])->Option<[Option<u16>;8]> {
        let slot=usize::from(self.record);
        if !self.registered_in(layout,extras,slot,owners[slot]?.entry) {return None;}
        crate::engine::mii::role_owner::claims_in_with(layout,extras,owners,kinds)?;
        // The adapter captures these current indices and bytes in Snapshot.
        // Publication/completion compare the complete Snapshot protected union;
        // a later independent creation must not invalidate a ready receipt.
        protected_assignment(layout,extras,slot,owners)
    }

    /// Rebase only a previously captured destination after its complete native
    /// owner phase matches. The adapter must first verify the current exact
    /// SourceBook receipt, G payload and native readiness; raw capture remains
    /// ordinary-only and cannot use this as an unregistered Mii origin.
    pub fn reuse_completed(self,now:Owner,facts:crate::engine::mii::capacity::ExtraFacts,
        kind:u32)->Option<Self> {
        self.completed_owner(now,facts,kind).then_some(Self {owner:now,original_kind:kind,..self})
    }

    /// Separate from the layout helper admission. A created destination has
    /// already proved its physical row and native creation roots.
    pub fn headgear_reload(self,before:u16,target:u16)->Result<bool,&'static str> {
        if target==u16::MAX {return Err("invalid created headgear row");}
        Ok(before!=target)
    }

    /// Fresh created publication retains the captured outgoing O and has no
    /// configure-skip or bank mutation exception.
    pub fn outgoing(self,header:[u32;5],tokens:[u16;4],now:OutgoingObject,
        initialized:bool,outstanding:Option<u32>)->bool {
        use crate::engine::mii::identity::{Identity,ExpectedMutation,verify};
        let captured=Identity {object:self.owner.object,header,index:self.owner.index,
            tokens:tokens.map(u32::from),..Identity::ZERO};
        let observed=Identity {object:now.object,header:now.header,index:now.index,
            tokens:now.tokens,..Identity::ZERO};
        !initialized && outstanding==Some(0) && header[0]==u32::from(self.record)
            && verify(&captured,&observed,ExpectedMutation::None).is_ok()
    }

    /// Created sources always carry an exact payload through a fresh native O.
    /// No bank/retained path or origin restore is authorized by this capability.
    pub fn validate_hat(self,hat:&HatPublication<'_>)->Result<(),&'static str> {
        let c=hat.payload.ok_or("created publication requires exact payload")?;
        let index=u16::from_le_bytes(hat.descriptor[0x46..0x48].try_into().unwrap());
        let changed=self.headgear_reload(hat.before,hat.target)?;
        if hat.layout!=crate::engine::mii::backend::Layout::Training
            || hat.owner!=u32::from(self.record) || !hat.source || hat.initialized
            || hat.destination_index!=self.record || index!=self.record
            || hat.original_index!=self.owner.index || hat.opt_in!=changed
            || (changed && !hat.fingerprints)
            || c.tokens().hat!=hat.target
            || u16::from_le_bytes(hat.descriptor[0x4a..0x4c].try_into().unwrap())!=hat.target
            || u32::from_le_bytes(hat.descriptor[0x18..0x1c].try_into().unwrap())!=c.canonical().class as u32
            || u32::from_le_bytes(hat.descriptor[4..8].try_into().unwrap())!=self.creation_selector {
            return Err("unqualified created headgear publication");
        }
        Ok(())
    }

    /// Same saved payload encoding as the native selector adapter, but only
    /// this captured destination chooses the G record. Creation word1 and
    /// unrelated descriptor bytes remain the captured values.
    pub fn staged(self, original: [u8; 0x70], target: Configuration) -> [u8; 0x70] {
        let mut descriptor = crate::engine::mii::ordinary_player::staged_descriptor(original, Some(target));
        descriptor[0x46..0x48].copy_from_slice(&self.record.to_le_bytes());
        descriptor
    }

    pub fn publication_indices(self, target_index:u16, e:u16, fe:u16, d:u16) -> bool {
        target_index == self.record && e == self.owner.index && d == self.owner.index && fe == self.record
    }

    fn same_roots(self, now:Owner, facts:crate::engine::mii::capacity::ExtraFacts) -> bool {
        Owner { index:self.owner.index, ..now } == self.owner
            && facts.entry_owner == u32::from(self.record) && facts.row_owner == u32::from(self.record)
            && facts.descriptor_selector == self.creation_selector
            && self.extras.owned_roots(usize::from(self.record),facts.resource,facts.alias_root,facts.selection).is_ok()
    }

    /// FE has the exact staged descriptor, while E/D/O still hold the outgoing
    /// values. The adapter must separately prove loader6, exact descriptor,
    /// uninitialized O, no outstanding task, and its executing source ticket.
    pub fn staged_owner(self, now:Owner, facts:crate::engine::mii::capacity::ExtraFacts, kind:u32) -> bool {
        self.same_roots(now,facts) && supported_kind(kind)
            && now.index == self.owner.index
            && facts.entry_index == self.record
            && facts.information_index == self.owner.index && facts.object_index == self.owner.index
            && facts.entry_kind == kind && facts.object_kind == self.original_kind
    }

    /// Published mirrors only, never load-completion or exact-payload proof.
    /// Completion additionally checks G CharInfo/resource bytes, all tokens,
    /// descriptor/body readiness, and unchanged protected records under G.
    pub fn completed_owner(self, now:Owner, facts:crate::engine::mii::capacity::ExtraFacts, kind:u32) -> bool {
        self.same_roots(now,facts) && supported_kind(kind)
            && now.index == self.record && facts.entry_index == self.record
            && facts.information_index == self.record && facts.object_index == self.record
            && facts.entry_kind == kind && facts.object_kind == kind
    }
}

pub fn staged_descriptor(original: [u8; 0x70], target: Option<Configuration>) -> [u8; 0x70] {
    staged_descriptor_for(OWNER, original, target).expect("CPU selector")
}


/// Preserve only captures needed by this request. Unknown original kind waits
/// for the existing native original-kind publication, not catalog qualification.
pub fn helper_captures_ready(mii_request: bool, original_cpu: Option<u32>,
    player_captured: bool, cpu_captured: bool) -> bool {
    if mii_request { return player_captured && cpu_captured; }
    match original_cpu {
        Some(kind) if supported_kind(kind) => cpu_captured,
        Some(_) => true,
        None => false,
    }
}

/// Preserve the chosen exact configuration. A hat that differs from the
/// captured helper tokens is that configuration's own hat; the helper adapter
/// routes it to fresh O instead of substituting another Mii or refusing it.
pub fn helper_configuration(configuration: Configuration, _inherited_hats: [u16; 2])
    -> Result<Configuration, &'static str> {
    Ok(configuration)
}

/// Callback admission for one helper slot. `captured[ordinal]` is that slot's
/// immutable origin hat. [`headgear_reload`] accepts an equal hat on every
/// layout and a changed hat for a helper that belongs to that layout.
/// Training uses slots 2/3. Waiting uses slots 5/6. A foreign slot is refused.
/// This does not rewrite the captured pair, pick another Mii, or grant a lease.
pub fn helper_slot_admission(layout:crate::engine::mii::backend::Layout,slot:usize,captured:[u16;2],hat:u16)->bool {
    let Some(ordinal)=layout.helper(slot) else {return false;};
    headgear_reload(layout,slot,captured[ordinal],hat).is_ok()
}

/// Saved CharInfo is copied to `binding.record_index`. Keep that payload only
/// when the published row is that same destination. An origin index publishes
/// the untouched origin record and must not receive the saved bytes.
pub fn saved_payload_routes_to_destination(published_index:u16,destination_index:u16)->bool {
    published_index==destination_index
}

/// No selector allocator: only the caller's already-proved owner is accepted.
pub fn staged_descriptor_for(owner: usize, original: [u8; 0x70], target: Option<Configuration>) -> Option<[u8; 0x70]> {
    staged_descriptor_in(crate::engine::mii::backend::Layout::Training,owner,original,target)
}
pub fn staged_descriptor_in(layout:crate::engine::mii::backend::Layout,owner:usize,original:[u8;0x70],target:Option<Configuration>)->Option<[u8;0x70]> {
    if !layout.selector(owner) { return None; }
    let mut result = crate::engine::mii::ordinary_player::staged_descriptor(original, target);
    if target.is_some() {
        // Saved row is the validated physical slot. That slot is also
        // Binding.record_index, the record publication writes. The captured
        // origin index stays in `original` and is not copied here.
        result[0x46..0x48].copy_from_slice(&(owner as u16).to_le_bytes());
    }
    Some(result)
}

/// The exact-Mii bytes each visible owner's adapter would INSTALL for `target`
/// over `original`: Player0 keeps its own record index (`stage_saved` copies the
/// origin binding's index, which the live window's `FE+0x46` already carries), a
/// selector owner takes its physical slot ([`staged_descriptor_in`]). The bank
/// planner derives a Mii group's identity with this rule (MII-GROUPS-DESIGN §2.1).
pub fn staged_descriptor_for_owner(layout:crate::engine::mii::backend::Layout,owner:usize,original:[u8;0x70],target:Option<Configuration>)->Option<[u8;0x70]> {
    if owner==0 && layout.visible()[0]==0 {
        return Some(crate::engine::mii::ordinary_player::staged_descriptor(original,target));
    }
    staged_descriptor_in(layout,owner,original,target)
}

/// The mixed phase is explicit: native FE already contains target; E and D still contain before.
pub fn publication_indices(before:u16,target_index:u16,e:u16,fe:u16,d:u16)->bool {
    // This is only descriptor-shape evidence. The caller still requires the executing
    // typed ticket, current protected union, loader6 and selected-O phase proof.
    publication_indices_for(OWNER, before, target_index, e, fe, d)
}
pub fn publication_indices_for(owner:usize,before:u16,target_index:u16,e:u16,fe:u16,d:u16)->bool {
    publication_indices_in(crate::engine::mii::backend::Layout::Training,owner,before,target_index,e,fe,d)
}
pub fn publication_indices_in(layout:crate::engine::mii::backend::Layout,owner:usize,before:u16,target_index:u16,e:u16,fe:u16,d:u16)->bool {
    layout.selector(owner) && (layout.waiting() || before==0 || before==owner as u16)
        && (layout.waiting() || target_index==0 || target_index==owner as u16)
        && e==before && d==before && fe==target_index
}


/// At prequeue FE contains the target, but native has not written O's target header.
/// Compare copied outgoing values only; current map/phase/lock authority remains the caller's.
/// `before_header` is the outgoing O header the owner snapshot read under the same lock
/// as `before`. It is not derived from the descriptor: after a same-R group exchange the
/// shared resource keeps the original window's kinds while FE+0x18.. names the exchanged
/// group (`GroupExchange::new`), so the descriptor words are not an expectation for it
/// (D-PTA1, PTA116T #658: header [1, 9, 1, 5, 0] under FE words [0x21, 0x1e, -1]).
pub fn outgoing_object_matches(
    before: Owner,
    before_header: [u32; 5],
    before_tokens: [u16; 4],
    selected: usize,
    header: [u32; 5],
    index: u16,
    tokens: [u32; 4],
) -> bool {
    usize::from(before.selector)==OWNER
        && outgoing_object_matches_for(before, before_header, before_tokens, selected, header, index, tokens)
}
/// WP5a: the prequeue proof is `identity::verify(captured, now, ExpectedMutation::None)` over the
/// outgoing object, its captured header, index and tokens; the selector clauses stay as they were.
pub fn outgoing_object_matches_for(
    before: Owner, before_header: [u32; 5], before_tokens: [u16; 4],
    selected: usize, header: [u32; 5], index: u16, tokens: [u32; 4],
) -> bool {
    outgoing_object_matches_with(before, before_header, before_tokens,
        OutgoingObject { object: selected, header, index, tokens }, crate::engine::mii::identity::ExpectedMutation::None)
}

/// The selected outgoing O as the prequeue reads it: its address, header `O+0x54`, record index
/// `O+0x68` and tokens `O+0x6c..0x7c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutgoingObject {
    pub object: usize,
    pub header: [u32; 5],
    pub index: u16,
    pub tokens: [u32; 4],
}

/// [`outgoing_object_matches_for`] with the one mutation the gate may have made to the O before
/// the proof runs (U5, W-M2-4): `ExpectedMutation::ConfigureSkipped { index, tokens }` admits exactly
/// the configure-equivalent `O+0x68` index and tokens the gate installed on the configure-skip path
/// (`O+0x4c` bit0 set at `0x6510f4`) and nothing else; `ExpectedMutation::None` is the plain proof.
pub fn outgoing_object_matches_with(
    before: Owner, before_header: [u32; 5], before_tokens: [u16; 4],
    now: OutgoingObject, allowed: crate::engine::mii::identity::ExpectedMutation,
) -> bool {
    use crate::engine::mii::identity::{verify, Identity};
    let captured = Identity { object: before.object, header: before_header, index: before.index, tokens: before_tokens.map(u32::from), ..Identity::ZERO };
    let now = Identity { object: now.object, header: now.header, index: now.index, tokens: now.tokens, ..Identity::ZERO };
    matches!(before.selector,1..=6)
        && before_header[0] == u32::from(before.selector)
        && verify(&captured, &now, allowed).is_ok()
}
/// The outgoing O header a canonical or window load leaves before publication: the
/// selector, the descriptor's three group words and the `+0x41` flag. An exchanged
/// group's outgoing header is the captured one, not this value.
pub fn descriptor_header(selector: u8, descriptor: &[u8; 0x70]) -> [u32; 5] {
    let word = |offset: usize| u32::from_le_bytes([
        descriptor[offset], descriptor[offset + 1],
        descriptor[offset + 2], descriptor[offset + 3],
    ]);
    [u32::from(selector), word(0x18), word(0x1c), word(0x20), u32::from(descriptor[0x41])]
}

#[cfg(test)]
mod native_rank_tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::mii::backend::Layout;

    // Synthetic values only. The oracle is the native rank domain and exact
    // preservation of every other descriptor byte, not a copied game descriptor.
    fn original(layout:Layout)->(Binding,[u8;0x70]) {
        let owner=layout.visible()[1];
        let binding=Binding {epoch:7,owner,entry:0x1000,row:0x2000,holder:0x3000,
            manager:0x4000,vector_begin:0x5000,vector_end:0x5000+32*0x68,record_index:owner as u16};
        let mut descriptor=[0;0x70];
        descriptor[4..8].copy_from_slice(&owner.to_le_bytes());
        // A CPU origin has no controller.
        descriptor[0x10..0x14].copy_from_slice(&(-1i32).to_le_bytes());
        descriptor[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        descriptor[0x18..0x1c].copy_from_slice(&9u32.to_le_bytes());
        descriptor[0x1c..0x24].fill(0xff);
        descriptor[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        descriptor[0x34]=1;descriptor[0x35]=1;descriptor[0x41]=1;
        let index=if layout.waiting() {26u16}else{0};
        descriptor[0x46..0x48].copy_from_slice(&index.to_le_bytes());
        (binding,descriptor)
    }

    fn saved_original_and_saved_reuse_retain_live_rank_without_mutating_origin() {
        use {crate::engine::mii::inventory::MiiClass, crate::engine::mii::lifetime::Tokens, crate::engine::mii::native_values::RowIds};
        let saved=Configuration::from_observed(MiiClass::Swordfighter,
            Tokens {outfit:0,hat:0,colour:4,voice:37,specials:[1,0,0,0]},[0;88],
            RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea}).unwrap();
        for layout in [Layout::Training,Layout::Ldn,Layout::Quickplay] {
            for original_kind in [9u32,0x48] {
                let (binding,mut origin)=original(layout);
                origin[0x18..0x1c].copy_from_slice(&original_kind.to_le_bytes());
                let frozen_origin=origin;
                let mut observed=origin;observed[0x37]=50;
                let expected=recapture_rank(layout,binding,binding,&origin,&observed).unwrap();
                let live_origin=with_native_rank(origin,&expected).unwrap();
                let first=staged_descriptor_in(layout,binding.owner as usize,live_origin,Some(saved)).unwrap();
                assert_eq!(first[0x37],50);
                assert_eq!(u32::from_le_bytes(first[0x18..0x1c].try_into().unwrap()),0x49);
                assert_eq!(u16::from_le_bytes(first[0x46..0x48].try_into().unwrap()),binding.record_index);
                assert_eq!(binding.record_index,binding.owner as u16);
                assert_eq!(&first[0x34..0x37],&origin[0x34..0x37]);
                assert_eq!(&first[0x38..0x43],&origin[0x38..0x43]);

                // After exact completion, native changes rank while this saved Mii is live.
                let mut changed=first;changed[0x37]=77;
                let expected=recapture_rank(layout,binding,binding,&first,&changed).unwrap();
                let live_origin=with_native_rank(origin,&expected).unwrap();
                let returned=staged_descriptor_in(layout,binding.owner as usize,live_origin,None).unwrap();
                let mut wanted=origin;wanted[0x37]=77;
                assert_eq!(returned,wanted); // Original identity/index/role, current native rank.
                assert_ne!(returned,origin);
                let reused=staged_descriptor_in(layout,binding.owner as usize,
                    with_native_rank(origin,&returned).unwrap(),Some(saved)).unwrap();
                let mut wanted=first;wanted[0x37]=77;
                assert_eq!(reused,wanted);
                assert_eq!(origin,frozen_origin);
            }
        }
    }

    // Synthetic behaviors: the oracle is each fixture's own FE+0x34..=0x37 bytes and the
    // unchanged origin bytes around them, not a copied game value. `control` models a
    // non-CPU operator (operation byte not 1) with its own AI+0xc10 byte.
    fn behavior(origin:[u8;0x70],operation:u8,control:u8,ai:u8,rank:u8)->[u8;0x70] {
        let mut descriptor=origin;
        descriptor[0x34]=operation;descriptor[0x35]=control;descriptor[0x36]=ai;descriptor[0x37]=rank;
        descriptor
    }
    // A behavior with an assigned controller word (Training Behavior=Control, C104).
    fn controlled(origin:[u8;0x70],controller:i32,operation:u8,control:u8,ai:u8,rank:u8)->[u8;0x70] {
        let mut descriptor=behavior(origin,operation,control,ai,rank);
        descriptor[0x10..0x14].copy_from_slice(&controller.to_le_bytes());
        descriptor
    }

    fn control_assignment_recaptures_the_controller_word_and_returns_to_cpu() {
        use {crate::engine::mii::inventory::MiiClass, crate::engine::mii::lifetime::Tokens, crate::engine::mii::native_values::RowIds};
        let saved=Configuration::from_observed(MiiClass::Swordfighter,
            Tokens {outfit:0,hat:0,colour:4,voice:37,specials:[1,0,0,0]},[0;88],
            RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea}).unwrap();
        for layout in [Layout::Training,Layout::Ldn,Layout::Quickplay] {
            for original_kind in [0x3fu32,9,0x48] {
                let (binding,mut origin)=original(layout);
                origin[0x18..0x1c].copy_from_slice(&original_kind.to_le_bytes());
                origin[0x37]=100;
                let frozen=origin;
                assert_eq!(native_controller(&origin),Ok(-1));
                // Exactly the C104 #336 descriptor diff: 10..13, 34, 35 plus rank.
                let control=controlled(origin,1,0,0x24,0,21);
                assert_eq!(non_rank_descriptor_diff(&origin,&control).count,6);
                assert_eq!(recapture_rank(layout,binding,binding,&origin,&control),Ok(control));
                assert_eq!(native_controller(&control),Ok(1));
                // The control-type-only preflight helper keeps refusing the controller word.
                assert!(refresh_control_type(layout,binding,binding,&origin,&control).is_err());
                let live_origin=with_native_live_metadata(origin,&control).unwrap();
                assert_eq!(live_origin,control);
                // Original restore and exact Mii staging both keep the assignment.
                let restored=staged_descriptor_in(layout,binding.owner as usize,live_origin,None).unwrap();
                assert_eq!(restored,control);
                let exact=staged_descriptor_in(layout,binding.owner as usize,live_origin,Some(saved)).unwrap();
                assert_eq!(&exact[0x10..0x14],&1i32.to_le_bytes());
                assert_eq!(&exact[0x34..0x38],&control[0x34..0x38]);
                assert_eq!(u32::from_le_bytes(exact[0x18..0x1c].try_into().unwrap()),0x49);
                assert_eq!(&exact[0x38..0x43],&origin[0x38..0x43]);
                // Releasing Control on the live Mii returns the CPU bundle; the origin returns exactly.
                let mut released=exact;released[0x10..0x14].copy_from_slice(&(-1i32).to_le_bytes());
                released[0x34]=1;released[0x35]=1;released[0x37]=100;
                assert_eq!(recapture_rank(layout,binding,binding,&exact,&released),Ok(released));
                assert_eq!(with_native_live_metadata(origin,&released),Ok(origin));
                // A generic ordinary return under Control is still the original.
                if MiiClass::from_kind(original_kind).is_none() {
                    let index=u16::from_le_bytes(origin[0x46..0x48].try_into().unwrap());
                    let facts=GenericOrdinaryFacts {
                        reservation_held:false,origin_is_saved:false,live_is_saved:false,transaction_busy:false,
                        kind:original_kind,origin_kind:original_kind,published_kind:original_kind,index,
                        descriptor_equals_origin:control==live_origin,roster_cursor:Some(2)};
                    assert_eq!(generic_ordinary_commit_in(layout,index,facts),
                        Ok(Destination {target:Target::ORIGINAL,cursor:None}));
                }
                assert_eq!(origin,frozen);
            }
        }
    }

    fn controller_word_outside_the_native_domain_or_its_neighbours_refuses() {
        let (binding,origin)=original(Layout::Training);
        let control=controlled(origin,1,0,0x24,0,21);
        for controller in [-2i32,9,0x100,0x7fff_ffff,i32::MIN] {
            let unqualified=controlled(origin,controller,0,0x24,0,21);
            assert!(native_controller(&unqualified).is_err());
            assert!(recapture_rank(Layout::Training,binding,binding,&origin,&unqualified).is_err());
            assert!(with_native_live_metadata(origin,&unqualified).is_err());
            // An unqualified frozen word is not a base for a qualified change either.
            assert!(recapture_rank(Layout::Training,binding,binding,&unqualified,&control).is_err());
        }
        // Every value inside the native domain is adopted with the rest of the bundle.
        for controller in NATIVE_CONTROLLER_NONE..=NATIVE_CONTROLLER_MAX {
            let assigned=controlled(origin,controller,0,0x24,0,21);
            assert_eq!(recapture_rank(Layout::Training,binding,binding,&origin,&assigned),Ok(assigned));
        }
        // Neighbouring identity bytes beside a controller change: 0x0c..0x0f, form count, kind.
        for offset in [0x0c,0x0f,0x14,0x15,0x17,0x18] {
            let mut changed=control;changed[offset]^=1;
            assert!(recapture_rank(Layout::Training,binding,binding,&origin,&changed).is_err(),
                "identity change at {offset:#x} was adopted beside a controller change");
            assert_ne!(with_native_live_metadata(origin,&changed),Ok(changed));
        }
        // Unqualified control type or rank beside a qualified controller change still refuses.
        for unqualified in [controlled(origin,1,0,0x28,0,21),controlled(origin,1,0,0x24,0,101)] {
            assert!(recapture_rank(Layout::Training,binding,binding,&origin,&unqualified).is_err());
            assert!(with_native_live_metadata(origin,&unqualified).is_err());
        }
        // A retired or foreign lifetime never adopts a controller assignment.
        let retired=Binding {epoch:binding.epoch+1,..binding};
        assert!(recapture_rank(Layout::Training,binding,retired,&origin,&control).is_err());
        assert!(recapture_rank(Layout::Training,binding,Binding {owner:0,..binding},&origin,&control).is_err());
        // An unchanged word keeps any value: no retroactive refusal of a captured origin.
        let stable=controlled(origin,0x40,1,1,0,0);
        assert_eq!(recapture_rank(Layout::Training,binding,binding,&stable,&stable),Ok(stable));
        let mut stable_rank=stable;stable_rank[0x37]=50;
        assert_eq!(recapture_rank(Layout::Training,binding,binding,&stable,&stable_rank),Ok(stable_rank));
    }

    fn original_restore_and_exact_mii_staging_keep_the_live_behavior() {
        use {crate::engine::mii::inventory::MiiClass, crate::engine::mii::lifetime::Tokens, crate::engine::mii::native_values::RowIds};
        let saved=Configuration::from_observed(MiiClass::Swordfighter,
            Tokens {outfit:0,hat:0,colour:4,voice:37,specials:[1,0,0,0]},[0;88],
            RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea}).unwrap();
        for layout in [Layout::Training,Layout::Ldn,Layout::Quickplay] {
            for original_kind in [9u32,0x48,0x4a] {
                let (binding,mut origin)=original(layout);
                origin[0x18..0x1c].copy_from_slice(&original_kind.to_le_bytes());
                let frozen=origin;
                // Control chosen after the owner was captured.
                let live=behavior(origin,0,1,3,40);
                let live_origin=with_native_live_metadata(origin,&live).unwrap();
                assert_eq!(live_origin,live);
                let restored=staged_descriptor_in(layout,binding.owner as usize,live_origin,None).unwrap();
                assert_eq!(restored,live);
                let exact=staged_descriptor_in(layout,binding.owner as usize,live_origin,Some(saved)).unwrap();
                assert_eq!(&exact[0x34..0x38],&live[0x34..0x38]);
                assert_eq!(u32::from_le_bytes(exact[0x18..0x1c].try_into().unwrap()),0x49);
                assert_eq!(&exact[0x38..0x43],&origin[0x38..0x43]);
                assert_eq!(origin,frozen);
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("saved_original_and_saved_reuse_retain_live_rank_without_mutating_origin", saved_original_and_saved_reuse_retain_live_rank_without_mutating_origin as fn()),
            ("control_assignment_recaptures_the_controller_word_and_returns_to_cpu", control_assignment_recaptures_the_controller_word_and_returns_to_cpu as fn()),
            ("controller_word_outside_the_native_domain_or_its_neighbours_refuses", controller_word_outside_the_native_domain_or_its_neighbours_refuses as fn()),
            ("original_restore_and_exact_mii_staging_keep_the_live_behavior", original_restore_and_exact_mii_staging_keep_the_live_behavior as fn()),
        ]);
    }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    fn rows() -> [Option<Owner>; 8] {
        core::array::from_fn(|id| (id < 4).then_some(Owner {
            entry: 0x1000 + id*0x100, row: 0x2000 + id*0x100,
            information: 0x3000 + id*0x100, object: 0x4000 + id*0x100,
            index: 0, selector: id as u8, alias: 0,
        }))
    }

    fn created_destination_requires_registered_roots_and_preserves_every_other_record() {
        use crate::engine::mii::{backend::Layout, capacity::{Extras, Roots, ExtraFacts}};
        let mut owners = rows();
        let extra = Owner { entry:0x1700, row:0x2700, information:0x3700,
            object:0x4700, index:0, selector:7, alias:0 };
        owners[7] = Some(extra);
        let mut kinds = [None;8];
        for slot in [0,1,2,3,7] { kinds[slot]=Some(9); }
        // C98b creation retains descriptor word1=0 while E.selector=7.
        let roots = Roots { entry:extra.entry, resource:0x7000, alias:extra.object,
            selection:2, descriptor_selector:0 };
        let extras = Extras::registered([Some(roots),None,None]).unwrap();
        let facts = ExtraFacts { entry_owner:7, row_owner:7, descriptor_selector:0,
            entry_index:0, information_index:0, object_index:0,
            entry_kind:9, object_kind:9, resource:0x7000,
            alias_root:extra.object, selection:2 };
        let capture = |extras, owners:&[Option<Owner>;8], kinds:&[Option<u32>;8], facts, count|
            CreatedDestination::capture(Layout::Training,extras,7,owners,kinds,facts,count);
        let accepted = capture(extras,&owners,&kinds,facts,8).unwrap();
        assert_eq!(accepted.owner(),extra);
        assert_eq!(accepted.record(),7);
        use crate::engine::mii::{inventory::MiiClass,lifetime::Tokens,native_values::RowIds};
        let exact=Configuration::from_observed(MiiClass::Swordfighter,
            Tokens {outfit:0,hat:0,colour:4,voice:37,specials:[1,0,0,0]},[0;88],
            RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea}).unwrap();
        let mut original=[0u8;0x70]; original[0x37]=50;
        let descriptor=accepted.staged(original,exact);
        assert_eq!(&descriptor[4..8],&original[4..8],"preserve creation word1, not selector7");
        assert_eq!(descriptor[0x37],50);
        assert_eq!(u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap()),7);
        assert_eq!(u32::from_le_bytes(descriptor[0x18..0x1c].try_into().unwrap()),0x49);
        let staged=ExtraFacts {entry_kind:0x49,entry_index:7,..facts};
        assert!(accepted.staged_owner(extra,staged,0x49));
        assert!(!accepted.staged_owner(extra,ExtraFacts {object_kind:0x49,..staged},0x49));
        assert!(!accepted.staged_owner(extra,ExtraFacts {information_index:7,..staged},0x49));
        let published=ExtraFacts {entry_kind:0x49,object_kind:0x49,
            entry_index:7,information_index:7,object_index:7,..facts};
        let live=Owner {index:7,..extra};
        assert!(accepted.completed_owner(live,published,0x49));
        assert!(!accepted.completed_owner(extra,published,0x49));
        assert!(!accepted.completed_owner(live,ExtraFacts {alias_root:0x4800,..published},0x49));
        assert!(!accepted.completed_owner(live,ExtraFacts {descriptor_selector:7,..published},0x49));
        assert!(!accepted.completed_owner(live,published,0x48));
        assert!(accepted.publication_indices(7,0,7,0));
        assert!(!accepted.publication_indices(1,0,1,0));
        assert!(!accepted.publication_indices(7,7,7,0));
        assert!(!accepted.publication_indices(7,0,7,7));
        assert_eq!(accepted.protected(),[Some(0),Some(1),Some(2),Some(3),None,None,None,None]);
        // Existing selector API cannot acquire the new destination implicitly.
        assert!(assignment_with(Layout::Training,extras,7,&owners).is_none());
        assert!(capture(Extras::new([extra.entry,0,0]).unwrap(),&owners,&kinds,facts,8).is_none());
        assert!(capture(extras,&owners,&kinds,ExtraFacts {descriptor_selector:7,..facts},8).is_none());
        assert!(capture(extras,&owners,&kinds,ExtraFacts {resource:0x7008,..facts},8).is_none());
        assert!(capture(extras,&owners,&kinds,ExtraFacts {information_index:7,..facts},8).is_none());
        assert!(capture(extras,&owners,&kinds,facts,7).is_none());
        for slot in 0..4 {
            let mut changed=owners;
            changed[slot].as_mut().unwrap().index=7;
            assert!(capture(extras,&changed,&kinds,facts,8).is_none(),"current G7 claim must survive");
            changed=owners; changed[slot].as_mut().unwrap().selector=7;
            assert!(capture(extras,&changed,&kinds,facts,8).is_none(),"selector claim must survive");
        }
        let mut replaced=owners;replaced[7].as_mut().unwrap().entry+=8;
        assert!(capture(extras,&replaced,&kinds,facts,8).is_none());
        let mut aliased=owners;aliased[7].as_mut().unwrap().alias=1;
        assert!(capture(extras,&aliased,&kinds,facts,8).is_none());
        kinds[7]=Some(0x48);
        assert!(capture(extras,&owners,&kinds,ExtraFacts {entry_kind:0x48,object_kind:0x48,..facts},8).is_none(),
            "an existing Mii needs its committed lifetime proof, not ordinary recapture");
    }

    fn outgoing_object_uses_old_header_before_native_publication() {
        // Retained combined06 #36/#38/#39: outgoing Luigi, FE already Gunner,
        // O still [owner1, Luigi, -1, -1, 1], index0 and zero tokens. Addresses normalized.
        let before = rows()[OWNER].unwrap();
        let mut descriptor = [0u8; 0x70];
        descriptor[0x18..0x1c].copy_from_slice(&9u32.to_le_bytes());
        descriptor[0x1c..0x20].copy_from_slice(&u32::MAX.to_le_bytes());
        descriptor[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
        descriptor[0x41] = 1;
        let old_header = [1, 9, u32::MAX, u32::MAX, 1];
        assert_eq!(descriptor_header(before.selector, &descriptor), old_header);
        let accepts = |object, header, index, tokens| outgoing_object_matches(
            before, old_header, [0; 4], object, header, index, tokens);
        assert!(accepts(before.object, old_header, 0, [0; 4]));
        assert!(!accepts(before.object, [1, 0x4a, u32::MAX, u32::MAX, 1], 0, [0; 4]));
        assert!(!accepts(before.object, [0; 5], 0, [0; 4]));
        for field in 0..5 {
            let mut changed = old_header;
            changed[field] ^= 1;
            assert!(!accepts(before.object, changed, 0, [0; 4]));
        }
        for object in [0, rows()[0].unwrap().object, before.object + 8] {
            assert!(!accepts(object, old_header, 0, [0; 4]));
        }
        assert!(!accepts(before.object, old_header, 1, [0; 4]));
        for field in 0..4 {
            let mut changed = [0; 4];
            changed[field] = 1;
            assert!(!accepts(before.object, old_header, 0, changed));
        }
        // Nonzero value-only variant checks token order/index1, not native route qualification.
        let saved = Owner {index: 1, ..before};
        descriptor[0x18..0x1c].copy_from_slice(&0x4au32.to_le_bytes());
        let saved_header = descriptor_header(saved.selector, &descriptor);
        assert_eq!(saved_header, [1, 0x4a, u32::MAX, u32::MAX, 1]);
        assert!(outgoing_object_matches(saved, saved_header, [68, 0, 4, 37],
            saved.object, [1, 0x4a, u32::MAX, u32::MAX, 1], 1, [68, 0, 4, 37]));
        assert!(!outgoing_object_matches(saved, saved_header, [68, 0, 4, 37],
            saved.object, [1, 0x4a, u32::MAX, u32::MAX, 1], 1, [68, 0, 37, 4]));
    }

    fn original_alias_does_not_license_destination_or_inactive_selector_alias() {
        let original = rows();
        assert_eq!(assignment(&original), Some([Some(0),Some(2),Some(3),None,None,None,None,None]));
        let mut saved = original; saved[1].as_mut().unwrap().index = 1;
        assert_eq!(assignment(&saved), assignment(&original));
        for id in [0, 2, 3] {
            let mut changed = original; changed[id].as_mut().unwrap().index = 1;
            assert_eq!(assignment(&changed), None);
            changed = original; changed[id].as_mut().unwrap().selector = 1;
            assert_eq!(assignment(&changed), None);
        }
        let mut changed = original; changed[2] = None;
        assert_eq!(assignment(&changed), None);
        changed = original; changed[1].as_mut().unwrap().object = changed[0].unwrap().object;
        assert_eq!(assignment(&changed), None);
        changed = original; changed[1].as_mut().unwrap().alias = 1;
        assert_eq!(assignment(&changed), None);
    }

    fn staged_cpu_preserves_native_role_and_original_return_is_byte_exact() {
        use crate::engine::mii::inventory::{build_snapshot,MiiClass,RawSlot,TaggedId};
        let info=[0u8;88];
        let catalog=build_snapshot(vec![RawSlot {index:0,slot_id:2622,name_len:1,
            name_units:[65,0,0,0,0,0,0,0,0,0],colour:4,voice:37,specials:[1,0,0,0],
            tag_outfit:TaggedId(0x1300291bfef7a5f7),tag_hat:TaggedId(0x5a00000fa9efa5ea),
            tag_chara:TaggedId(0xc100000000000000|MiiClass::Swordfighter.chara_hash40()),charinfo:info}]);
        let saved=Configuration::from_saved(&catalog.configs[0]).unwrap();
        let mut original=[0xa5;0x70];
        original[4..8].copy_from_slice(&1u32.to_le_bytes());
        original[0x18..0x1c].copy_from_slice(&9u32.to_le_bytes());
        original[0x46..0x48].copy_from_slice(&0u16.to_le_bytes());
        let target=staged_descriptor(original,Some(saved));
        assert_eq!(&target[4..8],&original[4..8]);
        assert_eq!(&target[0x34..0x43],&original[0x34..0x43]);
        assert_eq!(&target[0x46..0x48],&1u16.to_le_bytes());
        assert_eq!(u32::from_le_bytes(target[0x14..0x18].try_into().unwrap()),1);
        assert_eq!(u32::from_le_bytes(target[0x28..0x2c].try_into().unwrap()),1);
        assert_eq!(staged_descriptor(original,None),original);
        assert_eq!(staged_descriptor(original,Some(saved)),target); // fresh reuse constructs same value
        assert!(publication_indices(0,Target::mii(saved).index(),0,1,0));
        assert!(publication_indices(1,Target::mii(saved).index(),1,1,1));
        assert!(!publication_indices(1,Target::mii(saved).index(),1,1,0));
        let original=Destination {target:Target::ORIGINAL,cursor:None};
        let planned=Destination {target:Target::mii(saved),cursor:Some(2)};
        let ticket=crate::engine::mii::ordinary_player::Ticket {epoch:9,entry:0x1100,sequence:3};
        let mut gate=crate::engine::mii::ordinary_player::Gate::new(ticket).unwrap();
        let mut live=original;
        gate.refuse();
        if gate.can_complete(ticket) {live=planned;}
        assert_eq!(live,original);
        assert_eq!(Target::ORIGINAL.publish(Some(saved)),Some(saved));
        assert_eq!(Target::ORIGINAL.publish(None),None);
        assert_eq!(Target::fighter(9).publish(Some(saved)),None);
        assert_eq!(Target::mii(saved).publish(None),Some(saved));
        assert_eq!(planned.target,Target::mii(saved));
        assert_eq!(planned.cursor,Some(2));
        assert!(supported_kind(0x49));assert!(supported_kind(0x4a));assert!(supported_kind(0x48));assert!(!supported_kind(0x47));
    }
    fn menu_capture_does_not_acquire_but_ticket_blocks_other_writers_until_exact_retirement() {
        let mut lease=Reservation::empty();
        assert!(!lease.held());
        assert!(!lease.routes(false,false)); // an unrelated menu/P1 action retains its existing path
        assert!(lease.routes(true,false)); // configured CPU request reaches this adapter
        assert!(!lease.acquire(false)); // another INSTALL user cannot be bypassed
        assert!(!lease.held());
        assert!(lease.acquire(true));
        assert!(lease.held());
        assert!(!lease.conflicts(0,0));
        assert!(lease.conflicts(1,1));
        assert!(lease.conflicts(0,1));
        assert!(lease.conflicts(1,0));
        assert!(lease.routes(false,false)); // even returned-original G1 stays reserved
        lease.retire(false);assert!(lease.held()); // unrelated entry destruction
        lease.retire(true);assert!(!lease.held());
        assert!(!lease.routes(false,false));
    }
    fn receipt_renewal_cannot_authorize_install_after_the_cpu_changes() {
        use crate::engine::mii::inventory::{build_snapshot,MiiClass,RawSlot,TaggedId};
        let catalog=build_snapshot(vec![RawSlot {index:0,slot_id:2622,name_len:1,
            name_units:[65,0,0,0,0,0,0,0,0,0],colour:4,voice:37,specials:[1,0,0,0],
            tag_outfit:TaggedId(0x1300291bfef7a5f7),tag_hat:TaggedId(0x5a00000fa9efa5ea),
            tag_chara:TaggedId(0xc100000000000000|MiiClass::Swordfighter.chara_hash40()),charinfo:[0;88]}]);
        let configuration=Configuration::from_saved(&catalog.configs[0]).unwrap();
        let selection=crate::engine::mii::replay::Selection {source:crate::engine::mii::replay::SelectionSource::NamedMacro,
            generation:7,revision:11,target:crate::engine::mii::replay::Target::mii(configuration)};
        let explicit=Request::from_replay(selection).unwrap();
        let renewal=Request::acknowledge_replay(selection).unwrap();
        assert_ne!(explicit,renewal);
        assert!(explicit.can_install());
        assert!(!renewal.can_install());
        assert_eq!(renewal.replay(),Some(selection));
        // Same queued request remains non-installing when a different CPU wins.
        assert!(!selection.target.ready(9,None));
        assert!(!renewal.can_install());
        assert!(Request::CYCLE.can_install());
        let ordinary=crate::engine::mii::replay::Selection {target:crate::engine::mii::replay::Target::fighter(9),..selection};
        assert!(Request::acknowledge_replay(ordinary).is_none());
    }


    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("created_destination_requires_registered_roots_and_preserves_every_other_record", created_destination_requires_registered_roots_and_preserves_every_other_record as fn()),
            ("outgoing_object_uses_old_header_before_native_publication", outgoing_object_uses_old_header_before_native_publication as fn()),
            ("original_alias_does_not_license_destination_or_inactive_selector_alias", original_alias_does_not_license_destination_or_inactive_selector_alias as fn()),
            ("staged_cpu_preserves_native_role_and_original_return_is_byte_exact", staged_cpu_preserves_native_role_and_original_return_is_byte_exact as fn()),
            ("menu_capture_does_not_acquire_but_ticket_blocks_other_writers_until_exact_retirement", menu_capture_does_not_acquire_but_ticket_blocks_other_writers_until_exact_retirement as fn()),
            ("receipt_renewal_cannot_authorize_install_after_the_cpu_changes", receipt_renewal_cannot_authorize_install_after_the_cpu_changes as fn()),
        ]);
    }
    }
}

#[cfg(test)]
mod helper_tests {
    lib_unit_tests! {
    use super::*;
    #[test]
    fn helper_selectors_keep_foreign_union_and_old_object_phase() {
        let rows=std::array::from_fn(|id| (id<4).then_some(Owner {
            entry:0x1000+id*0x100,row:0x2000+id*0x100,information:0x3000+id*0x100,
            object:0x4000+id*0x100,index:0,selector:id as u8,alias:0}));
        let mut descriptor=[0u8;0x70];
        descriptor[0x18..0x1c].copy_from_slice(&9u32.to_le_bytes());
        descriptor[0x1c..0x24].fill(0xff);descriptor[0x41]=1;
        for owner in [2usize,3] {
            let protected=assignment_for(owner,&rows).unwrap();
            assert_eq!(protected[..3],[Some(0),Some(1),Some(if owner==2 {3}else{2})]);
            let mut own=rows;own[owner].as_mut().unwrap().index=owner as u16;
            assert_eq!(assignment_for(owner,&own),Some(protected));
            for other in (0..4).filter(|id|*id!=owner) {
                let mut foreign=rows;foreign[other].as_mut().unwrap().index=owner as u16;
                assert!(assignment_for(owner,&foreign).is_none());
            }
            let before=rows[owner].unwrap();let header=[owner as u32,9,u32::MAX,u32::MAX,1];
            assert_eq!(descriptor_header(before.selector,&descriptor),header);
            assert!(outgoing_object_matches_for(before,header,[0;4],before.object,header,0,[0;4]));
            assert!(!outgoing_object_matches_for(before,header,[0;4],before.object+8,header,0,[0;4]));
            assert!(!outgoing_object_matches_for(before,header,[0;4],before.object,header,owner as u16,[0;4]));
            assert!(publication_indices_for(owner,0,owner as u16,0,owner as u16,0));
            assert!(!publication_indices_for(owner,0,owner as u16,0,owner as u16,owner as u16));
        }
        // Ordinary requests do not depend on Player/catalog capture, but a saved
        // CPU origin must retain its helper copies until CPU capture succeeds.
        assert!(helper_captures_ready(false,Some(9),false,false));
        assert!(!helper_captures_ready(false,None,true,true));
        assert!(!helper_captures_ready(false,Some(0x48),true,false));
        assert!(helper_captures_ready(false,Some(0x48),false,true));
        assert!(!helper_captures_ready(true,Some(9),false,true));
        assert!(!helper_captures_ready(true,Some(9),true,false));
        assert!(helper_captures_ready(true,Some(9),true,true));
        for denied in [0usize,4,7,8] {
            assert!(assignment_for(denied,&rows).is_none());
            assert!(staged_descriptor_for(denied,descriptor,None).is_none());
        }
    }
    }
}

#[cfg(test)]
mod recapture_diagnostic_tests {
    lib_unit_tests! {
    use super::*;
    
    

    fn flags(index_eq: bool) -> RecaptureFlags {
        RecaptureFlags {
            index_eq, scalars_eq: true, selector_record_eq: false, current_record_eq: true,
            captured_index: 4, live_index: 26,
        }
    }

    fn every_non_rank_offset_fits_once_per_epoch_tuple() {
        let mut expected = [0x10u8; 0x70];
        expected[0x37] = 0;
        let current = [0x11u8; 0x70];
        let diff = non_rank_descriptor_diff(&expected, &current);
        assert_eq!(diff.count, 111);
        assert!(diff.pairs[..111].iter().all(|pair| pair.0 != 0x37));
        assert_eq!(diff.pairs[0].0, 0x00);
        assert_eq!(diff.pairs[110].0, 0x6f);
        let mut buf = [0u8; RECAPTURE_DIAGNOSTIC_BYTES];
        let n = write_recapture_diagnostic(&mut buf, &diff, &flags(true)).unwrap();
        assert!(n <= RECAPTURE_DIAGNOSTIC_BYTES);
        let line = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(line.starts_with("recapture non-rank n=111 "));
        assert!(line.contains(" 36:10>11 "));
        assert!(line.contains(" 38:10>11 "));
        assert!(line.ends_with(" 6f:10>11"));
        assert!(!line.contains(" 37:"));
        assert!(write_recapture_diagnostic(&mut [0u8; 8], &diff, &flags(true)).is_none());
        let seen = Some((9u64, diff, flags(true)));
        assert!(recapture_observation_seen(seen, 9, diff, flags(true)));
        assert!(!recapture_observation_seen(None, 9, diff, flags(true)));
        assert!(!recapture_observation_seen(seen, 10, diff, flags(true)));
        assert!(!recapture_observation_seen(seen, 9, diff, flags(false)));
        let mut changed = diff;
        changed.pairs[0].2 ^= 1;
        assert!(!recapture_observation_seen(seen, 9, changed, flags(true)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("every_non_rank_offset_fits_once_per_epoch_tuple", every_non_rank_offset_fits_once_per_epoch_tuple as fn()),
        ]);
    }
    }
}

#[cfg(test)]
#[path="created_destination_tests.rs"]
mod created_destination_tests;
