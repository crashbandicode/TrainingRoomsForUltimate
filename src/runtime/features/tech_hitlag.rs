//! Live native hitlag reader. See S0-hitlag-resolver/native-audit/NATIVE-CALL-MANIFEST.json.
use super::{tech_adapter::{self, Error}, tech_hitlag_read::{self, Native, ParamPair}};
use crate::model::sites_tech_hitlag as sites;
use crate::runtime::game::{self,natives};
#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attributes: u32,
    permission: u32,
    ipc: u32,
    device: u32,
    padding: u32,
}
/// Same pinned Horizon QueryMemory ABI as custom_stages::readable.
/// Mapping does not confer lifetime: caller remains on the native game thread.
unsafe fn memory_readable(address: usize, size: usize) -> bool {
    let Some(end) = address.checked_add(size) else {
        return false;
    };
    if address < 0x10000 || address & 3 != 0 || address >> 47 != 0 {
        return false;
    }
    let mut info = MemoryInfo::default();
    let result: u64;
    core::arch::asm!("svc 0x6", inout("x0") &mut info as *mut MemoryInfo as u64 => result,
        lateout("x1") _, in("x2") address as u64, clobber_abi("C"), options(nostack));
    result as u32 == 0
        && info.permission & 1 == 1
        && info.address <= address as u64
        && info
            .address
            .checked_add(info.size)
            .is_some_and(|e| end as u64 <= e)
}
unsafe fn pointer(address: usize) -> Option<usize> {
    (address & 7 == 0 && memory_readable(address, 8)).then(|| (address as *const usize).read())
}


/// Registration data only; native execution is the adapter launch callback.
pub const PROVIDER: tech_adapter::HitlagResolver = resolve;
struct Live(usize);
impl Native for Live {
    type Context=tech_adapter::HeldContext;
    fn hitlag_held(&mut self) -> Result<Self::Context,Error> { unsafe { tech_adapter::held_context(self.0) } }
    fn hitlag_base(&self) -> usize { game::text_base() }
    fn hitlag_admitted(&self) -> bool { sites::SITES.iter().all(|s| game::granted(s.offset)) }
    fn hitlag_word(&mut self,address:usize) -> Option<usize> { unsafe { pointer(address) } }
    fn hitlag_float(&mut self,address:usize) -> Option<f32> { unsafe {
        memory_readable(address,4).then(|| (address as *const f32).read())
    } }
    fn hitlag_byte(&mut self,address:usize) -> Option<u8> { unsafe {
        memory_readable(address,1).then(|| (address as *const u8).read())
    } }
    fn hitlag_readable(&mut self,address:usize,size:usize) -> bool { unsafe { memory_readable(address,size) } }
    fn hitlag_pair(&mut self,object:usize,hash:u64) -> Option<ParamPair> { unsafe { natives::tech_hitlag_param_pair(object,hash) } }
    fn hitlag_get_float(&mut self,pair:&ParamPair) -> Option<f32> { unsafe { natives::tech_hitlag_param_float(pair) } }
    fn hitlag_get_int(&mut self,pair:&ParamPair) -> Option<i32> { unsafe { natives::tech_hitlag_param_int(pair) } }
    fn hitlag_k(&mut self,map:usize,hash:u64) -> Option<f32> { unsafe { natives::tech_hitlag_k_lookup(map,hash) } }
}
/// Separate live G.f0 query for future UI conversion. Game thread, fresh eligible P1 context.
pub unsafe fn live_multiplier(accessor: usize) -> Result<f32,Error> {
    let receipt=tech_adapter::player_context(accessor)?;
    let value=tech_hitlag_read::multiplier(&mut Live(accessor)).ok_or(Error::HitlagUnavailable)?;
    if tech_adapter::player_context(accessor)? != receipt { return Err(Error::NoPlayer); }
    Ok(value)
}
/// BASE damage, before percent-display multiplier. No cached k/G values or fallback.
pub unsafe fn resolve(accessor: usize,base_damage:f32) -> Result<i32,Error> {
    tech_hitlag_read::resolve(&mut Live(accessor),base_damage)
}
