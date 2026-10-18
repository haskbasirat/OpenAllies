//! Original `KKC_GWorld::CreateGThing` (0x4F9C10), the single fresh-creation
//! entry point every system uses (construction completion, member spawn
//! 4FBF40, class-changed record replacement, triggers, ability summons).
//! This module is its control flow; each native callee is a [`CreateWorld`]
//! method named after the original, called in the original order.
//! Oracle: research/phase8-restoration/native-create-gthing.json (the
//! original x86 with every callee seamed and recorded).
use crate::components::ComponentSlot;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The original dereferences a null object (unknown class, failed
    /// allocation) — refused explicitly.
    #[error("native null dereference in 4F9C10 ({0})")]
    NullDereference(&'static str),
    #[error("creation world callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// The fields of the creation-parameter object (vtables A25B1C / A25B28)
/// that 4F9C10 itself reads. `payload` is the rest, consumed by the class
/// constructors and the object virtuals through the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRequest<L, P> {
    /// Definition `+18` category `+1C` (runtime class).
    pub class: u32,
    /// Definition `+8` IDS (log argument).
    pub definition_ids: String,
    /// Params `+1C` instance IDS; empty when none.
    pub instance_name: Vec<u16>,
    /// Params virtual 0: actor creation data (GACD).
    pub actor_data: bool,
    /// Params virtual 4: kingdom creation data (GKCD).
    pub kingdom_data: bool,
    /// Params `+2C`: Organization layout (actor class).
    pub organization_layout: Option<L>,
    /// Params `+38` creation mode (1: state 0x15; 3: Body first, state 0x1A).
    pub mode: u32,
    /// Params `+50` (state 0x18).
    pub byte_50: bool,
    pub payload: P,
}

/// Native allocation size per class (6F7810 before the constructor).
pub fn object_size(class: u32) -> Option<u32> {
    Some(match class {
        2 => 0x60,
        3 => 0x180,
        4 => 0x378,
        5 => 0x6c,
        6 => 0x8c,
        7 => 0x68,
        8 => 0x1c0,
        9 => 0x68,
        10 => 0x68,
        _ => return None,
    })
}

/// Every native call 4F9C10 makes, in its original names.
pub trait CreateWorld<L, P> {
    type Object: Copy + PartialEq;
    /// `4154D0` on GWorld `+F0`: the entry's `+4` GTID for an instance IDS.
    fn instance_name_gtid(&mut self, name: &[u16]) -> Result<Option<u32>>;
    /// IDMGR: the GTID's generation matches and its object is live.
    fn gtid_live(&self, gtid: u32) -> Result<bool>;
    /// `404ED0` (debug log B38B70, GWorld.cpp `line`).
    fn log(&mut self, line: u32, format: &'static str, argument: &str) -> Result<()>;
    /// `6F7810(size)` then the class constructor on the request; `None` is a
    /// failed allocation.
    fn construct(
        &mut self,
        class: u32,
        size: u32,
        request: &CreateRequest<L, P>,
    ) -> Result<Option<Self::Object>>;
    /// `4D64F0(state, enabled)`.
    fn set_state(&mut self, object: Self::Object, state: u8, enabled: bool) -> Result<()>;
    /// `4CF480(slot)`: construct the component into actor `+60[slot]`.
    fn create_component(&mut self, object: Self::Object, slot: ComponentSlot) -> Result<()>;
    /// Actor `+4` ActorData `+174`.
    fn definition_flags(&self, object: Self::Object) -> Result<u32>;
    /// `B3CC42`: the construction-state assertion latch.
    fn construction_assert_latch(&self) -> Result<bool>;
    fn set_construction_assert_latch(&mut self, value: bool) -> Result<()>;
    /// `432E90(state)`.
    fn state_active(&mut self, object: Self::Object, state: u8) -> Result<bool>;
    /// `855D30` assertion dialog (GWorld.cpp line 0x814); its result is the
    /// new latch value.
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> Result<bool>;
    /// `57C450(layout)` on the actor's Organization (`+7C`).
    fn set_organization_layout(&mut self, object: Self::Object, layout: &L) -> Result<()>;
    /// Object virtual `+E4(params)` (GActor 4D6F10 InitCreate).
    fn init_create(&mut self, object: Self::Object, request: &CreateRequest<L, P>) -> Result<bool>;
    /// Object virtual `+E8(params)` (GActor 4D7230).
    fn init_existing(&mut self, object: Self::Object, request: &CreateRequest<L, P>)
    -> Result<bool>;
    /// Object virtual `+EC(params, flag)` (GActor 4CF300).
    fn post_create(
        &mut self,
        object: Self::Object,
        request: &CreateRequest<L, P>,
        flag: u32,
    ) -> Result<()>;
    /// `4F9510`: GWorld registration (instance name, type lists).
    fn register(&mut self, object: Self::Object) -> Result<()>;
    /// `50C220` on GWorld `+130`.
    fn register_updatable(&mut self, object: Self::Object) -> Result<()>;
    /// Object virtual `+B0`.
    fn fires_creation_fx(&mut self, object: Self::Object) -> Result<bool>;
    /// `42A590(object, 0)` on definition `+BC`.
    fn creation_fx(&mut self, object: Self::Object) -> Result<()>;
    /// `4F5490`: destroy a created object whose initialization failed.
    fn destroy(&mut self, object: Self::Object) -> Result<()>;
}

const ASSERTION: &str = "!gactor->IsStateActive(KKE_ACTORSTATE_CONSTRUCTION)";

/// `0x4F9C10`. `Ok(None)` where the original returns null; errors where it
/// dereferences null or a world callback fails (effects before an error stay).
pub fn create<L, P, W: CreateWorld<L, P>>(
    request: &CreateRequest<L, P>,
    world: &mut W,
) -> Result<Option<W::Object>> {
    if !request.instance_name.is_empty()
        && let Some(gtid) = world.instance_name_gtid(&request.instance_name)?
        && world.gtid_live(gtid)?
    {
        world.log(
            0x7cf,
            "Instance IDS already used: %s",
            &String::from_utf16_lossy(&request.instance_name),
        )?;
        return Ok(None);
    }
    let size = object_size(request.class)
        .ok_or(Error::NullDereference("class outside 2..10: virtual E4 on null"))?;
    if matches!(request.class, 3 | 8) && !request.actor_data {
        let (line, format) = if request.class == 3 {
            (0x7f6, "Actor class requires a GACD: %s")
        } else {
            (0x82a, "Group class requires a GACD: %s")
        };
        world.log(line, format, &request.definition_ids)?;
        return Ok(None);
    }
    if request.class == 4 && !request.kingdom_data {
        world.log(0x821, "Kingdom class requires a GKCD: %s", &request.definition_ids)?;
        return Ok(None);
    }
    let object = world
        .construct(request.class, size, request)?
        .ok_or(Error::NullDereference("failed allocation: the null object is used"))?;
    if request.class == 3 {
        if request.mode == 1 {
            world.set_state(object, 0x15, true)?;
        }
        if request.mode == 3 {
            world.create_component(object, ComponentSlot::Body)?;
            world.set_state(object, 0x1a, true)?;
        }
        if request.byte_50 {
            world.set_state(object, 0x18, true)?;
        }
        if let Some(layout) = &request.organization_layout
            && world.definition_flags(object)? & (1 << 7) != 0
        {
            if !world.construction_assert_latch()? && world.state_active(object, 0x15)? {
                let latch = world.assertion_dialog(ASSERTION, 0x814)?;
                world.set_construction_assert_latch(latch)?;
            }
            world.create_component(object, ComponentSlot::Organization)?;
            world.set_organization_layout(object, layout)?;
        }
    }
    if world.init_create(object, request)? && world.init_existing(object, request)? {
        world.post_create(object, request, 0)?;
        world.register(object)?;
        world.register_updatable(object)?;
        if world.fires_creation_fx(object)? {
            world.creation_fx(object)?;
        }
        return Ok(Some(object));
    }
    world.destroy(object)?;
    Ok(None)
}

/// Organization member creation `0x4FBF40` (GWorld; params, organization
/// actor, slot): outside game mode 3 and with params `+28` set, the nearest
/// related recruit structure (definition `+3EC`) whose definition is in the
/// params definition's `+17C` list, within Organization `+AC` of the actor,
/// creates the member (`4FACA0`, after params `+40` = actor GTID) which
/// `57D860` seats at `slot`; otherwise the slot position (Organization
/// `+BC/+C0` + `+78[slot]`) must pass `578290` and `57D950` creates it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnRequest<D> {
    /// Params `+4` definition `+17C` list (structure definitions), in order.
    pub structure_definitions: Vec<D>,
    /// Params `+28`.
    pub word_28: u32,
    /// Params `+50` / `+51`.
    pub byte_50: u8,
    pub byte_51: u8,
}

pub trait SpawnWorld<D> {
    type Object: Copy + PartialEq;
    type Owner: Copy;
    /// `[B36550]+E4`.
    fn game_mode(&self) -> Result<u32>;
    /// Actor `+20/+24` and `+2C/+30` (binary32 bits), `+14` GTID.
    fn actor_position(&self, actor: Self::Object) -> Result<[u32; 2]>;
    fn actor_facing(&self, actor: Self::Object) -> Result<[u32; 2]>;
    fn actor_gtid(&self, actor: Self::Object) -> Result<u32>;
    /// Organization (`+7C`) `+AC`, `+BC/+C0` and `+78[slot]`.
    fn organization_radius(&self, actor: Self::Object) -> Result<u32>;
    fn organization_position(&self, actor: Self::Object) -> Result<[u32; 2]>;
    fn slot_offset(&self, actor: Self::Object, slot: i32) -> Result<[u32; 2]>;
    /// `552EE0(position, radius)` + iterator `5581F0/558040/558580`: the
    /// objects in iteration order (the callbacks below do not modify it).
    fn query(&mut self, position: [u32; 2], radius_bits: u32) -> Result<Vec<Self::Object>>;
    /// Object virtual `+108`.
    fn owner(&mut self, object: Self::Object) -> Result<Option<Self::Owner>>;
    /// Owner virtual `+138(other)`.
    fn related(&mut self, owner: Self::Owner, other: Option<Self::Owner>) -> Result<bool>;
    /// Object `+4` definition and its `+3EC` recruit-structure byte.
    fn definition(&self, object: Self::Object) -> Result<D>;
    fn recruit_structure(&self, definition: &D) -> Result<bool>;
    ///Read params-definition+17C only after the candidate's actual3EC byte.
    ///Runtime adapters borrow the current source-list owner here; the legacy
    ///pure API uses its explicitly supplied list.
    fn accepts_structure(&self,request:&SpawnRequest<D>,definition:&D)->Result<bool>
    where D:PartialEq {Ok(request.structure_definitions.contains(definition))}
    fn object_position(&self, object: Self::Object) -> Result<[u32; 2]>;
    /// `4FACA0(params, structure)` with params `+40` = the actor GTID.
    fn create_at_structure(
        &mut self,
        request: &SpawnRequest<D>,
        params_40: u32,
        structure: Self::Object,
    ) -> Result<Option<Self::Object>>;
    /// `57D860(slot, member)` on the Organization.
    fn set_member(&mut self, actor: Self::Object, slot: i32, member: Self::Object) -> Result<()>;
    /// `578290(Organization+10, slot, &position, facing)`.
    fn slot_placement(
        &mut self,
        actor: Self::Object,
        slot: i32,
        position: [u32; 2],
        facing: [u32; 2],
    ) -> Result<bool>;
    ///578290/554AD0 may move the SAME caller XY. The facing argument is a
    ///separate native stack copy whose mutations do not reach57D950.
    fn slot_placement_at(&mut self,actor:Self::Object,slot:i32,position:&mut [u32;2],facing:[u32;2])->Result<bool>{
        self.slot_placement(actor,slot,*position,facing)
    }
    /// `57D950(slot, position (x, y, 0), facing, +50, +51)` on the Organization.
    fn create_member(
        &mut self,
        actor: Self::Object,
        slot: i32,
        position: [u32; 3],
        facing: [u32; 2],
        byte_50: u8,
        byte_51: u8,
    ) -> Result<Option<Self::Object>>;
}

fn binary(bits: u32) -> Result<aa_numeric::Binary> {
    aa_numeric::Binary::from_f32_bits(bits)
        .map_err(|e| Error::World(format!("non-finite x87 operand outside the Binary domain: {e}")))
}
fn store(value: aa_numeric::Result<aa_numeric::Binary>) -> Result<u32> {
    let value = value.map_err(|e| Error::World(format!("x87 arithmetic: {e}")))?;
    crate::organization_state::store32(&value).map_err(|e| Error::World(e.to_string()))
}

/// `0x4FBF40`.
pub fn spawn_member<D: PartialEq, W: SpawnWorld<D>>(
    request: &SpawnRequest<D>,
    actor: W::Object,
    slot: i32,
    world: &mut W,
    pc: aa_numeric::Precision,
) -> Result<Option<W::Object>> {
    spawn_member_with_queries::<D,W,CollectedQueries>(request,actor,slot,world,pc)
}

///Query transport for the SAME4FBF40 kernel. A runtime implementation lends
///the actual iterator between current-owner callbacks; it must retain its
///marks until native exhaustion/cleanup. No world/actor snapshot is implied.
pub trait SpawnQueries<D,W:SpawnWorld<D>>{
    type Cursor;
    fn begin(world:&mut W,position:[u32;2],radius:u32)->Result<Self::Cursor>;
    fn next(world:&mut W,cursor:&mut Self::Cursor)->Result<Option<W::Object>>;
    fn finish(world:&mut W,cursor:&mut Self::Cursor)->Result<()>;
}
///Legacy pure coordinator's declared pre-collected query seam. It owns only
///its returned vector, with no native index or resource-readiness authority.
struct CollectedQueries;
impl<D,W:SpawnWorld<D>> SpawnQueries<D,W> for CollectedQueries{
    type Cursor=std::vec::IntoIter<W::Object>;
    fn begin(world:&mut W,position:[u32;2],radius:u32)->Result<Self::Cursor>{Ok(world.query(position,radius)?.into_iter())}
    fn next(_:&mut W,cursor:&mut Self::Cursor)->Result<Option<W::Object>>{Ok(cursor.next())}
    fn finish(_:&mut W,_:&mut Self::Cursor)->Result<()>{Ok(())}
}
///4FBF40 with a real live iterator. The arithmetic, source/owner callback
///order and placement/create tail are shared with the original pure API.
pub fn spawn_member_with_queries<D:PartialEq,W:SpawnWorld<D>,Q:SpawnQueries<D,W>>(
    request:&SpawnRequest<D>,actor:W::Object,slot:i32,world:&mut W,pc:aa_numeric::Precision,
)->Result<Option<W::Object>>{
    if world.game_mode()? != 3 && request.word_28 != 0 {
        let position = world.actor_position(actor)?;
        let radius = world.organization_radius(actor)?;
        let mut cursor=Q::begin(world,position,radius)?;
        let queried=(||{
        let mut best: Option<(W::Object, u32)> = None;
        while let Some(object)=Q::next(world,&mut cursor)? {
            if world.owner(object)?.is_none() {
                continue;
            }
            let Some(owner) = world.owner(object)? else {
                return Err(Error::NullDereference("owner +108 changed between calls"));
            };
            let other = world.owner(actor)?;
            if !world.related(owner, other)? {
                continue;
            }
            let definition = world.definition(object)?;
            if !world.recruit_structure(&definition)?
                || !world.accepts_structure(request,&definition)?
            {
                continue;
            }
            let at = world.object_position(object)?;
            let dy = store(binary(position[1])?.sub(&binary(at[1])?, pc))?;
            let dx = store(binary(position[0])?.sub(&binary(at[0])?, pc))?;
            let xx = binary(dx)?
                .mul(&binary(dx)?, pc)
                .map_err(|e| Error::World(format!("x87 arithmetic: {e}")))?;
            let yy = binary(dy)?
                .mul(&binary(dy)?, pc)
                .map_err(|e| Error::World(format!("x87 arithmetic: {e}")))?;
            let distance = store(xx.add(&yy, pc))?;
            let closer = match best {
                None => true,
                Some((_, best_distance)) => {
                    crate::organization_state::compare32(distance, best_distance)
                        == Some(std::cmp::Ordering::Less)
                }
            };
            if closer {
                best = Some((object, distance));
            }
        }
        Ok(match best {
            Some((structure, _)) => {
                let gtid = world.actor_gtid(actor)?;
                world.create_at_structure(request, gtid, structure)?
            }
            None => None,
        })
        })();
        let cleanup=Q::finish(world,&mut cursor);
        let created=match(queried,cleanup){
            (Ok(value),Ok(()))=>value,(Err(error),Ok(()))|(Ok(_),Err(error))=>return Err(error),
            (Err(error),Err(cleanup))=>return Err(Error::World(format!("{error}; member query cleanup failed: {cleanup}"))),
        };
        if let Some(member) = created {
            world.set_member(actor, slot, member)?;
            return Ok(Some(member));
        }
    }
    let base = world.organization_position(actor)?;
    let offset = world.slot_offset(actor, slot)?;
    let y = store(binary(base[1])?.add(&binary(offset[1])?, pc))?;
    let x = store(binary(base[0])?.add(&binary(offset[0])?, pc))?;
    let facing = world.actor_facing(actor)?;
    let mut position=[x,y];
    if !world.slot_placement_at(actor, slot, &mut position, facing)? {
        return Ok(None);
    }
    world.create_member(actor, slot, [position[0], position[1], 0], facing, request.byte_50, request.byte_51)
}
