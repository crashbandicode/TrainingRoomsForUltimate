//! Approved M3 read-only reader dependencies, proposal3979f5d6 (2026-11-15).
//! Existing Site guard policy applies; these rows install no hooks or patches.

use super::guard::SeamKind;
use super::sites::{Feature, Site};

pub static SITES: [Site; 17] = include!(concat!(env!("OUT_DIR"), "/tables/sites_mashout.SITES.rs"));

#[cfg(test)]
#[path = "../../tests/common/mod.rs"]
mod owned;

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::{owned, Feature, SeamKind, SITES};

        #[test]
        fn mashout_reader_rows_match_approved_fixture() {
            assert_eq!(SITES.len(), APPROVED.len());
            for (site, (offset, called, hex)) in SITES.iter().zip(APPROVED) {
                assert_eq!(site.offset, offset);
                assert_eq!(site.feature, Feature::Mashout);
                assert_eq!(site.kind, if called { SeamKind::Called } else { SeamKind::Relied });
                let bytes: Vec<u8> = hex.as_bytes().chunks_exact(2).map(|pair| {
                    u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap()
                }).collect();
                assert_eq!(site.guard.as_slice(), bytes);
                assert_eq!(SITES.iter().filter(|other| other.name == site.name).count(), 1);
                assert_ne!(site.offset, 0x3f89b0);
                assert_ne!(site.offset, 0x2001080);
            }
        }

        #[test]
        fn mashout_reader_rows_match_owned_elf() {
            let Some(owned) = owned::owned_main("mashout_reader_rows_match_owned_elf") else { return };
            for site in &SITES {
                assert_eq!(owned.window(site.offset), site.guard, "{}", site.name);
            }
        }

        // Frozen independently of the generated TSV: approved JSON SHA256
        // 3979f5d6ae401bba68851bc7038b3ceaf95420875c89d6358bebc6a45868f7a2.
        const APPROVED: [(usize, bool, &str); 17] = [
            (0x6b9160, false, "0b0c40f9ea0b1e32497c40934c2c2a9be803212af30300aa1f1d0072880000128a490091e0000054c30000374d014039bf01216b61000054830140bd6028201e"),
            (0x6b91a0, false, "800100bdec0b1e32292d0c9b210500bd220900bd3f210079e90300324101003969d2043968da0439e85f02f008a142f97f2a01b9000140f9080040f9080940f9"),
            (0x3f89d4, false, "080840f9f30300aa1fd404391f050071ab100054e803271ef40301aaf7031faaf5031faaf6031f2a187d4092d9058052fa030032680e40f91b01178b600340bd"),
            (0x3f8a14, false, "0820201e6d0b0054680240f9085141f9e00313aae103152a00013fd66a4fc039480500716b000054ea03082a684f00396853c039090500716b000054e803092a"),
            (0x3f8a54, false, "69530039695bc0392b0500716b000054e9030b2a6b5b00398106c03d0240211e6d4340396e4740392020221e65000054ec1f0032050000142020201eec030d2a"),
            (0x3f8a94, false, "4d000054ec03003221040c5e2020221e65000054ef1f0032050000142020201eef030e2a4d000054ef0300329f010d6beb031f2ae0014e7aa00100544a1d0013"),
            (0x3f8ad4, false, "5f0100714c010054600b40bd610340bd2038201e685f4039eb0300326c4300396f47003968530039600300bd8c8a40b98a01190a8a250933aa020034291d0013"),
            (0x3f8b14, false, "3f0500718b000054695740392a010a6ae0010054081d00131f0100718c0100546007412d620340bd685f4039684f0039686340396a5700390008211e4038201e"),
            (0x3f8b54, false, "eb030032685b0039600300bd6007402d0038211e0820201e008c281ee8979f1ad602082a600300bd4b0000347ad60439b5060091f77200911f0315ebc1f3ff54"),
            (0x3f915c, false, "08884339f40301aaf30300aa1540009168010034a80640f9681600f9a80240f9692a44a9681200f9687240b9687600b9687e40b9692a05a90a000014682643a9"),
            (0x3f919c, false, "6a2e46a9682602a9697a40b9e8030032688a0339688640b96a2e05a9697600b9688200b9880240f9080d40f9e00314aa00013fd6880240f9081140f9e00314aa"),
            (0x3f91dc, false, "00013fd668e640b9680000360140211e2004046e8800083601040c5e2140211e20040c6ef60313aac10ec43cc0029d3c00e4006f2040006e0040006ee00314aa"),
            (0x3f921c, false, "c002803d880240f9091d40f9e823009120013fd6e007412d0020211e00cc211ec03200bd880240f9081940f9e00314aa00013fd6c03e00b9a0008836c8a24239"),
            (0x3f925c, false, "6800003400001f32607e00b920011837607240bd0820201e0001005400001d32607e00b9688240b9c000203709000014e81b0932687200b9688240b9a0002036"),
            (0x3f929c, false, "88002037294080520000092a607e00b9800028366800283708791f12688200b90901004a2a01000a2801080a6a22112968f242396006c03d08102a1e09050011"),
            (0x6b9324, false, "618e40f975820091e00315aa84fff49768d24439080800347400003668de4439a8070035e00313aae10315aa98fdf49768da4439c802003468d6443988020034"),
            (0x6ba1e0, true, "080440f9002940f9080040f9033941f9210a8ed281bdbcf2e2fb90d262c9a2f2c100c0f24202c0f260001fd600000000008c40f9800000b4080040f9022140f9"),
        ];
    }
}
