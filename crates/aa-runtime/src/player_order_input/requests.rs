//! The UI request emitters of the selection manager and the order code.
//!
//! Each native emitter has the same frame: lock `B36524+8` (vt+4 / vt+8),
//! drop the request unless the Admin is running (0x766CD0 -> 0x74B720,
//! Admin+0x2C0), default a NULL player to the local Node+8 (0x76AC60), write
//! the SetPlayer prefix when the player changed (0x766C10 -> 0x74B360), the
//! 9-bit descriptor (0x766D00) and the payload into the request stream
//! (0x766C90 -> 0x74B0D0). That frame is [`AdminNode::emit_request`].
//!
//! | emitter | descriptor | payload |
//! |---|---|---|
//! | 0x436AD0 | 12 SelectionAddOrderRequest (B39EB8) | 0x76A500: `actor+0x14` u16 (0 for NULL) |
//! | 0x436B90 | 13 SelectionRemoveOrderRequest (B39EC4) | same |
//! | 0x436C50 | 14 SelectionClearOrderRequest (B39ED0) | none |
//! | 0x61E960 | 84 KKC_TellSelectedCommandOrderRequest (B3E5A0) | 0x61E6B0 command |
use aa_orders::{
    commands::CommandWire,
    net::admin::AdminNode,
    payload::{Field, Registry, SchemaContext, ThingCheck},
};

pub type Result<T> = std::result::Result<T, String>;

pub const SELECTION_ADD: usize = 12;
pub const SELECTION_REMOVE: usize = 13;
pub const SELECTION_CLEAR: usize = 14;
/// KKC_SaveGroupOrderRequest (descriptor object B39EDC, handler 0x439920,
/// process 0x5F4510 on the decree 19).
pub const SAVE_GROUP: usize = 15;
pub const TELL_SELECTED_COMMAND: usize = aa_orders::commands::SELECTED_REQUEST_ID;

/// The request effects of the selection manager. `player` None is the
/// native NULL argument, resolved to the local player by the emitter.
pub trait Requests<V> {
    /// 0x436AD0 with the view's actor (0x59E240: view+0x14).
    fn add_436ad0(&mut self, view: V, player: Option<u32>) -> Result<()>;
    /// 0x436B90.
    fn remove_436b90(&mut self, view: V, player: Option<u32>) -> Result<()>;
    /// 0x436C50.
    fn clear_436c50(&mut self, player: Option<u32>) -> Result<()>;
    /// 0x438180: KKC_SaveGroupOrder (descriptor 15; its name getter 0x438FF0
    /// is shared with descriptor 14, hence the registry name) with the group
    /// name as text (0x689F40).
    fn save_group_438180(&mut self, name: &[u16], player: Option<u32>) -> Result<()>;
}

/// 0x76A500's 16-bit field for SelectionAdd/Remove (reader 0x76A530 checks
/// virtual +0xB0 on the decree side).
pub fn selection_fields(actor_slot_14: u16) -> Vec<Field> {
    vec![Field::Thing { check: ThingCheck::B0, slot: actor_slot_14 }]
}

/// ActorCommand definitions (B3AA58).
pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
/// Object definitions (B39F58), the flag-1 reference 0x76A880 writes.
pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);

/// The registry counts the two references need (each null sentinel = count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub commands: usize,
    pub objects: usize,
}
fn reference(index: Option<u32>, count: usize, name: &str) -> Result<u32> {
    match index {
        Some(index) if (index as usize) < count => Ok(index),
        Some(index) => Err(format!("{name} index {index} outside {count}")),
        None => u32::try_from(count).map_err(|_| format!("{name} count")),
    }
}

/// 0x61E6B0's field list for a command built by the UI. Bits 5, 7 and 8
/// (parameter sets and the B3AB08/B3AA2C references) have no ported UI
/// producer and are refused.
pub fn command_fields(wire: &CommandWire, counts: Counts) -> Result<Vec<Field>> {
    if wire.flags >= 0x800 {
        return Err("command flags exceed the native eleven-bit field".into());
    }
    let has = |bit: u16| wire.flags & (1 << bit) != 0;
    for bit in [5, 7, 8] {
        if has(bit) {
            return Err(format!("UI command flag bit {bit} has no ported producer"));
        }
    }
    let mut fields = vec![Field::Bounded { n: 0x800, value: u32::from(wire.flags) }];
    if has(6) {
        let index = reference(wire.command, counts.commands, "ActorCommand")?;
        fields.push(Field::Def { registry: ACTOR_COMMANDS, index });
    }
    if has(0) {
        fields.push(Field::Uint { width: 32, value: wire.word });
    }
    if has(1) {
        let index = reference(wire.object, counts.objects, "object definition")?;
        fields.push(Field::Def { registry: OBJECT_DEFINITIONS, index });
    }
    if has(2) {
        fields.extend(wire.position.map(|value| Field::Uint { width: 32, value }));
    }
    if has(3) {
        fields.extend(wire.auxiliary.map(|value| Field::Uint { width: 32, value }));
    }
    if has(4) {
        fields.push(Field::Uint { width: 32, value: wire.scalar });
    }
    if has(9) {
        fields.push(Field::Uint { width: 1, value: u32::from(wire.boolean) });
    }
    fields.push(Field::Uint { width: 1, value: u32::from(wire.trailing) });
    Ok(fields)
}

/// The live emitter: the SAME AdminNode request stream, its running flag
/// (Admin+0x2C0 as 0x766CD0 reads it) and the local Node+8 player.
pub struct AdminRequests<'a, F> {
    pub admin: &'a mut AdminNode,
    pub running_2c0: bool,
    /// Node+8; required only when a NULL player reaches an active emitter.
    pub local_player: Option<u32>,
    pub context: &'a dyn SchemaContext,
    /// 0x59E240 then `actor+0x14` as u16 (0 when the view has no actor).
    pub actor_slot: F,
}
impl<F> AdminRequests<'_, F> {
    fn emit(&mut self, request: usize, fields: &[Field], player: Option<u32>) -> Result<bool> {
        if !self.running_2c0 {
            return Ok(false);
        }
        let player = match player {
            Some(player) => player,
            None => self.local_player.ok_or("0x76AC60 local Node+8 player is NULL")?,
        };
        self.admin
            .emit_request(true, request, fields, player, self.context)
            .map_err(|e| e.to_string())
    }
    /// 0x61E960 for the command 0x61EEF0 sends.
    pub fn tell_selected_61e960(&mut self, wire: &CommandWire, counts: Counts, player: Option<u32>) -> Result<bool> {
        if !self.running_2c0 {
            return Ok(false);
        }
        let fields = command_fields(wire, counts)?;
        self.emit(TELL_SELECTED_COMMAND, &fields, player)
    }
}
impl<V, F: FnMut(V) -> Result<u16>> Requests<V> for AdminRequests<'_, F> {
    fn add_436ad0(&mut self, view: V, player: Option<u32>) -> Result<()> {
        if !self.running_2c0 {
            return Ok(());
        }
        let slot = (self.actor_slot)(view)?;
        self.emit(SELECTION_ADD, &selection_fields(slot), player).map(|_| ())
    }
    fn remove_436b90(&mut self, view: V, player: Option<u32>) -> Result<()> {
        if !self.running_2c0 {
            return Ok(());
        }
        let slot = (self.actor_slot)(view)?;
        self.emit(SELECTION_REMOVE, &selection_fields(slot), player).map(|_| ())
    }
    fn clear_436c50(&mut self, player: Option<u32>) -> Result<()> {
        self.emit(SELECTION_CLEAR, &[], player).map(|_| ())
    }
    fn save_group_438180(&mut self, name: &[u16], player: Option<u32>) -> Result<()> {
        self.emit(SAVE_GROUP, &[Field::Text(name.to_vec())], player).map(|_| ())
    }
}

/// A recording implementation for tests and diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recorded<V> {
    pub calls: Vec<Recorded1<V>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded1<V> {
    Add(V),
    Remove(V),
    Clear,
    SaveGroup(Vec<u16>),
}
impl<V> Requests<V> for Recorded<V> {
    fn add_436ad0(&mut self, view: V, _: Option<u32>) -> Result<()> {
        self.calls.push(Recorded1::Add(view));
        Ok(())
    }
    fn remove_436b90(&mut self, view: V, _: Option<u32>) -> Result<()> {
        self.calls.push(Recorded1::Remove(view));
        Ok(())
    }
    fn clear_436c50(&mut self, _: Option<u32>) -> Result<()> {
        self.calls.push(Recorded1::Clear);
        Ok(())
    }
    fn save_group_438180(&mut self, name: &[u16], _: Option<u32>) -> Result<()> {
        self.calls.push(Recorded1::SaveGroup(name.to_vec()));
        Ok(())
    }
}
