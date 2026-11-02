//! Reads and writes of the game's `nn::ui2d` panes, through nnsdk's bindings (MIT, clean-room reverse
//! engineered; the pane structures are the library's). A pane is found by name in the pane tree; text
//! is written as UTF-16 straight into the text box's buffer; colours, visibility and positions are plain
//! field writes. Everything here runs on the UI job thread inside the layout tick.

use core::mem::offset_of;
use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::nn::ui2d::{Pane, PaneFlag, PaneNode, Picture, TextBox};

/// The most panes a walk visits (a guard against a corrupted list).
const WALK_LIMIT: usize = 4096;

unsafe fn pane_of_node(node: *mut PaneNode) -> *mut Pane {
    (node as *mut u8).sub(offset_of!(Pane, link)) as *mut Pane
}

unsafe fn name_is(pane: *const Pane, name: &[u8]) -> bool {
    let raw = &(*pane).name;
    let mut i = 0;
    while i < raw.len() && raw[i] != 0 {
        if i >= name.len() || raw[i] as u8 != name[i] {
            return false;
        }
        i += 1;
    }
    i == name.len()
}

/// The pane called `name` below `root` (depth first), or null.
pub unsafe fn find(root: *mut Pane, name: &[u8]) -> *mut Pane {
    let mut budget = WALK_LIMIT;
    find_in(root, name, &mut budget)
}

unsafe fn find_in(pane: *mut Pane, name: &[u8], budget: &mut usize) -> *mut Pane {
    if pane.is_null() || *budget == 0 {
        return core::ptr::null_mut();
    }
    *budget -= 1;
    if name_is(pane, name) {
        return pane;
    }
    let head = core::ptr::addr_of_mut!((*pane).children_list);
    let mut node = (*head).next;
    while !node.is_null() && node != head && *budget > 0 {
        let found = find_in(pane_of_node(node), name, budget);
        if !found.is_null() {
            return found;
        }
        node = (*node).next;
    }
    core::ptr::null_mut()
}

/// How many panes live below `pane` (itself included), for the diagnostics.
pub unsafe fn count(pane: *mut Pane) -> usize {
    let mut budget = WALK_LIMIT;
    let mut n = 0;
    count_in(pane, &mut budget, &mut n);
    n
}

unsafe fn count_in(pane: *mut Pane, budget: &mut usize, n: &mut usize) {
    if pane.is_null() || *budget == 0 {
        return;
    }
    *budget -= 1;
    *n += 1;
    let head = core::ptr::addr_of_mut!((*pane).children_list);
    let mut node = (*head).next;
    while !node.is_null() && node != head && *budget > 0 {
        count_in(pane_of_node(node), budget, n);
        node = (*node).next;
    }
}

pub unsafe fn set_visible(pane: *mut Pane, visible: bool) {
    (*pane).set_visible(visible);
}

pub unsafe fn set_alpha(pane: *mut Pane, alpha: u8) {
    (*pane).alpha = alpha;
}

/// Move a pane (centre, relative to its parent) and mark its matrix stale.
pub unsafe fn set_pos(pane: *mut Pane, x: f32, y: f32) {
    if (*pane).pos_x != x || (*pane).pos_y != y {
        (*pane).pos_x = x;
        (*pane).pos_y = y;
        (*pane).flags |= 1 << PaneFlag::IsGlobalMatrixDirty as u8;
    }
}

pub unsafe fn set_width(pane: *mut Pane, width: f32) {
    if (*pane).size_x != width {
        (*pane).size_x = width;
        (*pane).flags |= 1 << PaneFlag::IsGlobalMatrixDirty as u8;
    }
}

pub unsafe fn set_height(pane: *mut Pane, height: f32) {
    if (*pane).size_y != height {
        (*pane).size_y = height;
        (*pane).flags |= 1 << PaneFlag::IsGlobalMatrixDirty as u8;
    }
}

/// Turn a pane about its centre (degrees, counter-clockwise).
pub unsafe fn set_rot_z(pane: *mut Pane, degrees: f32) {
    if (*pane).rot_z != degrees {
        (*pane).rot_z = degrees;
        (*pane).flags |= 1 << PaneFlag::IsGlobalMatrixDirty as u8;
    }
}

/// The four vertex colours of a picture pane.
pub unsafe fn set_rect_color(pane: *mut Pane, color: [u8; 4]) {
    let picture = pane as *mut Picture;
    (*picture).vertex_colors = [color; 4];
}

pub unsafe fn text_capacity(pane: *mut Pane) -> usize {
    usize::from((*(pane as *mut TextBox)).text_buf_len)
}

/// Set a text box's text through the game's own pane text setter (it grows the buffer to fit, then sets the
/// text and marks the box for layout). Returns whether anything changed; the text is cut to `MAX_UNITS`.
pub unsafe fn set_text(pane: *mut Pane, units: &[u16]) -> bool {
    let tb = pane as *mut TextBox;
    let len = units.len().min(MAX_UNITS);
    let buffer = (*tb).text_buf as *const u16;
    if !buffer.is_null() && usize::from((*tb).text_len) == len && core::slice::from_raw_parts(buffer, len) == &units[..len] {
        return false;
    }
    let setter = TEXT_SETTER.load(Ordering::Acquire);
    if setter == 0 {
        return false;
    }
    // A NUL-terminated copy on the stack: the setter copies it into the box's own buffer.
    let mut staged = [0u16; MAX_UNITS + 1];
    staged[..len].copy_from_slice(&units[..len]);
    let set: extern "C" fn(*mut Pane, *const u16, i32, u32) = core::mem::transmute::<usize, extern "C" fn(*mut Pane, *const u16, i32, u32)>(setter);
    set(pane, staged.as_ptr(), len as i32, 0);
    true
}

/// The text setter's address, set at install when its 64-byte guard passed.
pub fn set_text_setter(address: usize) {
    TEXT_SETTER.store(address, Ordering::Release);
}

/// Longest text the menu writes to one box (the frame's longest line).
pub const MAX_UNITS: usize = 160;
static TEXT_SETTER: AtomicUsize = AtomicUsize::new(0);
pub unsafe fn set_text_color(pane: *mut Pane, color: [u8; 4]) {
    (*(pane as *mut TextBox)).set_color(color[0], color[1], color[2], color[3]);
}

pub unsafe fn pos_y(pane: *const Pane) -> f32 {
    (*pane).pos_y
}

pub unsafe fn pos_x(pane: *const Pane) -> f32 {
    (*pane).pos_x
}