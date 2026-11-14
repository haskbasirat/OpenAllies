//! Original `52CED0(other)` on a path (called by the element move result
//! `530F40`): the kind-1 node of this path that lies as far from its tail as
//! the other path's kind-1 count says.
//!
//! With `k = other.kind1_count - 1` (signed, wrapping): when `k` is not
//! below this path's kind-1 count the result is null; otherwise the nodes
//! are walked from the tail towards the head, each kind-1 node decrementing
//! `k`, and the node that makes it negative is returned (null when none
//! does).
use crate::OwnedPath;

/// Complete `52CED0`: the index (from the head) of the matching node in
/// `path.nodes`, `None` for null.
pub fn matching_kind1_node(path: &OwnedPath, other_kind1_count: i32) -> Option<usize> {
    let mut k = other_kind1_count.wrapping_sub(1);
    if k >= path.kind1_count {
        return None;
    }
    for (index, node) in path.nodes.iter().enumerate().rev() {
        if node.words[4] == 1 {
            k = k.wrapping_sub(1);
            if k < 0 {
                return Some(index);
            }
        }
    }
    None
}
