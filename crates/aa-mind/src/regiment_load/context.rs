//! Context-complete fresh Regiment Load528C20. The returned containers become
//! the one owner of transferred fields; corresponding native pointer/scalar
//! slots are removed from Installed, not retained as mutable shadow state.
use super::Regiment;
use crate::{
    Arena,
    construct::{Installed, Kind, Reference},
    saved_load::{CommandBindings, Error, Result, Store},
};

/// Facts from the actual world/Organization/ActorData registries at Load time.
/// No Default: a zero clock or an empty formation table must be supplied only
/// when they are the actual facts. Negative saved formation indices do not
/// dereference the formation table in native4C2D90.
pub struct Context<'a> {
    pub world_time_bits: u32,
    pub formations: Option<&'a [u32]>,
    /// Native+148 ActorData pointer replacement. None is valid only for an
    /// actually empty saved CString; unknown nonempty IDs must not become null.
    pub construct_definition: Option<u32>,
}

/// Exact contents transferred out of the constructed remainder. Native node
/// pointers and vector capacities have no duplicate mutable representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owned {
    pub tracked_words: crate::cai_arrivals::List,
    pub path: aa_navigation::OwnedPath,
    pub engage_targets: crate::cai_engage::Targets,
    pub join_words: Vec<u32>,
    pub bombard_text: Vec<u16>,
    pub desired_formation: Option<u32>,
    pub construct_definition: Option<u32>,
}
#[derive(Debug)]
pub struct AppliedRegiment {
    pub serialized_id: Option<i32>,
    pub owned: Owned,
}

///Move the actual empty collections made by526260 into their sole typed
///owner. This does not reset later/current data: every transferred native
///constructor word/reference must still match its proved constructor value.
///After successful transfer Installed no longer carries mutable shadow slots.
pub fn take_constructed_owned(installed:&mut Installed)->Result<Owned>{
    if installed.kind!=Kind::Regiment{return Err(Error::Mismatch("fresh Regiment collections require actual526260 kind".into()))}
    for (offset,value)in [(0xa0,0),(0xa4,0),(0xa8,0),(0xac,0),(0xb4,u32::MAX),(0xb8,0),
        (0x148,0),(0x1dc,0),(0x1e0,0),(0x268,0),(0x26c,1),(0x2bc,0),(0x2c0,0)]{
        if installed.words.get(&offset)!=Some(&value){return Err(Error::Mismatch(format!("Regiment constructor collection+{offset:X} changed or already transferred")))}
    }
    if installed.bytes.get(&0xb0)!=Some(&1)
        ||installed.references.get(&0x9c)!=Some(&Reference::NativeSubobject{vtable:0xa34738})
        ||installed.references.get(&0xe8)!=Some(&Reference::EmptyUtf16)
        ||installed.references.get(&0x264)!=Some(&Reference::EmptyWordVector{initial_capacity:1}){
        return Err(Error::Mismatch("Regiment actual constructor path/text/vector owner differs".into()))
    }
    let owned=Owned{tracked_words:crate::cai_arrivals::List::from_constructor(),path:aa_navigation::OwnedPath{
        nodes:Vec::new(),kind1_count:0,kind2_count:0,dirty:true,word18:u32::MAX},
        engage_targets:crate::cai_engage::Targets::default(),join_words:Vec::new(),bombard_text:Vec::new(),
        desired_formation:None,construct_definition:None};
    for offset in [0xa0,0xa4,0xa8,0xac,0xb4,0xb8,0x148,0x1dc,0x1e0,0x268,0x26c,0x2bc,0x2c0]{installed.words.remove(&offset);}
    installed.bytes.remove(&0xb0);
    for offset in [0x9c,0xe8,0x264]{installed.references.remove(&offset);}
    Ok(owned)
}

/// Full v2..21 application after factory526260. `saved` is consumed, moving
/// list/text/node payloads into the actual returned owner. Existing owned
/// commands transfer by their unique already-owned IDs, without reallocation.
/// Resolution/validation precedes mutation; native abort-on-Load-error partial
/// side effects and allocator exceptions are not claimed by this API.
pub fn apply_with_context(
    saved: Regiment,
    installed: &mut Installed,
    arena: &mut Arena,
    commands: &CommandBindings,
    context: Context<'_>,
) -> Result<AppliedRegiment> {
    if installed.kind != Kind::Regiment || !(2..=21).contains(&saved.version) {
        return Err(Error::Mismatch(
            "context Regiment application requires a fresh Regiment and saved version2..21".into(),
        ));
    }
    if saved.desired_formation_index.is_some() != (saved.version >= 3)
        || saved.bombard_text.is_some() != (saved.version >= 6)
    {
        return Err(Error::Mismatch(
            "Regiment version and reference payload presence disagree".into(),
        ));
    }
    let base = saved
        .base
        .as_ref()
        .ok_or_else(|| Error::Mismatch("Regiment base payload is missing".into()))?;
    let count = base.commands.len();
    let current = base.current_command.is_some();
    if commands.pending.len() != count || commands.current.is_some() != current {
        return Err(Error::Mismatch(
            "saved command bindings do not match the Regiment payload".into(),
        ));
    }
    let ids: std::collections::BTreeSet<_> = commands
        .pending
        .iter()
        .copied()
        .chain(commands.current)
        .collect();
    if ids.contains(&0) || ids.len() != count + usize::from(current) {
        return Err(Error::Mismatch(
            "saved command allocations require distinct nonzero identities".into(),
        ));
    }
    let mind = arena
        .mind(installed.mind_id)
        .map_err(|e| Error::Mismatch(e.to_string()))?;
    if !mind.pending_commands.is_empty() || mind.current_command.is_some() {
        return Err(Error::Mismatch(
            "constructed Regiment already owns commands".into(),
        ));
    }
    let status_state = |offset: u32| {
        offset
            .checked_sub(12)
            .and_then(|off| installed.state_bindings.get(&off).copied())
    };
    for store in &saved.stores {
        match *store {
            Store::Word { offset, .. } => {
                if let Some(id) = status_state(offset) {
                    arena
                        .state(id)
                        .map_err(|e| Error::Mismatch(e.to_string()))?;
                } else if !installed.words.contains_key(&offset) {
                    return Err(Error::Mismatch(format!(
                        "Regiment field{offset:#x} was not constructed"
                    )));
                }
            }
            Store::Byte { offset, .. } => {
                if !installed.bytes.contains_key(&offset) {
                    return Err(Error::Mismatch(format!(
                        "Regiment byte{offset:#x} was not constructed"
                    )));
                }
            }
        }
    }
    let mut stack = Vec::new();
    if let Some(states) = &base.stack {
        for offset in states {
            let id = *installed.state_bindings.get(offset).ok_or_else(|| {
                Error::Mismatch(format!("Regiment state{offset:#x} is not bound"))
            })?;
            arena
                .state(id)
                .map_err(|e| Error::Mismatch(e.to_string()))?;
            stack.push(Some(id));
        }
    }
    // Original fresh constructor owns empty collections and null references.
    // A second application cannot clear/recreate a detached canonical owner.
    for offset in [
        0xa0, 0xa4, 0x110, 0x114, 0x1dc, 0x1e0, 0x268, 0x2bc, 0x2c0, 0xb8, 0x148,
    ] {
        if installed.words.get(&offset) != Some(&0) {
            return Err(Error::Mismatch(format!(
                "Regiment owned field{offset:#x} is not fresh"
            )));
        }
    }
    for offset in [0xa8, 0xac, 0xb4, 0xbc, 0x26c] {
        if !installed.words.contains_key(&offset) {
            return Err(Error::Mismatch(format!(
                "Regiment field{offset:#x} was not constructed"
            )));
        }
    }
    if !installed.bytes.contains_key(&0xb0)
        || installed.references.get(&0x9c) != Some(&Reference::NativeSubobject { vtable: 0xa34738 })
        || installed.references.get(&0xe8) != Some(&Reference::EmptyUtf16)
        || installed.references.get(&0x264)
            != Some(&Reference::EmptyWordVector {
                initial_capacity: 1,
            })
    {
        return Err(Error::Mismatch(
            "Regiment owned path/text/vector constructor identity is unavailable".into(),
        ));
    }
    let desired_formation = match saved.desired_formation_index {
        None => None, // v2 leaves the proven constructor null value.
        Some(index) if index < 0 => None,
        Some(index) => {
            let formations = context.formations.ok_or(Error::Unsupported(
                "saved Regiment requires its actual Organization formation table",
            ))?;
            let value = *formations.get(index as usize).ok_or_else(|| {
                Error::Mismatch(format!(
                    "Bad desired formation index {index} when only {} formations",
                    formations.len()
                ))
            })?;
            (value != 0).then_some(value)
        }
    };
    let source_ref = saved
        .construct_definition
        .as_ref()
        .ok_or_else(|| Error::Mismatch("Regiment construct reference was not decoded".into()))?;
    match (source_ref, context.construct_definition) {
        (None, None) => {}
        (Some(_), Some(id)) if id != 0 => {}
        _ => {
            return Err(Error::Mismatch(
                "saved Regiment ActorData reference is unresolved or disagrees with its source"
                    .into(),
            ));
        }
    }
    let path = saved
        .path
        .as_ref()
        .ok_or_else(|| Error::Mismatch("Regiment owned path is missing".into()))?;
    let (kind1, kind2) = path.kind_counts();
    if saved.tracked_words.is_none() || saved.engage_records.is_none() || saved.join_words.is_none()
    {
        return Err(Error::Mismatch(
            "Regiment owned collection payload is missing".into(),
        ));
    }
    let engage_targets = saved.engage_targets().expect("validated records");
    // No fallible registry/world callback occurs after this point.
    let serialized_id = base.updatable.serialized_id.filter(|id| *id >= 0);
    installed.update_head = None;
    let mind = arena.mind_mut(installed.mind_id).expect("validated Mind");
    mind.words_24_30[0] = base.word_24;
    mind.words_24_30[1] = base.word_28;
    mind.pending_commands
        .extend(commands.pending.iter().copied());
    mind.current_command = commands.current;
    if let Some(word) = base.word_2c {
        mind.words_24_30[2] = word;
        mind.stack.clear();
        mind.stack.extend(stack);
    }
    for store in saved.stores {
        match store {
            Store::Word { offset, value } => {
                if let Some(id) = status_state(offset) {
                    arena.state_mut(id).expect("validated state").status = value;
                } else {
                    installed.words.insert(offset, value);
                }
            }
            Store::Byte { offset, value } => {
                installed.bytes.insert(offset, value);
            }
        }
    }
    installed.words.insert(0xbc, context.world_time_bits);
    let path = saved.path.expect("validated path");
    let owned = Owned {
        tracked_words: saved.tracked_words.expect("validated tracked words").into(),
        path: aa_navigation::OwnedPath {
            nodes: path.records.into_iter().rev().map(|r| r.node).collect(),
            kind1_count: kind1,
            kind2_count: kind2,
            dirty: true,
            word18: path.word_18,
        },
        engage_targets,
        join_words: saved.join_words.expect("validated join words"),
        bombard_text: saved.bombard_text.unwrap_or_default(), // v2..5 preserve constructor's empty CString.
        desired_formation,
        construct_definition: context.construct_definition,
    };
    for off in [
        0xa0, 0xa4, 0xa8, 0xac, 0xb4, 0xb8, 0x148, 0x1dc, 0x1e0, 0x268, 0x26c, 0x2bc, 0x2c0,
    ] {
        installed.words.remove(&off);
    }
    installed.bytes.remove(&0xb0);
    for off in [0x9c, 0xe8, 0x264] {
        installed.references.remove(&off);
    }
    Ok(AppliedRegiment {
        serialized_id,
        owned,
    })
}
