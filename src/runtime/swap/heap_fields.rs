//! The install line's heap fields (WP-H): `heap_allocations=<calls>` with the diag meter
//! (`heap_metrics`, feature `plugin-heap-meter`), `heap_allocations=unmetered` without it
//! (shipped builds have no counting allocator). WP-H's per-hook depth guard was never taken up
//! by a callback and the operator ruled the playbook's no-heap rule N/A for this plugin
//! (2026-10-31, WORKAROUND-DECISIONS), so the guard and its `in_hook=` field are gone.

/// The install line's heap fields.
pub(crate) struct HeapFields;

#[cfg(feature = "plugin-heap-meter")]
impl core::fmt::Display for HeapFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", crate::runtime::swap::heap_metrics::snapshot())
    }
}

#[cfg(not(feature = "plugin-heap-meter"))]
impl core::fmt::Display for HeapFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("heap_allocations=unmetered")
    }
}
