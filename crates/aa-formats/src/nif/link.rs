//! The object relations `LinkObject` builds, as native list order.
//!
//! * NiObjectNET::LinkObject 86B0E0: controller head (+0x10); below 5.0.0.11
//!   one extra data, otherwise AddExtraData 86AE60 for each non-NULL link.
//! * NiAVObject::LinkObject 86F3A0: every property link (NULL included) is a
//!   list node; below 4.1.0.8 appended at the tail, otherwise inserted at the
//!   head, so the list runs in reverse link order. From 5.0.0.19 the
//!   collision object.
//! * NiNode::LinkObject 8678D0: child i is SetAt(i) 867C40 (NULL slots
//!   kept), which re-parents the child (86EA10); each non-NULL effect is
//!   appended (below 4.1.0.8) or inserted at the head.
//! * NiTimeController::LinkObject 89F160: next controller (+0x38), target.
//!
//! Extra-data naming and ordering (86AE60 / 86AC80) are not ported.
use super::{is_kind_of, NifError, NifFile, NifObject, NULL_LINK};

/// One NiNode child array (NiTObjectArray +0xC4: data +0xC8, used size +0xD0).
#[derive(Default)]
struct Children {
    data: Vec<Option<usize>>,
    size: usize,
}

/// The child arrays and parent pointers NiNode::LinkObject 8678D0 leaves,
/// running every NiNode's links in LinkObject (object) order. For slot i,
/// SetAt 867C40: below the used size the previous child's parent is cleared;
/// the new child is re-parented by NiAVObject::AttachParent 86EA10, whose
/// current parent first detaches it (NiNode::DetachChild 866DF0: the first
/// slot holding it, parent cleared, RemoveAt 866B40 stores NULL and shrinks
/// the used size by one when it was the last slot); then the slot is stored
/// (882300; from the used size on, the used size becomes i + 1).
pub(crate) fn link_children(objects: &[NifObject]) -> Result<(Vec<Vec<Option<usize>>>, Vec<Option<usize>>), NifError> {
    let mut arrays: Vec<Children> = (0..objects.len()).map(|_| Children::default()).collect();
    let mut parents: Vec<Option<usize>> = vec![None; objects.len()];
    for (node, o) in objects.iter().enumerate() {
        if !is_kind_of(o.class, "NiNode") {
            continue;
        }
        let links = o.fields.links("children").ok_or(NifError::Unsupported("8678D0 child links"))?;
        for (slot, &link) in links.iter().enumerate() {
            let child = (link != NULL_LINK).then_some(link as usize);
            if let Some(c) = child {
                if !is_kind_of(objects[c].class, "NiAVObject") {
                    return Err(NifError::Unsupported("867C40 child link to a non-NiAVObject"));
                }
            }
            if slot < arrays[node].size {
                if let Some(old) = arrays[node].data[slot] {
                    parents[old] = None;
                }
            }
            if let Some(c) = child {
                if let Some(previous) = parents[c] {
                    let a = &mut arrays[previous];
                    if let Some(j) = (0..a.size).find(|&j| a.data[j] == Some(c)) {
                        parents[c] = None;
                        a.data[j] = None;
                        if j + 1 == a.size {
                            a.size -= 1;
                        }
                    }
                }
                parents[c] = Some(node);
            }
            let a = &mut arrays[node];
            if a.data.len() <= slot {
                a.data.resize(slot + 1, None);
            }
            a.data[slot] = child;
            if slot >= a.size {
                a.size = slot + 1;
            }
        }
    }
    let children = arrays
        .into_iter()
        .map(|mut a| {
            a.data.truncate(a.size);
            a.data
        })
        .collect();
    Ok((children, parents))
}

impl NifFile {
    fn link_index(&self, link: u32) -> Option<usize> {
        (link != NULL_LINK).then_some(link as usize)
    }

    /// NiAVObject property list, head first, as object indices (`None` for
    /// a NULL link, which the native list keeps as a node with NULL data).
    pub fn av_properties(&self, object: usize) -> Result<Vec<Option<usize>>, NifError> {
        let o = &self.objects[object];
        if !is_kind_of(o.class, "NiAVObject") {
            return Err(NifError::Unsupported("property list of a non-NiAVObject"));
        }
        let links = o.fields.links("properties").ok_or(NifError::Unsupported("86F3A0 property links"))?;
        let mut list: Vec<Option<usize>> = links.iter().map(|&l| self.link_index(l)).collect();
        if self.version >= 0x0401_0008 {
            list.reverse();
        }
        Ok(list)
    }

    /// NiNode children by slot after every LinkObject (NULL slots kept,
    /// re-parented children removed from their earlier parent).
    pub fn node_children(&self, object: usize) -> Result<Vec<Option<usize>>, NifError> {
        Ok(self.children.get(object).cloned().unwrap_or_default())
    }

    /// The NiAVObject parent (+0x24) after every LinkObject.
    pub fn parent(&self, object: usize) -> Option<usize> {
        self.parents.get(object).copied().flatten()
    }

    /// NiNode dynamic-effect list, head first; NULL links are not inserted.
    pub fn node_effects(&self, object: usize) -> Result<Vec<usize>, NifError> {
        let o = &self.objects[object];
        if !is_kind_of(o.class, "NiNode") {
            return Ok(Vec::new());
        }
        let links = o.fields.links("effects").ok_or(NifError::Unsupported("8678D0 effect links"))?;
        let mut list: Vec<usize> = links.iter().filter_map(|&l| self.link_index(l)).collect();
        if self.version >= 0x0401_0008 {
            list.reverse();
        }
        Ok(list)
    }

    /// The controller chain: NiObjectNET +0x10, then each controller's next
    /// (+0x38). A repeated controller is a cycle the native list would loop on.
    pub fn controllers(&self, object: usize) -> Result<Vec<usize>, NifError> {
        let o = &self.objects[object];
        let mut link = o.fields.link("controller").ok_or(NifError::Unsupported("86B0E0 controller link"))?;
        let mut out = Vec::new();
        while let Some(i) = self.link_index(link) {
            if out.contains(&i) {
                return Err(NifError::OutOfRange("cyclic controller chain"));
            }
            let c = &self.objects[i];
            if !is_kind_of(c.class, "NiTimeController") {
                return Err(NifError::Unsupported("controller link to a non-NiTimeController"));
            }
            out.push(i);
            link = c.fields.link("next").ok_or(NifError::Unsupported("89F160 next controller"))?;
        }
        Ok(out)
    }

    /// The object a single-link field resolves to.
    pub fn linked(&self, object: usize, field: &str) -> Option<usize> {
        self.objects[object].fields.link(field).and_then(|l| self.link_index(l))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nif::{Fields, Value};

    fn node(children: &[u32]) -> NifObject {
        let mut fields = Fields::default();
        fields.put("children", Value::Links(children.to_vec()));
        NifObject { class: "NiNode", stream_name: b"NiNode".to_vec(), fields }
    }

    fn leaf() -> NifObject {
        NifObject { class: "NiTriShape", stream_name: b"NiTriShape".to_vec(), fields: Fields::default() }
    }

    #[test]
    fn a_child_linked_twice_moves_to_the_later_parent() {
        // Detached from the last slot: the earlier parent's used size shrinks.
        let objects = vec![node(&[1, 2]), node(&[3]), node(&[3, 4]), leaf(), leaf()];
        let (children, parents) = link_children(&objects).unwrap();
        assert_eq!(children[1], Vec::<Option<usize>>::new());
        assert_eq!(children[2], [Some(3), Some(4)]);
        assert_eq!((parents[1], parents[2], parents[3], parents[4]), (Some(0), Some(0), Some(2), Some(2)));
        // Detached from an inner slot: the slot keeps a NULL.
        let objects = vec![node(&[2, 3]), node(&[2]), leaf(), leaf()];
        let (children, parents) = link_children(&objects).unwrap();
        assert_eq!(children[0], [None, Some(3)]);
        assert_eq!(children[1], [Some(2)]);
        assert_eq!(parents[2], Some(1));
        // Listed twice by one node: the earlier slot is cleared; NULL links keep slots.
        let objects = vec![node(&[1, NULL_LINK, 1]), leaf()];
        let (children, parents) = link_children(&objects).unwrap();
        assert_eq!(children[0], [None, None, Some(1)]);
        assert_eq!(parents[1], Some(0));
    }
}
