//! Executes the production traversal against stable, host-owned intrusive lists.
//! The shim supplies fields only; native ABI qualification remains a separate gate.
//! `name` matches pinned nnsdk 0.6.2: [libc_nnsdk::c_char; 25], where
//! libc-nnsdk 0.5.0 defines c_char = u8 (including the Switch target).
extern crate self as skyline;
// The current integration pane writer references this unrelated text bound.
// These tests exercise traversal only, never the text writer or native ABI.
pub mod model { pub mod notify { pub const BODY_MAX: usize = 320; } }
pub mod nn {
    pub mod ui2d {
        #[repr(C)]
        pub struct PaneNode { pub prev: *mut PaneNode, pub next: *mut PaneNode }
        #[repr(C)]
        pub struct Pane {
            pub link: PaneNode, pub children_list: PaneNode, pub name: [u8; 25],
            pub alpha: u8, pub flags: u8, pub pos_x: f32, pub pos_y: f32,
            pub size_x: f32, pub size_y: f32, pub rot_z: f32,
        }
        impl Pane { pub fn set_visible(&mut self, visible: bool) { self.flags = u8::from(visible); } }
        pub enum PaneFlag { IsGlobalMatrixDirty = 1 }
        #[repr(C)]
        pub struct Picture { pub pane: Pane, pub vertex_colors: [[u8; 4]; 4] }
        #[repr(C)]
        pub struct TextBox { pub pane: Pane, pub text_buf_len: u16, pub text_len: u16, pub text_buf: *mut u16 }
        impl TextBox { pub fn set_color(&mut self, _: u8, _: u8, _: u8, _: u8) {} }
    }
}
#[path = "../src/runtime/ui/pane.rs"]
pub mod pane;
use nn::ui2d::Pane;

// Allocate the complete arena before linking, so no pointer can move.
fn tree(rows: &[(usize, &str)]) -> Vec<Pane> {
    let mut nodes: Vec<Pane> = rows.iter().map(|(_, name)| {
        let mut p: Pane = unsafe { core::mem::zeroed() };
        assert!(name.len() < p.name.len(), "pane name must leave a NUL terminator");
        p.name[..name.len()].copy_from_slice(name.as_bytes());
        p
    }).collect();
    for p in &mut nodes {
        let head = core::ptr::addr_of_mut!(p.children_list);
        p.children_list.prev = head;
        p.children_list.next = head;
    }
    for (i, &(parent, _)) in rows.iter().enumerate().skip(1) {
        let head = core::ptr::addr_of_mut!(nodes[parent].children_list);
        let link = core::ptr::addr_of_mut!(nodes[i].link);
        unsafe {
            (*link).prev = (*head).prev;
            (*link).next = head;
            (*(*head).prev).next = link;
            (*head).prev = link;
        }
    }
    nodes
}

#[test]
fn notice_and_strip_after_expanded_stock_are_reachable_without_larger_budget() {
    let mut rows = vec![(0, "RootPane"), (0, "stock")];
    rows.extend((0..4100).map(|_| (1, "stock_leaf")));
    rows.extend([(0, "TrModMenu"), (0, "TrModDisp0"), (0, "TrIv")]);
    let mut nodes = tree(&rows);
    let root = nodes.as_mut_ptr();
    unsafe {
        assert!(pane::find(root, b"TrModDisp0").is_null(), "original failing search");
        assert!(pane::find(root, b"TrIv").is_null());
        for name in [b"TrModMenu".as_slice(), b"TrModDisp0", b"TrIv"] {
            assert!(!pane::find_child(root, name).is_null());
        }
    }
}

#[test]
fn waiting_requires_direct_owned_host_and_containers() {
    let mut wrong_host = tree(&[(0,"RootPane"),(0,"stock"),(1,"TrModPort"),(2,"TrModDisp0")]);
    unsafe { assert!(pane::find_child(wrong_host.as_mut_ptr(), b"TrModPort").is_null()); }
    let mut nodes = tree(&[(0,"RootPane"),(0,"stock"),(1,"TrModPort"),
        (2,"TrModDisp0"),(0,"TrModPort"),(4,"TrModMenu"),(4,"TrModDisp0"),
        (6,"TrModDisp0Long"),(4,"TrIv")]);
    unsafe {
        let root = nodes.as_mut_ptr();
        let host = pane::find_child(root, b"TrModPort");
        assert_eq!(host, root.add(4));
        assert_eq!(pane::find_child(host, b"TrModDisp0"), root.add(6));
        assert_eq!(pane::find(pane::find_child(host, b"TrModDisp0"), b"TrModDisp0Long"), root.add(7));
        assert_eq!(pane::find_child(host, b"TrIv"), root.add(8));
        assert!(pane::find_child(root, b"TrModDisp0").is_null());
        assert!(pane::find_child(host, b"missing").is_null());
        assert!(pane::find_child(core::ptr::null_mut(), b"TrModPort").is_null());
    }
}

#[test]
fn cyclic_sibling_list_is_bounded() {
    let mut nodes = tree(&[(0,"RootPane"),(0,"stock")]);
    nodes[1].link.next = core::ptr::addr_of_mut!(nodes[1].link);
    unsafe { assert!(pane::find_child(nodes.as_mut_ptr(), b"TrModDisp0").is_null()); }
}

#[test]
#[ignore = "requires the operator-owned expanded UI18 topology fixture in UI_PANE_FIXTURE"]
fn actual_expanded_training_and_waiting_fixture() {
    let dir = std::env::var("UI_PANE_FIXTURE").expect("set UI_PANE_FIXTURE to the private fixture directory");
    for (kind, expected) in [("training",4493),("waiting",1565)] {
        let data = std::fs::read_to_string(format!("{dir}/{kind}.tsv")).unwrap();
        let rows: Vec<_> = data.lines().map(|line| {
            let (parent,name) = line.split_once('\t').unwrap();
            (parent.parse::<usize>().unwrap(),name)
        }).collect();
        assert_eq!(rows.len(),expected);
        let mut nodes = tree(&rows);
        unsafe {
            let root = nodes.as_mut_ptr();
            let host = if kind == "waiting" { pane::find_child(root,b"TrModPort") } else { root };
            for name in [b"TrModMenu".as_slice(),b"TrModDisp0",b"TrIv"] {
                assert!(!pane::find_child(host,name).is_null(), "{kind} {name:?}");
            }
            let notice = pane::find_child(host,b"TrModDisp0");
            for name in [b"TrModDisp0Header".as_slice(),b"TrModDisp0Txt",b"TrModDisp0Long"] {
                assert!(!pane::find(notice,name).is_null(), "{kind} {name:?}");
            }
            assert!(!pane::find(pane::find_child(host,b"TrModMenu"),b"tr_rl_00").is_null());
            assert!(!pane::find(pane::find_child(host,b"TrIv"),b"TrIvRun00").is_null());
            if kind == "training" {
                assert!(pane::find(root,b"TrModDisp0").is_null());
                assert!(pane::find(root,b"TrIv").is_null());
                assert!(!pane::find(root,b"TrModMenu").is_null());
            }
        }
    }
}

#[test]
fn binding_width_and_unsigned_name_bytes_are_searchable() {
    let mut nodes = tree(&[(0, "RootPane"), (0, "abcdefghijklmnopqrstuvwx"), (0, "é")]);
    assert_eq!(nodes[1].name.len(), 25);
    assert_eq!(nodes[1].name[24], 0);
    assert_eq!(nodes[2].name[0], 0xc3);
    unsafe {
        let root = nodes.as_mut_ptr();
        assert_eq!(pane::find_child(root, b"abcdefghijklmnopqrstuvwx"), root.add(1));
        assert_eq!(pane::find(root, "é".as_bytes()), root.add(2));
        assert!(pane::find_child(root, b"abcdefghijklmnopqrstuvw").is_null());
    }
}

#[test]
#[should_panic(expected = "pane name must leave a NUL terminator")]
fn host_tree_refuses_name_without_binding_terminator() {
    let _ = tree(&[(0, "abcdefghijklmnopqrstuvwxy")]);
}