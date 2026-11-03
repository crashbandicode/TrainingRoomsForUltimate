# binrw 0.13.4 host-tool compatibility patch

2026-11-03. The layout builder's pinned Toolbox dependency requires binrw 0.13.
RC4 layout and owned-layout fixture tests completed successfully, but their gates
failed Rust's E0365 future-incompatibility warning for the private crate alias
re-export in `binrw::prelude`.

Base: complete crates.io `binrw-0.13.4.crate`, SHA-256
`768230c3e8ce988f15f99af8d0d2c3afd0a656b88b35480fb1f924699e810e57`.
Its `.cargo_vcs_info.json` identifies upstream
`e2f91374cb98c4298d68e651de4f734e7b0f5608`. The MIT license is copied from that
revision's repository-root LICENSE, which the crate archive omitted.

`src/lib.rs` backports the exact prelude correction from upstream
[`4b5732db949925c574c3be67e57afc94af36e1c2`](https://github.com/jam1garner/binrw/commit/4b5732db949925c574c3be67e57afc94af36e1c2):
export the crate alias publicly within the prelude and re-export the attribute
macro directly from `binrw_derive`. A targeted build then exposed four
`unexpected_cfgs` warnings in the local dependency. `build.rs` now declares its
existing `nightly` and `coverage_nightly` configuration names with
`cargo:rustc-check-cfg`; it does not enable either configuration. All remaining
original crate files are unchanged. The patch is selected only by `tools/layout-builder/Cargo.toml`.
Parser behavior, package version and the derive dependency are retained.

Validation: final candidate layout all-target tests and the operator-owned stock
archive fixture, with warnings rejected. RC4 failures remain in the private gate
artifacts; final results are recorded with the candidate.
