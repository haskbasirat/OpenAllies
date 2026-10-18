//! Phase5 generic command wire fields and guarded process boundaries.
//!
//! Runtime registry indices are not actor IDs or authored enum ordinals. This
//! module preserves wire data; resolving definitions, including native virtual
//! type checks, remains an explicit prerequisite to gameplay delivery.
use crate::{Cursor, Error, Position, Result, native, registry_bits};
use serde::{Deserialize, Serialize};

pub const SELECTED_REQUEST_ID: usize = 84;
pub const ACTOR_REQUEST_ID: usize = 85;
pub const SELECTED_DECREE_ID: usize = 86;
pub const ACTOR_DECREE_ID: usize = 87;
pub const SET_PLAYER_DECREE_ID: usize = 298;
pub const SET_PLAYER_REQUEST_ID: usize = 320;

/// Exact original registry counts, supplied by the caller. Each serializer has
/// an additional null sentinel equal to count. No default or guessed registry
/// order is supplied. The proved scalar domain permits count<=65535.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySizes {
    pub command: usize,
    pub object: usize,
    pub parameter: usize,
    pub parameter_set: usize,
    pub extra: usize,
}
impl RegistrySizes {
    fn validate(self) -> Result<()> {
        for count in [
            self.command,
            self.object,
            self.parameter,
            self.parameter_set,
            self.extra,
        ] {
            registry_bits(count.checked_add(1).ok_or(Error::RegistrySize)?)?;
        }
        Ok(())
    }
}
/// Native parameter lists have a signed32 count and can contain zero-bit object
/// references when the registry is empty. Require an explicit allocation budget
/// even if the remaining packet length cannot limit that particular list.
#[derive(Debug, Clone, Copy)]
pub struct DecodeBudget {
    pub max_parameter_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterSet {
    pub definition: Option<u32>,
    pub objects: Vec<Option<u32>>,
    /// A negative native count (0x4C2C60 reads none, 0x50B530 stores it as
    /// the vector size and 0x4C2050 writes it back). `None` = `objects.len()`.
    #[serde(default)]
    pub negative_count: Option<i32>,
}
/// Native command +8..+3C. Absent fields retain constructor zero values. The
/// eleven-bit mask is retained, including bit10 which has no conditional payload
/// in these routines. `None` is the native count sentinel; it is not permission
/// to invent a definition. Float fields remain exact raw IEEE bits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandWire {
    pub flags: u16,
    pub command: Option<u32>,
    pub word: u32,
    pub object: Option<u32>,
    pub position: [u32; 2],
    pub auxiliary: [u32; 2],
    pub scalar: u32,
    pub parameter: Option<u32>,
    pub parameters: Option<ParameterSet>,
    pub extra: Option<u32>,
    pub boolean: bool,
    pub trailing: bool,
}
fn read_reference(cursor: &mut Cursor<'_>, count: usize) -> Result<Option<u32>> {
    let value = cursor.read_possibility(count.checked_add(1).ok_or(Error::RegistrySize)?)?;
    Ok((value as usize != count).then_some(value))
}
impl CommandWire {
    fn has(&self, bit: u8) -> bool {
        self.flags & (1 << bit) != 0
    }
    /// Exact field order of 0x61EBA0. Native pointer resolution is deliberately
    /// separated: object references additionally call virtual+38 in the engine.
    /// Decoding alone does not prove those definitions are present or eligible.
    pub fn decode(
        cursor: &mut Cursor<'_>,
        sizes: RegistrySizes,
        budget: DecodeBudget,
    ) -> Result<Self> {
        sizes.validate()?;
        let mut value = Self {
            flags: cursor.read_possibility(0x800)? as u16,
            ..Self::default()
        };
        if value.has(6) {
            value.command = read_reference(cursor, sizes.command)?;
        }
        if value.has(0) {
            value.word = cursor.read_uint(32)?;
        }
        if value.has(1) {
            value.object = read_reference(cursor, sizes.object)?;
        }
        if value.has(2) {
            value.position = [cursor.read_uint(32)?, cursor.read_uint(32)?];
        }
        if value.has(3) {
            value.auxiliary = [cursor.read_uint(32)?, cursor.read_uint(32)?];
        }
        if value.has(4) {
            value.scalar = cursor.read_uint(32)?;
        }
        if value.has(7) {
            value.parameter = read_reference(cursor, sizes.parameter)?;
        }
        if value.has(5) && cursor.read_uint(1)? == 0 {
            let definition = read_reference(cursor, sizes.parameter_set)?;
            let raw_count = cursor.read_uint(32)?;
            // Signed native loop `0 < count`: negative counts read no entries.
            let negative_count = (raw_count as i32 <= 0 && raw_count != 0).then_some(raw_count as i32);
            let count = if negative_count.is_some() { 0 } else { raw_count as usize };
            if count > budget.max_parameter_entries {
                return Err(Error::ParameterBudget {
                    requested: count,
                    budget: budget.max_parameter_entries,
                });
            }
            let width = registry_bits(sizes.object + 1)? as usize;
            let requested = count.checked_mul(width).ok_or(Error::Allocation)?;
            if requested > cursor.remaining_bits() {
                return Err(Error::Truncated {
                    requested,
                    remaining: cursor.remaining_bits(),
                });
            }
            let mut objects = Vec::new();
            objects
                .try_reserve_exact(count)
                .map_err(|_| Error::Allocation)?;
            for _ in 0..count {
                objects.push(read_reference(cursor, sizes.object)?);
            }
            value.parameters = Some(ParameterSet {
                definition,
                objects,
                negative_count,
            });
        }
        if value.has(8) {
            value.extra = read_reference(cursor, sizes.extra)?;
        }
        if value.has(9) {
            value.boolean = cursor.read_uint(1)? == 1;
        }
        value.trailing = cursor.read_uint(1)? == 1;
        Ok(value)
    }
    /// Original writer0x61E6B0 field sequence. This is a raw serializer, not an
    /// authoritative packet scheduler/finalizer. Omitted field values are ignored
    /// as in the native writer. Failure can leave earlier fields written.
    pub fn encode(&self, writer: &mut WireWriter, sizes: RegistrySizes) -> Result<()> {
        sizes.validate()?;
        if self.flags >= 0x800 {
            return Err(Error::CommandFlags);
        }
        writer.write_uint(self.flags as u32, 11)?;
        if self.has(6) {
            writer.reference(self.command, sizes.command)?;
        }
        if self.has(0) {
            writer.write_uint(self.word, 32)?;
        }
        if self.has(1) {
            writer.reference(self.object, sizes.object)?;
        }
        if self.has(2) {
            for v in self.position {
                writer.write_uint(v, 32)?;
            }
        }
        if self.has(3) {
            for v in self.auxiliary {
                writer.write_uint(v, 32)?;
            }
        }
        if self.has(4) {
            writer.write_uint(self.scalar, 32)?;
        }
        if self.has(7) {
            writer.reference(self.parameter, sizes.parameter)?;
        }
        if self.has(5) {
            writer.write_uint(u32::from(self.parameters.is_none()), 1)?;
            if let Some(parameters) = &self.parameters {
                writer.reference(parameters.definition, sizes.parameter_set)?;
                let count = match parameters.negative_count {
                    Some(n) if n < 0 && parameters.objects.is_empty() => n as u32,
                    Some(_) => return Err(Error::ParameterCount(parameters.objects.len() as u32)),
                    None => {
                        let n = u32::try_from(parameters.objects.len()).map_err(|_| Error::Allocation)?;
                        if n > i32::MAX as u32 {
                            return Err(Error::ParameterCount(n));
                        }
                        n
                    }
                };
                writer.write_uint(count, 32)?;
                for object in &parameters.objects {
                    writer.reference(*object, sizes.object)?;
                }
            }
        }
        if self.has(8) {
            writer.reference(self.extra, sizes.extra)?;
        }
        if self.has(9) {
            writer.write_uint(u32::from(self.boolean), 1)?;
        }
        writer.write_uint(u32::from(self.trailing), 1)
    }
}

/// The generic actor order carries only the low16 slot, with no generation.
/// Native0x76A5A0 indexes the slot then requires virtual+B4; preserve that lookup
/// contract rather than treating this as a full generation-tagged ActorId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSlotReference(pub u16);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCommandWire {
    pub command: CommandWire,
    pub actor: ActorSlotReference,
}
impl ActorCommandWire {
    pub fn decode(cursor: &mut Cursor<'_>, sizes: RegistrySizes, budget: DecodeBudget) -> Result<Self> {
        let command = CommandWire::decode(cursor, sizes, budget)?;
        let actor = ActorSlotReference(cursor.read_uint(16)? as u16);
        Ok(Self { command, actor })
    }
    pub fn encode(&self, writer: &mut WireWriter, sizes: RegistrySizes) -> Result<()> {
        self.command.encode(writer, sizes)?;
        writer.write_uint(self.actor.0 as u32, 16)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    SelectedRequest(CommandWire),
    SelectedDecree(CommandWire),
    ActorRequest(ActorCommandWire),
    ActorDecree(ActorCommandWire),
    SetPlayerRequest(u32),
    SetPlayerDecree(u32),
    Movement(native::KnownPayload),
}
/// Requests remain explicitly different from accepted decrees. IDs are the
/// native 358-entry registry. No implicit request authorization.
pub fn decode_payload(
    descriptor: usize,
    cursor: &mut Cursor<'_>,
    sizes: RegistrySizes,
    budget: DecodeBudget,
) -> Result<Payload> {
    match descriptor {
        SELECTED_REQUEST_ID => {
            CommandWire::decode(cursor, sizes, budget).map(Payload::SelectedRequest)
        }
        SELECTED_DECREE_ID => {
            CommandWire::decode(cursor, sizes, budget).map(Payload::SelectedDecree)
        }
        ACTOR_REQUEST_ID | ACTOR_DECREE_ID => {
            let value = ActorCommandWire::decode(cursor, sizes, budget)?;
            Ok(if descriptor == ACTOR_REQUEST_ID {
                Payload::ActorRequest(value)
            } else {
                Payload::ActorDecree(value)
            })
        }
        SET_PLAYER_REQUEST_ID => cursor.read_uint(32).map(Payload::SetPlayerRequest),
        SET_PLAYER_DECREE_ID => cursor.read_uint(32).map(Payload::SetPlayerDecree),
        other => native::decode_payload(other, cursor).map(Payload::Movement),
    }
}

/// Exact0x40D420 raw buffer packing, including its unusual whole-byte/fragment
/// order. Storage starts zeroed, grows fallibly, and never rewrites earlier data.
/// No native packet-header finalization or network transmission is implied.
#[derive(Debug)]
pub struct WireWriter {
    bytes: Vec<u8>,
    position: Position,
}
impl WireWriter {
    pub fn new(position: Position) -> Result<Self> {
        let size = position.absolute()?.div_ceil(8).max(1);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation)?;
        bytes.resize(size, 0);
        Ok(Self { bytes, position })
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn reference(&mut self, index: Option<u32>, count: usize) -> Result<()> {
        let possibilities = count.checked_add(1).ok_or(Error::RegistrySize)?;
        let width = registry_bits(possibilities)?;
        let value = index.unwrap_or(count as u32);
        if index.is_some() && value as usize >= count {
            return Err(Error::RegistryIndex {
                index: value,
                count: possibilities,
            });
        }
        self.write_uint(value, width)
    }
    pub fn write_uint(&mut self, value: u32, width: u8) -> Result<()> {
        if width > 32 {
            return Err(Error::ScalarWidth(width));
        }
        self.write_raw_bits(&value.to_le_bytes(), width as usize)
    }
    pub fn write_raw_bits(&mut self, source: &[u8], requested: usize) -> Result<()> {
        if requested.div_ceil(8) > source.len() {
            return Err(Error::WriteSource);
        }
        if requested == 0 {
            return Ok(());
        }
        let end = self
            .position
            .absolute()?
            .checked_add(requested)
            .ok_or(Error::Allocation)?;
        let size = end.div_ceil(8);
        if size > self.bytes.len() {
            self.bytes
                .try_reserve_exact(size - self.bytes.len())
                .map_err(|_| Error::Allocation)?;
            self.bytes.resize(size, 0);
        }
        let mask = |n: usize| -> u8 { ((1u16 << n) - 1) as u8 };
        let available = 8 - self.position.bit as usize;
        if requested <= available {
            self.bytes[self.position.byte] |= (source[0] & mask(requested)) << self.position.bit;
            self.position.bit += requested as u8;
        } else {
            let remaining = requested - available;
            let whole = remaining / 8;
            self.bytes[self.position.byte] |= ((source.get(whole).copied().unwrap_or(0) as u16
                & mask(available) as u16)
                << self.position.bit) as u8;
            self.bytes[self.position.byte + 1..self.position.byte + 1 + whole]
                .copy_from_slice(&source[..whole]);
            self.position.byte += whole;
            let tail = remaining % 8;
            if tail == 0 {
                self.position.bit = 8;
            } else {
                self.position.byte += 1;
                self.position.bit = tail as u8;
                if available + tail <= 8 {
                    self.bytes[self.position.byte] |=
                        ((source[whole] as u16 >> available) as u8) & mask(tail);
                } else {
                    let low = 8 - available;
                    self.bytes[self.position.byte] |=
                        ((source[whole] as u16 >> available) as u8) & mask(low);
                    self.bytes[self.position.byte] |= (source[whole + 1] & mask(tail - low)) << low;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorOutcome {
    ActorNotFound,
    PermissionDenied,
    Delivered,
}
/// Required leaves after wire references have been resolved. `Command` is the
/// caller's resolved command representation, not a disguised raw pointer. The
/// permit callback may replace it, matching native smart-pointer semantics.
pub trait ActorCommandContext {
    type Actor;
    type Command;
    type Group;
    fn resolve_slot(&mut self, slot: ActorSlotReference) -> Option<Self::Actor>;
    /// Original virtual+150 arguments are null player and selection=false.
    fn permit(&mut self, actor: &Self::Actor, command: &mut Self::Command) -> bool;
    /// Exact linked-GTID validation0x44DEA0 is an external actor-registry query.
    fn has_valid_link(&mut self, actor: &Self::Actor) -> bool;
    fn linked_group(&mut self, actor: &Self::Actor) -> Self::Group;
    fn prepare_group(&mut self, group: Self::Group);
    /// Return ignored by native0x61E220; delivery does not imply acceptance.
    fn deliver(&mut self, actor: &Self::Actor, command: &mut Self::Command);
}
pub fn process_actor<C: ActorCommandContext>(
    slot: ActorSlotReference,
    command: &mut C::Command,
    context: &mut C,
) -> ActorOutcome {
    match try_process_actor(slot, command, &mut InfallibleActor(context)) {
        Ok(value) => value,
        Err(never) => match never {},
    }
}
/// Fallible61E220 leaves over real storage. A required child failure commits
/// the native prefix and does not become a false permission/delivery result.
pub trait TryActorCommandContext {
    type Actor;
    type Command;
    type Group;
    type Error;
    fn resolve_slot(
        &mut self,
        slot: ActorSlotReference,
    ) -> std::result::Result<Option<Self::Actor>, Self::Error>;
    fn permit(
        &mut self,
        actor: &Self::Actor,
        command: &mut Self::Command,
    ) -> std::result::Result<bool, Self::Error>;
    fn has_valid_link(&mut self, actor: &Self::Actor) -> std::result::Result<bool, Self::Error>;
    fn linked_group(
        &mut self,
        actor: &Self::Actor,
    ) -> std::result::Result<Self::Group, Self::Error>;
    fn prepare_group(&mut self, group: Self::Group) -> std::result::Result<(), Self::Error>;
    fn deliver(
        &mut self,
        actor: &Self::Actor,
        command: &mut Self::Command,
    ) -> std::result::Result<(), Self::Error>;
}
struct InfallibleActor<'a, C>(&'a mut C);
impl<C: ActorCommandContext> TryActorCommandContext for InfallibleActor<'_, C> {
    type Actor = C::Actor;
    type Command = C::Command;
    type Group = C::Group;
    type Error = std::convert::Infallible;
    fn resolve_slot(
        &mut self,
        slot: ActorSlotReference,
    ) -> std::result::Result<Option<Self::Actor>, Self::Error> {
        Ok(self.0.resolve_slot(slot))
    }
    fn permit(
        &mut self,
        actor: &Self::Actor,
        command: &mut Self::Command,
    ) -> std::result::Result<bool, Self::Error> {
        Ok(self.0.permit(actor, command))
    }
    fn has_valid_link(&mut self, actor: &Self::Actor) -> std::result::Result<bool, Self::Error> {
        Ok(self.0.has_valid_link(actor))
    }
    fn linked_group(
        &mut self,
        actor: &Self::Actor,
    ) -> std::result::Result<Self::Group, Self::Error> {
        Ok(self.0.linked_group(actor))
    }
    fn prepare_group(&mut self, group: Self::Group) -> std::result::Result<(), Self::Error> {
        self.0.prepare_group(group);
        Ok(())
    }
    fn deliver(
        &mut self,
        actor: &Self::Actor,
        command: &mut Self::Command,
    ) -> std::result::Result<(), Self::Error> {
        self.0.deliver(actor, command);
        Ok(())
    }
}
pub fn try_process_actor<C: TryActorCommandContext>(
    slot: ActorSlotReference,
    command: &mut C::Command,
    context: &mut C,
) -> std::result::Result<ActorOutcome, C::Error> {
    let Some(actor) = context.resolve_slot(slot)? else {
        return Ok(ActorOutcome::ActorNotFound);
    };
    if !context.permit(&actor, command)? {
        return Ok(ActorOutcome::PermissionDenied);
    }
    if context.has_valid_link(&actor)? {
        let group = context.linked_group(&actor)?;
        context.prepare_group(group)?;
    }
    context.deliver(&actor, command)?;
    Ok(ActorOutcome::Delivered)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedOutcome {
    EmptySelection,
    DeliveredToSelection,
}
/// Boundary of0x61F210 only. 0x4ED360 performs formation/default-order/ownership
/// processing; it is deliberately a required callback, not a loop that simply
/// sends the same command to selected actors.
pub trait SelectedCommandContext {
    type Selection;
    type Command;
    fn current_selection(&mut self) -> Option<Self::Selection>;
    fn is_empty(&mut self, selection: &Self::Selection) -> bool;
    /// Preserve the original diagnostic/default-definition inspection boundary.
    /// Native first-actor virtual queries and0x4DA570 are not assumed pure here.
    fn inspect_selected(&mut self, selection: &Self::Selection, command: &mut Self::Command);
    fn deliver_selection(&mut self, selection: &Self::Selection, command: &mut Self::Command);
}
pub fn process_selected<C: SelectedCommandContext>(
    command: &mut C::Command,
    context: &mut C,
) -> Result<SelectedOutcome> {
    try_process_selected(command, &mut PortableSelected(context))
}
/// Fallible61F210 leaves. The live adapter must provide the empty-selection
/// diagnostic as well; the old portable context has no logging owner.
pub trait TrySelectedCommandContext {
    type Selection;
    type Command;
    type Error: From<Error>;
    fn current_selection(&mut self) -> std::result::Result<Option<Self::Selection>, Self::Error>;
    fn is_empty(&mut self, selection: &Self::Selection) -> std::result::Result<bool, Self::Error>;
    fn inspect_empty(
        &mut self,
        selection: &Self::Selection,
        command: &mut Self::Command,
    ) -> std::result::Result<(), Self::Error>;
    fn inspect_selected(
        &mut self,
        selection: &Self::Selection,
        command: &mut Self::Command,
    ) -> std::result::Result<(), Self::Error>;
    fn deliver_selection(
        &mut self,
        selection: &Self::Selection,
        command: &mut Self::Command,
    ) -> std::result::Result<(), Self::Error>;
}
struct PortableSelected<'a, C>(&'a mut C);
impl<C: SelectedCommandContext> TrySelectedCommandContext for PortableSelected<'_, C> {
    type Selection = C::Selection;
    type Command = C::Command;
    type Error = Error;
    fn current_selection(&mut self) -> Result<Option<Self::Selection>> {
        Ok(self.0.current_selection())
    }
    fn is_empty(&mut self, selection: &Self::Selection) -> Result<bool> {
        Ok(self.0.is_empty(selection))
    }
    fn inspect_empty(&mut self, _: &Self::Selection, _: &mut Self::Command) -> Result<()> {
        Ok(())
    }
    fn inspect_selected(
        &mut self,
        selection: &Self::Selection,
        command: &mut Self::Command,
    ) -> Result<()> {
        self.0.inspect_selected(selection, command);
        Ok(())
    }
    fn deliver_selection(
        &mut self,
        selection: &Self::Selection,
        command: &mut Self::Command,
    ) -> Result<()> {
        self.0.deliver_selection(selection, command);
        Ok(())
    }
}
pub fn try_process_selected<C: TrySelectedCommandContext>(
    command: &mut C::Command,
    context: &mut C,
) -> std::result::Result<SelectedOutcome, C::Error> {
    let selection = context
        .current_selection()?
        .ok_or_else(|| C::Error::from(Error::MissingCurrentPlayer))?;
    if context.is_empty(&selection)? {
        context.inspect_empty(&selection, command)?;
        return Ok(SelectedOutcome::EmptySelection);
    }
    context.inspect_selected(&selection, command)?;
    context.deliver_selection(&selection, command)?;
    Ok(SelectedOutcome::DeliveredToSelection)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerOutcome {
    Selected,
    Cleared,
    SecurityRejected,
    MissingFilmPlayer,
}
/// Native0x75D710 guard order. The lookup must match both exact player ID and
/// film flag in original list order. Live player binding updates the player's
/// client+2C, then admin+284; film players do not change that client association.
pub trait PlayerContext {
    type Player;
    type Client: PartialEq;
    fn film_mode(&mut self) -> bool;
    fn find_player(&mut self, id: u32, film: bool) -> Option<Self::Player>;
    fn player_is_film(&mut self, player: &Self::Player) -> bool;
    fn is_authorizing_admin(&mut self) -> bool;
    fn player_client(&mut self, player: &Self::Player) -> Self::Client;
    fn ordering_client(&mut self) -> Self::Client;
    fn bind_client_player(&mut self, client: Self::Client, player: &Self::Player);
    fn set_current_player(&mut self, player: Option<Self::Player>);
    ///0x751EB0 unported film-stop/event callback. It runs before the abort store.
    fn stop_film(&mut self);
    fn set_abort(&mut self);
}
/// Fallible75D710 service adapter. Every source/identity dependency is read
/// lazily at the native point; a refused dependency preserves prior stores.
/// The old portable API delegates to this same coordinator.
pub trait TryPlayerContext {
    type Player;
    type Client: PartialEq;
    type Error;
    fn film_mode(&mut self)->std::result::Result<bool,Self::Error>;
    fn find_player(&mut self,id:u32,film:bool)->std::result::Result<Option<Self::Player>,Self::Error>;
    fn player_is_film(&mut self,player:&Self::Player)->std::result::Result<bool,Self::Error>;
    fn is_authorizing_admin(&mut self)->std::result::Result<bool,Self::Error>;
    fn player_client(&mut self,player:&Self::Player)->std::result::Result<Self::Client,Self::Error>;
    fn ordering_client(&mut self)->std::result::Result<Self::Client,Self::Error>;
    fn bind_client_player(&mut self,client:Self::Client,player:&Self::Player)->std::result::Result<(),Self::Error>;
    fn set_current_player(&mut self,player:Option<Self::Player>)->std::result::Result<(),Self::Error>;
    fn stop_film(&mut self)->std::result::Result<(),Self::Error>;
    fn set_abort(&mut self)->std::result::Result<(),Self::Error>;
}
struct InfalliblePlayer<'a,C>(&'a mut C);
impl<C:PlayerContext> TryPlayerContext for InfalliblePlayer<'_,C> {
    type Player=C::Player; type Client=C::Client; type Error=std::convert::Infallible;
    fn film_mode(&mut self)->std::result::Result<bool,Self::Error>{Ok(self.0.film_mode())}
    fn find_player(&mut self,id:u32,film:bool)->std::result::Result<Option<Self::Player>,Self::Error>{Ok(self.0.find_player(id,film))}
    fn player_is_film(&mut self,p:&Self::Player)->std::result::Result<bool,Self::Error>{Ok(self.0.player_is_film(p))}
    fn is_authorizing_admin(&mut self)->std::result::Result<bool,Self::Error>{Ok(self.0.is_authorizing_admin())}
    fn player_client(&mut self,p:&Self::Player)->std::result::Result<Self::Client,Self::Error>{Ok(self.0.player_client(p))}
    fn ordering_client(&mut self)->std::result::Result<Self::Client,Self::Error>{Ok(self.0.ordering_client())}
    fn bind_client_player(&mut self,c:Self::Client,p:&Self::Player)->std::result::Result<(),Self::Error>{self.0.bind_client_player(c,p);Ok(())}
    fn set_current_player(&mut self,p:Option<Self::Player>)->std::result::Result<(),Self::Error>{self.0.set_current_player(p);Ok(())}
    fn stop_film(&mut self)->std::result::Result<(),Self::Error>{self.0.stop_film();Ok(())}
    fn set_abort(&mut self)->std::result::Result<(),Self::Error>{self.0.set_abort();Ok(())}
}
pub fn process_set_player<C:PlayerContext>(id:u32,context:&mut C)->PlayerOutcome {
    try_process_set_player(id,&mut InfalliblePlayer(context)).unwrap_or_else(|never|match never{})
}
pub fn try_process_set_player<C:TryPlayerContext>(id:u32,context:&mut C)->std::result::Result<PlayerOutcome,C::Error> {
    let film = context.film_mode()?;
    let player = context.find_player(id, film)?;
    if let Some(player) = player {
        if !context.player_is_film(&player)? {
            if context.is_authorizing_admin()? {
                let owner = context.player_client(&player)?;
                if owner != context.ordering_client()? {
                    return Ok(PlayerOutcome::SecurityRejected);
                }
            }
            let owner = context.player_client(&player)?;
            context.bind_client_player(owner, &player)?;
        }
        context.set_current_player(Some(player))?;
        Ok(PlayerOutcome::Selected)
    } else if context.film_mode()? {
        context.stop_film()?;
        context.set_abort()?;
        Ok(PlayerOutcome::MissingFilmPlayer)
    } else {
        context.set_current_player(None)?;
        Ok(PlayerOutcome::Cleared)
    }
}

/// Actor-graph queries of KKC_GActorCommand::Process 0x4D9B60, the delivery
/// of a constructed actor command (TellActorCommand decree, combat special
/// operations and summons, triggers, minds, navigation).
pub trait CommandRouting {
    type Actor: Copy + PartialEq;
    type Handler;
    /// `[actor+0x60+4*slot]` (slot = `[[command+8]+0x18]`, the command
    /// type's handler slot); `None` for a null entry.
    fn handler(&mut self, actor: Self::Actor, slot: u32) -> Option<Self::Handler>;
    /// Actor virtual +0x38(command type).
    fn accepts(&mut self, actor: Self::Actor) -> bool;
    /// Handler virtual +0x24(command); the native result is its low byte.
    fn handle(&mut self, handler: Self::Handler) -> u8;
    /// Actor virtual +0x158: `Some(virtual +0x15C())` when it redirects
    /// (the target may be null), `None` otherwise.
    fn redirect(&mut self, actor: Self::Actor) -> Option<Option<Self::Actor>>;
    /// `[[actor+0x7C]+0x44]`: `None` for a null container or member.
    fn container_member(&mut self, actor: Self::Actor) -> Option<Self::Actor>;
}

/// Result of 0x4D9B60.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDelivery {
    /// Returned byte: the handler's result, or 0.
    pub result: u8,
    /// "ignoring command to null actor" (GActorCommands.cpp line 0xF7,
    /// 0x404F30): the start actor or a redirect target was null.
    pub null_actor_logged: bool,
}

/// Fallible form of the same native4D9B60 leaves. Runtime adapters must
/// propagate a required service error before the next actor is queried.
pub trait TryCommandRouting {
    type Actor: Copy + PartialEq;
    type Handler;
    type Error: From<Error>;
    fn handler(&mut self, actor: Self::Actor, slot: u32) -> std::result::Result<Option<Self::Handler>, Self::Error>;
    fn accepts(&mut self, actor: Self::Actor) -> std::result::Result<bool, Self::Error>;
    fn handle(&mut self, handler: Self::Handler) -> std::result::Result<u8, Self::Error>;
    fn redirect(&mut self, actor: Self::Actor) -> std::result::Result<Option<Option<Self::Actor>>, Self::Error>;
    fn container_member(&mut self, actor: Self::Actor) -> std::result::Result<Option<Self::Actor>, Self::Error>;
}
struct InfallibleRouting<'a, R>(&'a mut R);
impl<R: CommandRouting> TryCommandRouting for InfallibleRouting<'_, R> {
    type Actor = R::Actor;
    type Handler = R::Handler;
    type Error = Error;
    fn handler(&mut self,a:Self::Actor,s:u32)->Result<Option<Self::Handler>> { Ok(self.0.handler(a,s)) }
    fn accepts(&mut self,a:Self::Actor)->Result<bool> { Ok(self.0.accepts(a)) }
    fn handle(&mut self,h:Self::Handler)->Result<u8> { Ok(self.0.handle(h)) }
    fn redirect(&mut self,a:Self::Actor)->Result<Option<Option<Self::Actor>>> { Ok(self.0.redirect(a)) }
    fn container_member(&mut self,a:Self::Actor)->Result<Option<Self::Actor>> { Ok(self.0.container_member(a)) }
}
/// KKC_GActorCommand::Process 0x4D9B60. A cycle in the redirect /
/// container walk (an endless loop in the original) is a typed error.
pub fn deliver_actor_command<R: CommandRouting>(
    routing: &mut R,
    slot: u32,
    actor: Option<R::Actor>,
) -> Result<CommandDelivery> {
    try_deliver_actor_command(&mut InfallibleRouting(routing), slot, actor)
}
/// Same4D9B60 traversal over required fallible runtime services. The old API
/// uses the infallible shim above; there is only one traversal algorithm.
pub fn try_deliver_actor_command<R: TryCommandRouting>(
    routing: &mut R,
    slot: u32,
    actor: Option<R::Actor>,
) -> std::result::Result<CommandDelivery, R::Error> {
    let logged = CommandDelivery { result: 0, null_actor_logged: true };
    let quiet = CommandDelivery { result: 0, null_actor_logged: false };
    let Some(mut actor) = actor else { return Ok(logged) };
    let mut seen: Vec<R::Actor> = Vec::new();
    loop {
        if seen.contains(&actor) {
            return Err(Error::CommandRoutingCycle.into());
        }
        seen.push(actor);
        if let Some(handler) = routing.handler(actor, slot)? {
            if !routing.accepts(actor)? {
                return Ok(quiet);
            }
            return Ok(CommandDelivery { result: routing.handle(handler)?, null_actor_logged: false });
        }
        match routing.redirect(actor)? {
            Some(Some(next)) => actor = next,
            Some(None) => return Ok(logged),
            None => match routing.container_member(actor)? {
                Some(next) => actor = next,
                None => return Ok(quiet),
            },
        }
    }
}
