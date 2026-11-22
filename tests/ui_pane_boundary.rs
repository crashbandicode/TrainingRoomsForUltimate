//! Executes the production traversal against stable, host-owned intrusive lists.
//! The shim supplies fields only; native ABI qualification remains a separate gate.
//! `name` matches pinned nnsdk 0.6.2: [libc_nnsdk::c_char; 25], where
//! libc-nnsdk 0.5.0 defines c_char = u8 (including the Switch target).
use crate::nn::ui2d::Pane;
use crate::pane;

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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("binding_width_and_unsigned_name_bytes_are_searchable", binding_width_and_unsigned_name_bytes_are_searchable as fn()),
    ]);
}
