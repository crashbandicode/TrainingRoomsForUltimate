//! Native SDK scalar slot boundary; exact owned SDK evidence: ui-input-contract.
use crate::model::scalar_tls::Tls;

pub(crate) struct NativeTls;

extern "C" {
    #[link_name = "_ZN2nn2os15AllocateTlsSlotEPNS0_7TlsSlotEPFvmE"]
    fn allocate_tls_slot(key: *mut u32, destructor: Option<unsafe extern "C" fn(usize)>) -> u32;
    #[link_name = "_ZN2nn2os11FreeTlsSlotENS0_7TlsSlotE"]
    fn free_tls_slot(key: u32);
    #[link_name = "_ZN2nn2os11GetTlsValueENS0_7TlsSlotE"]
    fn get_tls_value(key: u32) -> usize;
    #[link_name = "_ZN2nn2os11SetTlsValueENS0_7TlsSlotEm"]
    fn set_tls_value(key: u32, value: usize);
}

impl Tls for NativeTls {
    type Error = u32;
    fn allocate(&self) -> Result<u32, u32> {
        let mut key = 0;
        // SAFETY: SDK output pointer, no destructor or per-thread allocation.
        let result = unsafe { allocate_tls_slot(&mut key, None) };
        if result == 0 { Ok(key) } else { Err(result) }
    }
    fn free(&self, key: u32) {
        // SAFETY: callers roll back only successfully reserved keys,
        // before publication/installation. Published keys live for the process.
        unsafe { free_tls_slot(key) };
    }
    fn get(&self, key: u32) -> usize {
        // SAFETY: Slots supplies successfully reserved, live keys.
        unsafe { get_tls_value(key) }
    }
    fn set(&self, key: u32, value: usize) {
        // SAFETY: as get; value is stored by the SDK, never dereferenced here.
        unsafe { set_tls_value(key, value) };
    }
}
