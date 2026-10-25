//! Typed film decrees: the flat aa-orders payload fields of one decoded
//! descriptor, given names. Field order is the native read order of each
//! schema (aa-orders `payload/{game,lobby,complex}.rs`). Floats stay raw
//! binary32 bits. Descriptors without a typed variant keep their fields in
//! [`Decree::Other`].
use aa_orders::commands::{CommandWire, ParameterSet};
use aa_orders::payload::{Field, Registry};
use serde::Serialize;

/// Native descriptor ordinals (aa-orders `native_registry.rs`) used here.
pub mod id {
    pub const SELECTION_ADD: usize = 16;
    pub const SELECTION_REMOVE: usize = 17;
    pub const SELECTION_CLEAR: usize = 18;
    pub const SELECTION_CLEAR_TEXT: usize = 19;
    pub const TEAM_COMMAND: usize = 23;
    pub const TELL_ACTOR_ATTACK: usize = 65;
    pub const TELL_ACTOR_MOVE: usize = 66;
    pub const TELL_ACTOR_ABILITY: usize = 67;
    pub const GAME_PLAYER_KINGDOM: usize = 68;
    pub const FACTION_USE_ABILITY: usize = 70;
    pub const SET_GAME_SPEED: usize = 73;
    pub const TOGGLE_PAUSED: usize = 75;
    pub const SET_PAUSED: usize = 76;
    pub const PUBLIC_CHAT: usize = 77;
    pub const PRIVATE_CHAT: usize = 78;
    pub const TEAM_CHAT: usize = 79;
    pub const SPEED_UP: usize = 80;
    pub const SPEED_DOWN: usize = 81;
    pub const TELL_SELECTED_COMMAND: usize = 86;
    pub const TELL_ACTOR_COMMAND: usize = 87;
    pub const SET_PLAYER: usize = 298;
    pub const SIM_SYNC_VALUE: usize = 311;
    pub const SYNC_VALUE: usize = 314;
    pub const VERSION_ACK: usize = 323;
    pub const UPDATE_KINGDOM_NAME: usize = 328;
    pub const SELECTION_SET: usize = 333;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Decree {
    /// 298 TGC_SetPlayerOrderDecree: following decrees are ordered by this
    /// session player (Admin+0x284, process 0x75D710).
    SetPlayer {
        player: u32,
    },
    /// 311 KKC_SetSimSyncValueOrderDecree {S last index, S hash} (0x74E670).
    SimSyncValue {
        index: i32,
        hash: i32,
    },
    /// 314 TGC_SetSyncValueOrderDecree {A last index, A hash} (0x74E8B0).
    SyncValue {
        index: i32,
        hash: i32,
    },
    /// 333 SelectionSetOrderDecree: the selection becomes this thing.
    SelectionSet {
        slot: u16,
    },
    /// 16 SelectionAddOrderDecree.
    SelectionAdd {
        slot: u16,
    },
    /// 17 SelectionRemoveOrderDecree.
    SelectionRemove {
        slot: u16,
    },
    /// 18 SelectionClearOrderDecree.
    SelectionClear,
    /// 19 SelectionClearOrderDecree (the text variant, process 0x5F4510).
    SelectionClearNamed {
        name: Vec<u16>,
    },
    /// 86 KKC_TellSelectedCommandOrderDecree: the generic command (0x61EBA0)
    /// applied to the ordering player's selection.
    TellSelectedCommand(CommandWire),
    /// 87 KKC_TellActorCommandOrderDecree: the command, then the actor slot.
    TellActorCommand {
        command: CommandWire,
        actor: u16,
    },
    /// 65 KKC_TellActorAttackOrderDecree.
    TellActorAttack {
        attacker: u16,
        target: u16,
    },
    /// 66 KKC_TellActorMoveOrderDecree (0x60F760): CString selector and the
    /// native +8/+C/+10 words (XY used by the process 0x61E830).
    TellActorMove {
        selector: Vec<u16>,
        words: [u32; 3],
    },
    /// 67 KKC_TellActorAbilityOrderDecree.
    TellActorAbility {
        selector: Vec<u16>,
        ability: u32,
        text: Vec<u16>,
    },
    /// 68 GamePlayerKingdomDecree.
    GamePlayerKingdom {
        kingdom: u16,
    },
    /// 70 KKC_FactionUseAbilityOrderDecree (0x6109F0): ability definition,
    /// two raw words (the target point), optional target actor.
    FactionUseAbility {
        ability: u32,
        words: [u32; 2],
        target: Option<u16>,
    },
    /// 73 KKC_SetGameSpeedOrderDecree (payload one 32-bit word).
    SetGameSpeed {
        bits: u32,
    },
    /// 75/76/80/81 timeline decrees.
    TogglePaused,
    SetPaused {
        paused: bool,
    },
    SpeedUp,
    SpeedDown,
    /// 328 KKC_UpdateKingdomNameOrderDecree.
    UpdateKingdomName {
        kingdom: Vec<u16>,
        name: Vec<u16>,
        key: Vec<u16>,
        flag: bool,
    },
    /// Any other descriptor, with its decoded fields.
    Other {
        fields: Vec<Field>,
    },
}

struct Reader<'a>(std::slice::Iter<'a, Field>);
impl Reader<'_> {
    fn uint(&mut self, width: u8) -> Option<u32> {
        match self.0.next()? {
            Field::Uint { width: w, value } if *w == width => Some(*value),
            _ => None,
        }
    }
    fn text(&mut self) -> Option<Vec<u16>> {
        match self.0.next()? {
            Field::Text(u) => Some(u.clone()),
            _ => None,
        }
    }
    fn thing(&mut self) -> Option<u16> {
        match self.0.next()? {
            Field::Thing { slot, .. } => Some(*slot),
            _ => None,
        }
    }
    fn def(&mut self, registry: Registry) -> Option<u32> {
        match self.0.next()? {
            Field::Def { registry: r, index } if *r == registry => Some(*index),
            _ => None,
        }
    }
    fn bounded(&mut self, n: u32) -> Option<u32> {
        match self.0.next()? {
            Field::Bounded { n: m, value } if *m == n => Some(*value),
            _ => None,
        }
    }
    fn done(&mut self) -> bool {
        self.0.next().is_none()
    }
}

/// `Some(index)` unless the wire value is the null sentinel `count`.
fn reference(index: u32, count: Option<usize>) -> Option<u32> {
    match count {
        Some(n) if index as usize == n => None,
        _ => Some(index),
    }
}

/// The 0x61EBA0 generic command from its decoded fields (the inverse of
/// aa-orders `payload::complex::command`). `counts` resolves null sentinels.
fn command(r: &mut Reader<'_>, counts: &dyn Fn(Registry) -> Option<usize>) -> Option<CommandWire> {
    use crate::registries::*;
    let flags = r.bounded(0x800)? as u16;
    let has = |bit: u32| flags >> bit & 1 != 0;
    let mut wire = CommandWire {
        flags,
        ..CommandWire::default()
    };
    if has(6) {
        wire.command = reference(r.def(ACTOR_COMMANDS)?, counts(ACTOR_COMMANDS));
    }
    if has(0) {
        wire.word = r.uint(32)?;
    }
    if has(1) {
        wire.object = reference(r.def(OBJECT_DEFINITIONS)?, counts(OBJECT_DEFINITIONS));
    }
    if has(2) {
        wire.position = [r.uint(32)?, r.uint(32)?];
    }
    if has(3) {
        wire.auxiliary = [r.uint(32)?, r.uint(32)?];
    }
    if has(4) {
        wire.scalar = r.uint(32)?;
    }
    if has(7) {
        wire.parameter = reference(r.def(FORMATIONS)?, counts(FORMATIONS));
    }
    if has(5) && r.uint(1)? == 0 {
        let definition = reference(r.def(ORGANIZATION_LAYOUTS)?, counts(ORGANIZATION_LAYOUTS));
        let raw = r.uint(32)?;
        let entries = if (raw as i32) > 0 { raw } else { 0 };
        let mut objects = Vec::new();
        for _ in 0..entries {
            objects.push(reference(
                r.def(OBJECT_DEFINITIONS)?,
                counts(OBJECT_DEFINITIONS),
            ));
        }
        let negative_count = (raw as i32 <= 0 && raw != 0).then_some(raw as i32);
        wire.parameters = Some(ParameterSet {
            definition,
            objects,
            negative_count,
        });
    }
    if has(8) {
        wire.extra = reference(r.def(SHARED_PROPERTIES)?, counts(SHARED_PROPERTIES));
    }
    if has(9) {
        wire.boolean = r.uint(1)? == 1;
    }
    wire.trailing = r.uint(1)? == 1;
    Some(wire)
}

impl Decree {
    /// Name the fields of `descriptor`. Field shapes that do not match the
    /// schema fall back to [`Decree::Other`] (they cannot occur for fields
    /// produced by `aa_orders::payload::decode`).
    pub fn from_fields(
        descriptor: usize,
        fields: &[Field],
        counts: &dyn Fn(Registry) -> Option<usize>,
    ) -> Self {
        Self::typed(descriptor, fields, counts).unwrap_or_else(|| Decree::Other {
            fields: fields.to_vec(),
        })
    }
    fn typed(
        descriptor: usize,
        fields: &[Field],
        counts: &dyn Fn(Registry) -> Option<usize>,
    ) -> Option<Self> {
        let mut r = Reader(fields.iter());
        let out = match descriptor {
            id::SET_PLAYER => Decree::SetPlayer {
                player: r.uint(32)?,
            },
            id::SIM_SYNC_VALUE => Decree::SimSyncValue {
                index: r.uint(32)? as i32,
                hash: r.uint(32)? as i32,
            },
            id::SYNC_VALUE => Decree::SyncValue {
                index: r.uint(32)? as i32,
                hash: r.uint(32)? as i32,
            },
            id::SELECTION_SET => Decree::SelectionSet { slot: r.thing()? },
            id::SELECTION_ADD => Decree::SelectionAdd { slot: r.thing()? },
            id::SELECTION_REMOVE => Decree::SelectionRemove { slot: r.thing()? },
            id::SELECTION_CLEAR => Decree::SelectionClear,
            id::SELECTION_CLEAR_TEXT => Decree::SelectionClearNamed { name: r.text()? },
            id::TELL_SELECTED_COMMAND => Decree::TellSelectedCommand(command(&mut r, counts)?),
            id::TELL_ACTOR_COMMAND => {
                let command = command(&mut r, counts)?;
                Decree::TellActorCommand {
                    command,
                    actor: r.thing()?,
                }
            }
            id::TELL_ACTOR_ATTACK => Decree::TellActorAttack {
                attacker: r.thing()?,
                target: r.thing()?,
            },
            id::TELL_ACTOR_MOVE => Decree::TellActorMove {
                selector: r.text()?,
                words: [r.uint(32)?, r.uint(32)?, r.uint(32)?],
            },
            id::TELL_ACTOR_ABILITY => Decree::TellActorAbility {
                selector: r.text()?,
                ability: r.def(crate::registries::ABILITIES)?,
                text: r.text()?,
            },
            id::GAME_PLAYER_KINGDOM => Decree::GamePlayerKingdom {
                kingdom: r.thing()?,
            },
            id::FACTION_USE_ABILITY => {
                let ability = r.def(crate::registries::ABILITIES)?;
                let words = [r.uint(32)?, r.uint(32)?];
                let target = if r.uint(1)? == 1 {
                    Some(r.thing()?)
                } else {
                    None
                };
                Decree::FactionUseAbility {
                    ability,
                    words,
                    target,
                }
            }
            id::SET_GAME_SPEED => Decree::SetGameSpeed { bits: r.uint(32)? },
            id::TOGGLE_PAUSED => Decree::TogglePaused,
            id::SET_PAUSED => Decree::SetPaused {
                paused: r.uint(1)? == 1,
            },
            id::SPEED_UP => Decree::SpeedUp,
            id::SPEED_DOWN => Decree::SpeedDown,
            id::UPDATE_KINGDOM_NAME => Decree::UpdateKingdomName {
                kingdom: r.text()?,
                name: r.text()?,
                key: r.text()?,
                flag: r.uint(1)? == 1,
            },
            _ => return None,
        };
        r.done().then_some(out)
    }
    /// The thing slots this decree references (for id-table coverage).
    pub fn thing_slots(&self) -> Vec<u16> {
        match self {
            Decree::SelectionSet { slot }
            | Decree::SelectionAdd { slot }
            | Decree::SelectionRemove { slot } => {
                vec![*slot]
            }
            Decree::TellActorCommand { actor, .. } => vec![*actor],
            Decree::TellActorAttack { attacker, target } => vec![*attacker, *target],
            Decree::GamePlayerKingdom { kingdom } => vec![*kingdom],
            Decree::FactionUseAbility {
                target: Some(t), ..
            } => vec![*t],
            Decree::Other { fields } => fields
                .iter()
                .filter_map(|f| match f {
                    Field::Thing { slot, .. } => Some(*slot),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}
