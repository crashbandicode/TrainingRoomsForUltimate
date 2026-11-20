#![allow(dead_code)]

mod sites_mashout {
    #[derive(PartialEq)]
    pub enum Feature {
        Mashout,
    }
    pub enum HookForm {
        VtableSlot,
    }
    pub enum Placement {
        Data,
    }
    pub enum SeamKind {
        Relied,
        Called,
        Hook { form: HookForm, placement: Placement },
    }
    pub struct Site {
        name: &'static str,
        feature: Feature,
        pub offset: usize,
        kind: SeamKind,
        pub guard: [u8; 64],
        note: &'static str,
    }
    pub static SITES: &[Site] = &/* GENERATED_SITES */ [];
    // FLOOR_SITE_SELECTION
}

mod game {
    use std::cell::Cell;

    thread_local! {
        pub static REJECTED_GRANT: Cell<usize> = const { Cell::new(0) };
        pub static CHANGED_WINDOW: Cell<usize> = const { Cell::new(0) };
    }

    pub fn granted(offset: usize) -> bool {
        REJECTED_GRANT.get() != offset
    }

    pub fn read_window(offset: usize) -> [u8; 64] {
        let mut guard = super::sites_mashout::SITES
            .iter()
            .find(|site| site.offset == offset)
            .unwrap()
            .guard;
        if CHANGED_WINDOW.get() == offset {
            guard[0] ^= 1;
        }
        guard
    }

    pub fn text_base() -> usize {
        panic!("unguarded floor ray reached native memory");
    }
}

mod entry {
    #[derive(Debug, PartialEq)]
    pub struct PlacementFloor {
        pub ends: [[f32; 2]; 2],
        pub stationary: bool,
        pub flags: u32,
    }
}

use sites_mashout::{FLOOR_RAY, FLOOR_RAY_IMPL};
// FLOOR_RAY_READER

#[test]
fn requires_each_new_layout_before_accessing_native_memory() {
    for offset in [0x15cbc40, 0x15cbc10] {
        game::REJECTED_GRANT.set(offset);
        assert!(unsafe { FloorRay::new(0) }.is_none());
        game::REJECTED_GRANT.set(0);

        game::CHANGED_WINDOW.set(offset);
        assert!(unsafe { FloorRay::new(0) }.is_none());
        game::CHANGED_WINDOW.set(0);
    }
}

unsafe extern "C" fn cast_to_line(
    line: usize,
    _: *const RayPoint,
    _: *const RayPoint,
    _: *mut RayPoint,
    _: bool,
) -> usize {
    line
}

#[test]
fn passable_line_uses_platform_rules_even_when_floor_marked() {
    #[repr(C, align(8))]
    struct Line([u8; 0xc8]);

    let mut line = Line([0; 0xc8]);
    let ends = [[-6.0, 0.0], [6.0, 0.0]];
    let vertices = ends.map(|[x, y]| [0., 0., 0., 0., x, y, 0., 0., x, y]);
    let address = line.0.as_mut_ptr();
    let ray = FloorRay { accessor: address as usize, cast: cast_to_line };
    unsafe {
        address.add(0x88).cast::<[usize; 2]>()
            .write(std::array::from_fn(|i| vertices[i].as_ptr() as usize));
        for (flags, normal_y, platforms, accepted) in [
            (0x2000_u32, 1.0_f32, true, true),
            (0x2800, 1.0, true, true),
            (0x2800, 1.0, false, false),
            (0x2800, -1.0, true, false),
            (0x2800, f32::NAN, true, false),
            (0x2800, f32::INFINITY, true, false),
            (0x2000, f32::INFINITY, true, false),
            (0x2900, 1.0, true, false),
            (0x2a00, 1.0, true, false),
            (0x0800, 1.0, false, true),
            (0x0800, f32::NAN, false, false),
        ] {
            address.add(0xc4).cast::<u32>().write(flags);
            address.add(0xa4).cast::<f32>().write(normal_y);
            let expected = accepted.then_some(entry::PlacementFloor {
                ends,
                stationary: true,
                flags,
            });
            assert_eq!(
                ray.floor(0.0, [1.0, -1.0], platforms),
                expected,
                "flags={flags:#06x} normal_y={normal_y} platforms={platforms}",
            );
        }
    }
}
