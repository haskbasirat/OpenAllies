//! Original Regiment Mind saved Load 528C20 (versions0..21). Source references
//! and world-owned collections remain typed; decoding never supplies live IDs.
use crate::saved_load::{Base, Reader, Result, SavedPath, Store, base, path};

mod context;
pub use context::{AppliedRegiment, Context, Owned, apply_with_context,take_constructed_owned};

/// Virtual+19C at 521C30, in original list order.
pub const STATE_LIST: [(u32, &str); 20] = [
    (0x34, "cai_sleep"),
    (0x54, "cai_rampage"),
    (0x68, "cai_move"),
    (0xc0, "cai_kill"),
    (0xd4, "cai_bombard"),
    (0xf8, "cai_siege"),
    (0x120, "cai_chasing"),
    (0x134, "cai_construct"),
    (0x160, "cai_repair"),
    (0x17c, "cai_guard"),
    (0x1a8, "cai_regroup"),
    (0x1b8, "cai_reform"),
    (0x1cc, "cai_engage"),
    (0x1f0, "cai_retreat"),
    (0x20c, "cai_rout"),
    (0x220, "cai_explore"),
    (0x24c, "cai_join"),
    (0x270, "cai_exhausted"),
    (0x280, "cai_unpack"),
    (0x2a4, "cai_pack"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regiment {
    pub version: u32,
    pub base: Option<Base>,
    pub stores: Vec<Store>,
    /// +2BC, appended in serialized order, version>=2.
    pub tracked_words: Option<Vec<u32>>,
    /// +9C owned path, version>=2.
    pub path: Option<SavedPath>,
    /// +B8 through 4C2D90: signed index in the owner's Organization LayoutData
    /// formation table; negative resolves null. It is not a command definition.
    pub desired_formation_index: Option<i32>,
    /// +148 ActorData reference: outer None means the field was not read;
    /// inner None is an empty serialized CString, which loads a null pointer.
    pub construct_definition: Option<Option<Vec<u16>>>,
    /// +E8, version>=6; preserve raw CString units.
    pub bombard_text: Option<Vec<u16>>,
    /// Original saved order, each (priority bits, GTID). Native 51FE60 inserts
    /// from tail, preserving equal-priority order, before storing that GTID.
    pub engage_records: Option<Vec<[u32; 2]>>,
    /// +264 vector, cleared at v2+ and populated at v7+.
    pub join_words: Option<Vec<u32>>,
    /// Read into stack temporaries and discarded, in native order.
    pub discarded: Vec<u8>,
}
impl Regiment {
    pub fn needs_world_time(&self) -> bool {
        self.version >= 2
    }
    pub fn engage_targets(&self) -> Option<crate::cai_engage::Targets> {
        self.engage_records.as_ref().map(|records| {
            let mut targets = crate::cai_engage::Targets::default();
            for &[priority, gtid] in records {
                crate::cai_engage::insert_sorted(&mut targets, gtid, priority as i32, true);
            }
            targets
        })
    }
}
fn words(r: &mut Reader<'_>, stores: &mut Vec<Store>, offsets: &[u32]) -> Result<()> {
    for &offset in offsets {
        stores.push(Store::Word {
            offset,
            value: r.u32()?,
        });
    }
    Ok(())
}
fn byte(r: &mut Reader<'_>, stores: &mut Vec<Store>, offset: u32) -> Result<()> {
    stores.push(Store::Byte {
        offset,
        value: r.u8()?,
    });
    Ok(())
}
fn count_words(r: &mut Reader<'_>) -> Result<Vec<u32>> {
    let count = r.u32()? as i32;
    let mut values = Vec::new();
    for _ in 0..count.max(0) {
        values.push(r.u32()?);
    }
    Ok(values)
}
pub(crate) fn read(r: &mut Reader<'_>) -> Result<Regiment> {
    let v = r.version(21)?;
    let mut result = Regiment {
        version: v,
        base: None,
        stores: Vec::new(),
        tracked_words: None,
        path: None,
        desired_formation_index: None,
        construct_definition: None,
        bombard_text: None,
        engage_records: None,
        join_words: None,
        discarded: Vec::new(),
    };
    if v == 0 {
        return Ok(result);
    }
    result.base = Some(base(r, &STATE_LIST)?);
    let s = &mut result.stores;
    words(r, s, &[0x2b4, 0x2b8])?;
    if v < 2 {
        return Ok(result);
    }
    result.tracked_words = Some(count_words(r)?);
    words(r, s, &[0x40])?;
    if v >= 9 {
        byte(r, s, 0x44)?;
        words(r, s, &[0x48])?;
    }
    if v >= 12 {
        byte(r, s, 0x4c)?;
    }
    s.push(Store::Word {
        offset: 0x50,
        value: 0,
    });
    words(r, s, &[0x60])?;
    byte(r, s, 0x64)?;
    words(r, s, &[0x74, 0x78, 0x7c, 0x80, 0x8c, 0x90, 0x94, 0x98])?;
    // Native world+E0 -> +BC is not serialized; application requires it.
    if v >= 15 {
        words(r, s, &[0x84, 0x88])?;
    }
    result.path = Some(path(r)?);
    if v >= 3 {
        result.desired_formation_index = Some(r.u32()? as i32);
    }
    words(r, s, &[0xcc, 0xd0, 0xe0])?;
    s.push(Store::Byte {
        offset: 0xf4,
        value: 1,
    });
    if v < 8 {
        result.discarded.push(r.u8()?);
    }
    words(r, s, &[0xe4])?;
    if v >= 6 {
        result.bombard_text = Some(r.units()?);
        words(r, s, &[0xec, 0xf0])?;
    }
    if v >= 4 {
        words(r, s, &[0x104, 0x108])?;
        if v < 5 || v >= 13 {
            words(r, s, &[0x10c])?;
        }
        s.extend([
            Store::Word {
                offset: 0x110,
                value: 0,
            },
            Store::Word {
                offset: 0x114,
                value: 0,
            },
            Store::Byte {
                offset: 0x118,
                value: 1,
            },
            Store::Byte {
                offset: 0x119,
                value: 1,
            },
            Store::Word {
                offset: 0x11c,
                value: 0,
            },
        ]);
    }
    words(r, s, &[0x12c, 0x130, 0x140])?;
    byte(r, s, 0x144)?;
    let reference = r.units()?;
    result.construct_definition = Some((!reference.is_empty()).then_some(reference));
    words(r, s, &[0x14c, 0x150])?;
    if v >= 11 {
        words(r, s, &[0x15c])?;
    }
    if v >= 20 {
        words(r, s, &[0x154, 0x158])?;
    }
    words(r, s, &[0x16c])?;
    byte(r, s, 0x170)?;
    words(r, s, &[0x174])?;
    if v >= 10 {
        byte(r, s, 0x178)?;
    }
    words(r, s, &[0x188, 0x18c])?;
    byte(r, s, 0x190)?;
    words(
        r,
        s,
        &[
            0x194, 0x198, 0x19c, 0x1a0, 0x1a4, 0x1b4, 0x1c4, 0x1c8, 0x1d8,
        ],
    )?;
    let count = r.u32()? as i32;
    let mut records = Vec::new();
    for _ in 0..count.max(0) {
        records.push([r.u32()?, r.u32()?]);
    }
    result.engage_records = Some(records);
    if v >= 18 {
        words(r, s, &[0x1e4])?;
    }
    if v >= 19 {
        byte(r, s, 0x1e8)?;
    }
    if v >= 5 {
        result.discarded.push(r.u8()?);
    }
    words(r, s, &[0x1fc, 0x200, 0x204, 0x208, 0x218])?;
    result.discarded.extend_from_slice(r.take(8)?);
    words(r, s, &[0x21c, 0x22c, 0x230, 0x234, 0x238, 0x23c])?;
    s.push(Store::Word {
        offset: 0x248,
        value: 0,
    });
    if v >= 17 {
        words(r, s, &[0x240, 0x244, 0x248])?;
    }
    words(r, s, &[0x258, 0x25c])?;
    byte(r, s, 0x260)?;
    result.join_words = Some(if v >= 7 {
        byte(r, s, 0x261)?;
        count_words(r)?
    } else {
        s.push(Store::Byte {
            offset: 0x261,
            value: 0,
        });
        Vec::new()
    });
    words(r, s, &[0x27c])?;
    if v >= 14 {
        words(r, s, &[0x28c, 0x290, 0x294])?;
        if v >= 21 {
            words(r, s, &[0x298, 0x29c])?;
        }
        if v >= 16 {
            words(r, s, &[0x2a0])?;
        }
        words(r, s, &[0x2b0])?;
    }
    Ok(result)
}
pub fn decode(bytes: &[u8]) -> Result<(Regiment, usize)> {
    let mut r = Reader { bytes, at: 0 };
    let saved = read(&mut r)?;
    Ok((saved, r.at))
}

/// Version0 returns immediately. Version1 restores only Base52C580 and the
/// pair at2B4. Later versions require world/Organization/reference/collection
/// owners; the contextless entry point fails before changing anything.
pub(crate) fn apply_basic(
    saved: &Regiment,
    installed: &mut crate::construct::Installed,
    arena: &mut crate::Arena,
    commands: &crate::saved_load::CommandBindings,
) -> Result<crate::saved_load::Applied> {
    use crate::saved_load::Error;
    if installed.kind != crate::construct::Kind::Regiment {
        return Err(Error::Mismatch(
            "Regiment payload for a different constructed Mind".into(),
        ));
    }
    if saved.version >= 2 {
        return Err(Error::Unsupported(
            "Regiment v2+ application requires world time, Organization formations, ActorData/CString references and typed collection owners",
        ));
    }
    let (count, current) = saved
        .base
        .as_ref()
        .map(|b| (b.commands.len(), b.current_command.is_some()))
        .unwrap_or((0, false));
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
    let mut stack = Vec::new();
    if let Some(base) = &saved.base {
        if let Some(states) = &base.stack {
            for offset in states {
                let state = *installed.state_bindings.get(offset).ok_or_else(|| {
                    Error::Mismatch(format!("Regiment state {offset:#x} is not bound"))
                })?;
                arena
                    .state(state)
                    .map_err(|e| Error::Mismatch(e.to_string()))?;
                stack.push(Some(state));
            }
        }
    }
    for store in &saved.stores {
        let Store::Word { offset, .. } = store else {
            return Err(Error::Mismatch(
                "unexpected byte store in Regiment v0/v1".into(),
            ));
        };
        if ![0x2b4, 0x2b8].contains(offset) || !installed.words.contains_key(offset) {
            return Err(Error::Mismatch(
                "Regiment v0/v1 field was not constructed".into(),
            ));
        }
    }
    let mut serialized_id = None;
    if let Some(base) = &saved.base {
        installed.update_head = None;
        serialized_id = base.updatable.serialized_id.filter(|id| *id >= 0);
        let mind = arena
            .mind_mut(installed.mind_id)
            .map_err(|e| Error::Mismatch(e.to_string()))?;
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
    }
    for store in &saved.stores {
        if let Store::Word { offset, value } = store {
            installed.words.insert(*offset, *value);
        }
    }
    Ok(crate::saved_load::Applied {
        serialized_id,
        element_path: None,
    })
}
