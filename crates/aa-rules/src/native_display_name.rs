//! ThingData `+10` (ActorData `+10`) display name exactly as the original
//! stores it: ThingData `47D960` (`+10 = 5B5620(node, "name")`, `+14 = 1`)
//! and TGC_DataFileNode::GetLocalized `5B5620`:
//! * the field is looked up (`5B4750`); absent -> TGC_DataFileNodeException
//!   "Node doesn't contain field." (a value-less declaration inherited from
//!   a strict template already fails at parse time: "Field 'name' missing
//!   from node using strict template");
//! * a stored value starting with `#` is localized: `Right(len - 1)`
//!   (`41D390`) through `4B9F40([B36544]; key, B43B48)`, the static empty
//!   CString being the default for a missing key;
//! * any other stored value is kept verbatim.
//! The stored value is the parser's text (quotes removed, one leading `@`
//! removed: `ResolvedNode::localized`), widened byte by byte to UTF-16 as the
//! original line source does (857210). Oracle:
//! research/phase8-saved-components/native-display-name.json.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result},
};

/// `4B9F40` string-table lookup: the localized text for `key`, or None when
/// the table returns its default (the empty string).
pub type Lookup<'a> = dyn FnMut(&[u16]) -> Result<Option<Vec<u16>>> + 'a;

/// `5B5620(node, "name")` as stored by `47D960`.
pub fn native_stored_name(node: &ResolvedNode<'_>, lookup: &mut Lookup<'_>) -> Result<Vec<u16>> {
    let Some(stored) = node.localized("name")? else {
        // Ok(None) is either no `name` field at all or a declaration without
        // a value anywhere; `field` tells them apart (absent -> Ok(None)).
        return Err(match node.field("name", aa_numeric::Precision::Double53) {
            Ok(None) => Error::Unsupported("Node doesn't contain field. (name; 5B5620 throws)".into()),
            _ => Error::Unsupported(
                "Field 'name' missing from node using strict template (native parse exception)".into(),
            ),
        });
    };
    let units: Vec<u16> = stored.text.bytes().map(u16::from).collect();
    if units.first() != Some(&u16::from(b'#')) {
        return Ok(units);
    }
    Ok(lookup(&units[1..])?.unwrap_or_default())
}

impl ActorCatalog {
    /// ActorData `+10` of an actor definition (ThingData `47D960`).
    pub fn native_display_name(&self, id: &str, lookup: &mut Lookup<'_>) -> Result<Vec<u16>> {
        native_stored_name(&self.resolved_actor(id)?, lookup)
    }
}
