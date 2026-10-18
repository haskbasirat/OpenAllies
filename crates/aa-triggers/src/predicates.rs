//! Trigger event payloads (manager+50 queue) and the original event scanners.
//!
//! Payload layouts follow the original constructors; the kind is the value of
//! the payload's first virtual (4C9FA0..6B2450):
//! * 0 `Destroyed`   54B7B0: +4 victim GTID, +8 killer GTID, +C killer word.
//! * 1 `Captured`    54B820: +4 actor GTID, +8 capturer GTID, +C Kingdom GTID.
//! * 2 `AreaEntry`   54B8C0: +4 actor GTID, +8 Area object.
//! * 3 `Construction`54B900: +8 Kingdom GTID, +C definition, +10 flag (1 begins).
//! * 4 `Recruitment` 54B940: +8 Kingdom GTID, +C definition, +10 flag.
//! * 5 `Script`      54B9F0: +8 owned string.
//!
//! The queue is newest-first (54BE10.. prepend).  54B9B0/54B980 iterate it and
//! call the kind virtual of every visited payload, so a null payload node is an
//! explicit error here (native would dereference null).  Subject additions use
//! Manager::add_subject (54C380 semantics: dedupe, prepend, cursor := head).
use crate::manager::{Error, Manager, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Destroyed {
        victim_id: u32,
        killer_id: u32,
        word_c: u32,
    },
    Captured {
        actor_id: u32,
        capturer_id: u32,
        kingdom_id: u32,
    },
    /// Area is a stable runtime object handle (native pointer).
    AreaEntry { actor_id: u32, area: u64 },
    /// Definition is a stable runtime handle of the ActorData (native pointer).
    Construction {
        kingdom_id: u32,
        definition: u64,
        flag_10: u8,
    },
    Recruitment {
        kingdom_id: u32,
        definition: u64,
        flag_10: u8,
    },
    Script { text: Vec<u16> },
    /// Old manager sequencing fixtures have no native event kind.
    UnclassifiedWords(Vec<u32>),
}
impl Event {
    pub fn kind(&self) -> Result<u32> {
        Ok(match self {
            Self::Destroyed { .. } => 0,
            Self::Captured { .. } => 1,
            Self::AreaEntry { .. } => 2,
            Self::Construction { .. } => 3,
            Self::Recruitment { .. } => 4,
            Self::Script { .. } => 5,
            Self::UnclassifiedWords(_) => {
                return Err(Error::State("event kind is unresolved".into()));
            }
        })
    }
}

/// 54B9B0/54B980 filtered iteration, collected before any subject mutation.
fn scan(manager: &Manager, kind: u32) -> Result<Vec<Event>> {
    let mut out = vec![];
    for payload in manager.events() {
        let payload = payload.as_ref().ok_or_else(|| {
            Error::State("null event payload: native iterator dereferences it".into())
        })?;
        if payload.data.kind()? == kind {
            out.push(payload.data.clone());
        }
    }
    Ok(out)
}

/// 54C450: every Destroyed event adds killer then victim; true if any exists.
pub fn destroyed_any(manager: &mut Manager) -> Result<bool> {
    let events = scan(manager, 0)?;
    for e in &events {
        if let Event::Destroyed {
            victim_id,
            killer_id,
            ..
        } = e
        {
            manager.add_subject(*killer_id);
            manager.add_subject(*victim_id);
        }
    }
    Ok(!events.is_empty())
}

/// 54C530: killer of the first Destroyed event for `victim`; adds it as subject.
pub fn killer_of(manager: &mut Manager, victim: u32) -> Result<u32> {
    for e in scan(manager, 0)? {
        if let Event::Destroyed {
            victim_id,
            killer_id,
            ..
        } = e
            && victim_id == victim
        {
            manager.add_subject(killer_id);
            return Ok(killer_id);
        }
    }
    Ok(0)
}

/// 54C660: first Destroyed event whose killer is `killer`; adds the killer.
pub fn killed_by(manager: &mut Manager, killer: u32) -> Result<bool> {
    for e in scan(manager, 0)? {
        if let Event::Destroyed { killer_id, .. } = e
            && killer_id == killer
        {
            manager.add_subject(killer_id);
            return Ok(true);
        }
    }
    Ok(false)
}

/// 54C5F0(kingdom, actor): the FIRST Captured event for `actor` decides.
pub fn captured(manager: &mut Manager, kingdom: u32, actor: u32) -> Result<bool> {
    for e in scan(manager, 1)? {
        if let Event::Captured {
            actor_id,
            kingdom_id,
            ..
        } = e
            && actor_id == actor
        {
            if kingdom_id != kingdom {
                return Ok(false);
            }
            manager.add_subject(actor_id);
            return Ok(true);
        }
    }
    Ok(false)
}

/// 54C710: first AreaEntry of `actor` into `area`.
pub fn entered(manager: &mut Manager, actor: u32, area: u64) -> Result<bool> {
    for e in scan(manager, 2)? {
        if let Event::AreaEntry {
            actor_id,
            area: entered,
        } = e
            && actor_id == actor
            && entered == area
        {
            manager.add_subject(actor_id);
            return Ok(true);
        }
    }
    Ok(false)
}

/// 54C790: every AreaEntry into `area` adds its actor.
pub fn entered_any(manager: &mut Manager, area: u64) -> Result<bool> {
    let mut found = false;
    for e in scan(manager, 2)? {
        if let Event::AreaEntry {
            actor_id,
            area: entered,
        } = e
            && entered == area
        {
            manager.add_subject(actor_id);
            found = true;
        }
    }
    Ok(found)
}

/// 54BA90/54BB00 (begins: flag != 0) and 54BB70/54BBE0 (finishes: flag == 0).
pub fn construction(
    manager: &Manager,
    kingdom: u32,
    definition: Option<u64>,
    begins: bool,
) -> Result<bool> {
    Ok(scan(manager, 3)?.into_iter().any(|e| {
        matches!(e, Event::Construction { kingdom_id, definition: d, flag_10 }
            if kingdom_id == kingdom && (flag_10 != 0) == begins && definition.is_none_or(|w| w == d))
    }))
}

/// 54BC50/54BCC0: Recruitment with flag == 0.
pub fn recruited(manager: &Manager, kingdom: u32, definition: Option<u64>) -> Result<bool> {
    Ok(scan(manager, 4)?.into_iter().any(|e| {
        matches!(e, Event::Recruitment { kingdom_id, definition: d, flag_10 }
            if kingdom_id == kingdom && flag_10 == 0 && definition.is_none_or(|w| w == d))
    }))
}

/// MSVC C-locale `_wcsicmp` equality: ASCII A-Z fold, ends at first NUL.
pub fn wcsicmp_eq(a: &[u16], b: &[u16]) -> bool {
    let fold = |c: u16| if (65..=90).contains(&c) { c + 32 } else { c };
    a.iter()
        .copied()
        .take_while(|c| *c != 0)
        .map(fold)
        .eq(b.iter().copied().take_while(|c| *c != 0).map(fold))
}

/// 542DB0/54CAC0: a nonempty name heard as a Script event.
pub fn script_heard(manager: &Manager, text: &[u16]) -> Result<bool> {
    if text.is_empty() {
        return Ok(false);
    }
    Ok(scan(manager, 5)?
        .into_iter()
        .any(|e| matches!(e, Event::Script { text: t } if wcsicmp_eq(&t, text))))
}

/// 542DE0 with __allshl: the low byte of `index` is the shift count; 64..255 -> 0.
pub fn has_state_bit(bits: u64, index: u32) -> bool {
    let shift = index as u8;
    shift < 64 && bits & (1u64 << shift) != 0
}

/// Objects read by the 54B820 Captured payload constructor.
pub trait CaptureObjects {
    /// Object+14 GTID.
    fn capture_gtid(&self, object: u64) -> Result<u32>;
    /// Virtual +A8: the object is itself a Kingdom.
    fn capture_is_kingdom(&mut self, object: u64) -> Result<bool>;
    /// Virtual +108: the object's owning Kingdom.
    fn capture_owner(&mut self, object: u64) -> Result<Option<u64>>;
}

/// 54BEA0 TriggerManager::QueueCaptured(captured, source): a 54B820 payload
/// (+4 captured +14, +8 source +14, +C the source itself when virtual +A8 is
/// true, else its virtual +108 owner, that object's +14; 0 for a null object)
/// prepended to the manager+50 queue (newest first). This is the binding of
/// aa-combat's capture notification (`AttemptWorld::capture_notification`,
/// native 4D5230 -> 54BEA0). `identity` is the caller's stable payload id.
/// A null source is an explicit error: 54B820 calls its virtual +A8.
pub fn queue_captured<C: CaptureObjects + ?Sized>(
    manager: &mut Manager,
    identity: u64,
    captured: Option<u64>,
    source: Option<u64>,
    objects: &mut C,
) -> Result<()> {
    let actor_id = captured.map(|o| objects.capture_gtid(o)).transpose()?.unwrap_or(0);
    let source = source.ok_or_else(|| {
        Error::State("54B820 calls virtual +A8 through a null capture source".into())
    })?;
    let capturer_id = objects.capture_gtid(source)?;
    let kingdom = if objects.capture_is_kingdom(source)? {
        Some(source)
    } else {
        objects.capture_owner(source)?
    };
    let kingdom_id = kingdom.map(|k| objects.capture_gtid(k)).transpose()?.unwrap_or(0);
    manager.push_event(Some(crate::manager::DeferredPayload {
        identity,
        data: Event::Captured {
            actor_id,
            capturer_id,
            kingdom_id,
        },
    }));
    Ok(())
}
