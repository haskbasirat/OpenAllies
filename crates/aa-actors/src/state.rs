//! Original actor state bit groups (477FC0/473490) and4D64F0 transition order.
//! Entry/exit/property/action effects are required callbacks into the live world.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("actor state callback failed: {0}; earlier effects remain committed")]
    Callback(String),
    #[error("actor state index outside registered table: {0}")]
    State(u8),
    #[error("actor state group outside registered table: {0}")]
    Group(u8),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub index: u8,
    pub name: &'static str,
    pub group: Option<u8>,
    pub exclusive: bool,
}
pub const DESCRIPTORS: [Descriptor; 42] = [
    Descriptor {
        index: 0,
        name: "mind_attacking",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 1,
        name: "mind_bombard",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 2,
        name: "mind_building",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 3,
        name: "mind_chasing",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 4,
        name: "mind_engaged",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 5,
        name: "mind_exhausted",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 6,
        name: "mind_guarding",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 7,
        name: "mind_joining",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 8,
        name: "mind_moving",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 9,
        name: "mind_reattaching",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 10,
        name: "mind_regrouping",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 11,
        name: "mind_repairing",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 12,
        name: "mind_resting",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 13,
        name: "mind_retreating",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 14,
        name: "mind_routing",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 15,
        name: "mind_reforming",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 16,
        name: "mind_exploring",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 17,
        name: "mind_rampaging",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 18,
        name: "mind_razing",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 19,
        name: "out_of_supply",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 20,
        name: "under_siege",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 21,
        name: "construction",
        group: Some(15),
        exclusive: false,
    },
    Descriptor {
        index: 22,
        name: "organization_entrenched",
        group: Some(7),
        exclusive: false,
    },
    Descriptor {
        index: 23,
        name: "dead",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 24,
        name: "falling",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 25,
        name: "structure_packing",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 26,
        name: "structure_unpacking",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 27,
        name: "structure_normal",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 28,
        name: "structure_researching",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 29,
        name: "structure_upgrading",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 30,
        name: "structure_razing",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 31,
        name: "structure_recruiting",
        group: Some(13),
        exclusive: true,
    },
    Descriptor {
        index: 32,
        name: "active_construction",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 33,
        name: "summoning",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 34,
        name: "unpacked",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 35,
        name: "mind_packing",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 36,
        name: "mind_unpacking",
        group: Some(4),
        exclusive: true,
    },
    Descriptor {
        index: 37,
        name: "missing_primary",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 38,
        name: "unloading",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 39,
        name: "disabled",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 40,
        name: "targeted",
        group: None,
        exclusive: false,
    },
    Descriptor {
        index: 41,
        name: "hq_attached",
        group: None,
        exclusive: false,
    },
];
pub const GROUP_MASKS: [u64; 31] = [
    0x0,
    0x0,
    0x0,
    0x0,
    0x180007ffdf,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0xfe000000,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
];

/// GActor::Load removes these transient Mind states before component restoration.
pub const LOAD_CLEARED_MIND_MASK: u64 = 0x180007ffdf;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fields {
    pub bits: u64,
    pub thing_flags: u32,
}
pub fn group(bits: u64, group: u8) -> Result<Option<&'static Descriptor>> {
    let mask = *GROUP_MASKS
        .get(usize::from(group))
        .ok_or(Error::Group(group))?;
    let selected = bits & mask;
    Ok(if selected == 0 {
        None
    } else {
        Some(&DESCRIPTORS[selected.trailing_zeros() as usize])
    })
}
pub trait Effects {
    fn leave(&mut self, fields: &mut Fields, old: &'static Descriptor) -> Result<()>;
    fn enter(
        &mut self,
        fields: &mut Fields,
        new: &'static Descriptor,
        old: Option<&'static Descriptor>,
    ) -> Result<()>;
    fn rebuild_abilities(&mut self, fields: &mut Fields) -> Result<()>;
}
/// Earlier bit and callback effects remain committed on error. Native construction
/// flag31 suppresses enter/leave but does not suppress exclusive ability rebuild.
pub fn set(
    fields: &mut Fields,
    index: u8,
    enabled: bool,
    effects: &mut impl Effects,
) -> Result<()> {
    let descriptor = DESCRIPTORS
        .get(usize::from(index))
        .ok_or(Error::State(index))?;
    let bit = 1u64 << index;
    if (fields.bits & bit != 0) == enabled {
        return Ok(());
    }
    let live = fields.thing_flags & 0x80000000 == 0;
    if !descriptor.exclusive {
        if enabled {
            fields.bits |= bit;
        } else {
            fields.bits &= !bit;
        }
        if live {
            if enabled {
                effects.enter(fields, descriptor, None)?;
            } else {
                effects.leave(fields, descriptor)?;
                return Ok(());
            }
        }
    } else {
        let old = group(
            fields.bits,
            descriptor.group.expect("registered exclusive group"),
        )?;
        if let Some(old) = old {
            fields.bits &= !(1u64 << old.index);
            if live {
                effects.leave(fields, old)?;
            }
        }
        // Native exclusive path always sets this bit, including a request to disable
        // an already active exclusive state. Do not replace with generic toggle logic.
        fields.bits |= 1u64 << descriptor.index;
        if live {
            effects.enter(fields, descriptor, old)?;
        }
        effects.rebuild_abilities(fields)?;
    }
    if enabled && index == 24 {
        fields.thing_flags |= 0x1000;
    }
    Ok(())
}
