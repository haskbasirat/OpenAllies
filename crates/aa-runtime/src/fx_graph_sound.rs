//!42ADD0/42C3C0 PLAY_SFX definition compilation. This does not play audio.
use super::{Bound, Draft, Inputs, Node, NodeId, Reference, Result};
use aa_rules::resources::Provenance;
use std::collections::BTreeMap;

///42ADD0 writes only the common constructor fields. +3C/+40/+44 are
/// genuinely unwritten until their loader stores; CreateVFX's +68 is absent.
pub(super) fn construct_play_sfx(source: Vec<Provenance>) -> Node {
    Node {
        words: BTreeMap::from([
            (0x18, 0),
            (0x1c, 0),
            (0x24, 0),
            (0x28, 0),
            (0x2c, 0),
            (0x30, u32::MAX),
            (0x34, 42),
        ]),
        bytes: BTreeMap::from([(4, 0), (0x38, 0)]),
        references: BTreeMap::from([(0x20, None)]),
        texts: BTreeMap::new(),
        textures: BTreeMap::new(),
        immediate: Vec::new(),
        chain: Vec::new(),
        source,
        complete: false,
        node_links: BTreeMap::new(),
    }
}

///42C3C0's derived body, after the same actual42C1B0 base loader returns.
/// A failed required input preserves every preceding committed store.
pub(super) fn load_play_sfx<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    let looping = input.flag(source, "looping")?;
    draft.byte(id, 0x40, u8::from(looping));
    let audio = input.text(source, "audio_ids")?;
    let reference = input.find("Audio", &audio)?;
    let missing = reference.is_none();
    draft.reference(id, 0x44, reference);
    if missing {
        return Err(format!("Could not find audio data: {audio}"));
    }
    if looping {
        //404ED0 warning is recorded before the native correction store.
        draft.warnings.push(id);
        draft.byte(id, 0x40, 0);
    }
    let at = input.text(source, "at")?;
    let at = input.enum_value("AT", &at)?;
    draft.store(id, 0x3c, at as u32);
    if at < 0 {
        return Err(format!("Unknown AT value for creating '{audio}'"));
    }
    draft.nodes[id.0].complete = true;
    Ok(id)
}

///416370: empty IDS is null; otherwise a one-hop historical translation and
/// the actual Audio mode0 registry identify an already registered definition.
///482090/47D880 construct that manager;48D2D0 inserts translation roots.
/// The returned reference is source identity, not an invented audio resource.
pub(super) fn find_audio(bound: &Bound<'_>, ids: &str) -> Result<Option<Reference>> {
    if ids.is_empty() {
        return Ok(None);
    }
    if !ids.is_ascii() {
        return Err("Audio lookup non-ASCII case-folding domain is not proved".into());
    }
    let ids = bound.translated("Audio", ids)?;
    if !ids.is_ascii() {
        return Err("translated Audio IDS non-ASCII case-folding domain is not proved".into());
    }
    let registry = bound
        .actors
        .native_data_registry("Audio")
        .ok_or("actual FX Audio registry missing")?;
    if registry.mode != Some(0) {
        return Err("actual FX Audio manager mode differs from482090".into());
    }
    let Some(entry) = registry
        .entries
        .iter()
        .find(|e| e.ids.eq_ignore_ascii_case(&ids))
    else {
        return Ok(None);
    };
    if !bound.before_owner(&entry.source)? {
        return Ok(None);
    }
    let node = bound
        .actors
        .native_data_node(entry)
        .map_err(|e| e.to_string())?;
    let actual_ids = match bound.value(&node, "IDS")? {
        aa_rules::native::TypedValue::String { value, .. } => value,
        _ => return Err("Audio IDS is not an actual string".into()),
    };
    if !actual_ids.eq_ignore_ascii_case(&entry.ids) {
        return Err("actual FX Audio registered/source IDS differs".into());
    }
    Ok(Some(Reference {
        id: entry.ids.clone(),
        source: node.sources(),
        ordinal: None,
        cutscene: false,
    }))
}

///483A40 copies the first dispatched root's name into the empty manager+20.
///48DB60 registers this manager under the tag "Audio" (A0E65C) and every
///shipped Audio root spells it so. A name before any registered Audio root
///reached this owner, or another authored tag spelling, is not proved here.
pub(super) fn audio_manager_name(bound: &Bound<'_>) -> Result<String> {
    let registry = bound
        .actors
        .native_data_registry("Audio")
        .ok_or("actual FX Audio registry missing")?;
    for entry in &registry.entries {
        if bound.before_owner(&entry.source)? {
            return Ok(registry.manager.clone());
        }
    }
    Err("483A40 Audio manager name before its first root at this owner is unproved".into())
}

#[cfg(test)]
#[path = "fx_graph_sound_tests.rs"]
mod tests;
