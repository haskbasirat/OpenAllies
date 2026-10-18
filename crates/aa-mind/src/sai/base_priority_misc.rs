//! Original SAI goal-class base priorities (goal virtual `+78`) of the
//! non-military classes, with the conventions of `base_priority`:
//!
//! | class | +78 | Rust |
//! |---|---|---|
//! | PACK | 7CB640 | [`pack`] |
//! | RECRUIT | 4B2830 (`ret`) | [`recruit`] |
//! | GIVE_RESOURCES | 7C10B0 | [`give_resources`] |
//! | GIVE_ACTOR | 7C1220 | [`give_actor`] |
//! | RESEARCH | 7D0680 | [`research`] |
//! | ATTACH | 7BE260 | [`attach`] |
//!
//! Callees kept as services: the zone/tile queries `4282A0`, `42A530`,
//! `54F8B0`, the SAI query `79E590`, the HQ attach test `574A10`, `4D01E0`,
//! and the ego valuations `4A6FA0`, `4ACE30`, `4A9DB0`. The ego hash
//! lookups `4A9A20` (ego `+82C..+834`) and `4A8C50` are ported here.
use super::base_priority::{BasePriorityWorld, ClassGoal, Obj, PlayerContext};
use super::ego::Ego;
use super::native_hash::NativeHash;
use crate::{
    Error, Result,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};

/// Services of the non-military `+78` bodies beyond [`BasePriorityWorld`].
pub trait ClassPriorityWorld: BasePriorityWorld {
    /// Field: the first word of the object a class word points to.
    fn first_word(&mut self, obj: Obj) -> Result<u32>;
    /// Field: component `+40`.
    fn component_40(&mut self, component: Obj) -> Result<Option<Obj>>;
    /// Field: object `+84`.
    fn word_84(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+98` (organization).
    fn word_98(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+128`.
    fn word_128(&mut self, obj: Obj) -> Result<u32>;
    /// Field: word `+548` of the object's definition `+4`.
    fn definition_548(&mut self, obj: Obj) -> Result<u32>;
    /// Field: byte `+348` of the object's definition `+4`.
    fn definition_348(&mut self, obj: Obj) -> Result<bool>;
    /// Field: the object's definition `+4` (opaque handle).
    fn definition(&mut self, obj: Obj) -> Result<Obj>;
    /// Field: GTID `+D0` of a component.
    fn component_d0(&mut self, component: Obj) -> Result<u32>;
    /// Field: object `+D8` component.
    fn component_d8(&mut self, obj: Obj) -> Result<Obj>;
    /// Field: object `+154` SAI actor record (`4D03C0`).
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Field: the hash table object at SAI actor record `+24` (`+48`
    /// buckets, `+4C` bucket count; `None` when `+24` is null).
    fn record_table(&mut self, record: Obj) -> Result<Option<NativeHash<u32>>>;
    /// Field: the player object's `+2DC/+2E0` HQ array (signed count).
    fn hq_slots(&mut self, player: Obj) -> Result<Vec<Option<Obj>>>;
    /// Field: `[B36534]+30` map extent `+C/+10` (binary32).
    fn map_extent(&mut self) -> Result<[u32; 2]>;
    /// `54F8B0(kingdom, 0)` on the tile `42A530(point)` of `[B36534]+30`.
    fn tile_zone(&mut self, point: [u32; 2], kingdom: Option<Obj>) -> Result<bool>;
    /// `79E590(kingdom)` on the SAI object.
    fn sai_query_79e590(&mut self, kingdom: Option<Obj>) -> Result<bool>;
    /// `574A10(actor, 0, 0)` on an HQ's `+D8` component.
    fn hq_accepts(&mut self, component: Obj, actor: Option<Obj>) -> Result<bool>;
    /// `4D01E0(flag)` on an object (ST0).
    fn value_4d01e0(&mut self, obj: Obj, flag: bool) -> Result<Binary>;
    /// `4A6FA0(kingdom, actor, amount)` on the ego (ST0).
    fn give_resources_value(
        &mut self,
        kingdom: Option<Obj>,
        actor: Option<Obj>,
        amount: u32,
    ) -> Result<Binary>;
    /// `4ACE30(kingdom, recipient, given actor, player)` on the ego (ST0).
    fn give_actor_value(
        &mut self,
        kingdom: Option<Obj>,
        recipient: Option<Obj>,
        given: Option<Obj>,
        player: Obj,
    ) -> Result<Binary>;
    /// `4A9DB0(definition, player, flag)` on the ego (ST0).
    fn definition_value(&mut self, definition: Obj, player: Obj, flag: bool) -> Result<Binary>;
}

fn null(what: &str) -> Error {
    Error::World(format!("{what} (native null dereference)"))
}

/// First node of `key`'s chain (head first), as the native finds walk it.
fn lookup(table: &NativeHash<u32>, key: u32) -> Result<Option<u32>> {
    let n = table.buckets.len() as u32;
    if n == 0 {
        return Err(Error::World(
            "native hash lookup divides by a zero bucket count".into(),
        ));
    }
    Ok(table.buckets[(key % n) as usize]
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v))
}

/// `4A9A20(key)` on the ego: the `+830` table value, else `+82C`.
pub fn ego_table_830(ego: &Ego, key: u32) -> Result<u32> {
    Ok(lookup(&ego.table_830, key)?.unwrap_or(ego.u32(0x82c)))
}

/// `4A8C50(key)`: the table value, else +0.0.
pub fn table_value(table: &NativeHash<u32>, key: u32) -> Result<u32> {
    Ok(lookup(table, key)?.unwrap_or(0))
}

/// `4282A0(point)` on `[B36534]+30`: `0 <= x < +C` and `0 <= y < +10`
/// (an unordered coordinate fails).
pub fn point_on_map(point: [u32; 2], extent: [u32; 2]) -> bool {
    let (x, y) = (f32::from_bits(point[0]), f32::from_bits(point[1]));
    x >= 0.0 && x < f32::from_bits(extent[0]) && y >= 0.0 && y < f32::from_bits(extent[1])
}

/// `4B2830` (RECRUIT): a bare `ret`; the base priority is left as is.
pub fn recruit(_goal: &mut ClassGoal) -> Result<()> {
    Ok(())
}

/// Complete `7CB640` (PACK, `+3C` SAI actor record): ego `+368` when the
/// packable object (the record's object, or its `+7C` component's `+40`)
/// stands on the map outside the `54F8B0` zone and `79E590` holds.
pub fn pack(
    w: &mut impl ClassPriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
) -> Result<()> {
    goal.priority = 0;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let record = goal.word(0x3c)?;
    if record == 0 {
        return Err(null("7A10F0 of a null record"));
    }
    let id = w.record_object(record)?;
    let object = w
        .resolve(id)?
        .ok_or_else(|| null("7CB665 +7C of the record's stale object"))?;
    let subject = match w.component_7c(object)? {
        Some(component) => w.component_40(component)?,
        None => Some(object),
    };
    let Some(subject) = subject else {
        return Ok(());
    };
    if w.flags(subject)? & 1 != 0
        || w.definition_548(subject)? == 0
        || w.word_84(subject)? == 0
        || !w.definition_348(subject)?
    {
        return Ok(());
    }
    let point = w.position(subject)?;
    if !point_on_map(point, w.map_extent()?) || w.tile_zone(point, ctx.actor)? {
        return Ok(());
    }
    if !w.sai_query_79e590(ctx.actor)? {
        return Ok(());
    }
    goal.priority = ctx.ego.u32(0x368);
    Ok(())
}

/// Complete `7C10B0` (GIVE_RESOURCES, `+3C` recipient GTID, `+40` amount).
pub fn give_resources(
    w: &mut impl ClassPriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
) -> Result<()> {
    let recipient = goal.word(0x3c)?;
    let Some(r) = w.resolve(recipient)? else {
        goal.priority = 0;
        return Ok(());
    };
    if !w.virtual_138(r, ctx.actor)? {
        goal.priority = 0;
        return Ok(());
    }
    let r = w.resolve(recipient)?;
    goal.priority = store(w.give_resources_value(ctx.actor, r, goal.word(0x40)?)?)?;
    Ok(())
}

/// Complete `7C1220` (GIVE_ACTOR, `+3C` recipient GTID, `+40` actor GTID).
pub fn give_actor(
    w: &mut impl ClassPriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
) -> Result<()> {
    let (recipient, actor) = (goal.word(0x3c)?, goal.word(0x40)?);
    if w.resolve(actor)?.is_none() {
        goal.priority = 0;
        return Ok(());
    }
    let Some(r) = w.resolve(recipient)? else {
        goal.priority = 0;
        return Ok(());
    };
    if !w.virtual_138(r, ctx.actor)? {
        goal.priority = 0;
        return Ok(());
    }
    let given = w.resolve(actor)?;
    let r = w.resolve(recipient)?;
    goal.priority = store(w.give_actor_value(ctx.actor, r, given, ctx.player)?)?;
    Ok(())
}

/// Complete `7D0680` (RESEARCH, `+3C` researcher GTID, `+40` pointer to the
/// research key): the ego `+830` value plus the researcher's progress table
/// (its organization, else virtual `+100`, then the SAI record `+24`).
pub fn research(
    w: &mut impl ClassPriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    let researcher = goal.word(0x3c)?;
    let Some(r) = w.resolve(researcher)? else {
        goal.priority = 0;
        return Ok(());
    };
    if w.flags(r)? & 1 != 0 {
        goal.priority = 0;
        return Ok(());
    }
    let key_ptr = goal.word(0x40)?;
    if key_ptr == 0 {
        return Err(null("7D06F4 research key"));
    }
    let key = w.first_word(key_ptr)?;
    goal.priority = ego_table_830(ctx.ego, key)?;
    let r = w
        .resolve(researcher)?
        .ok_or_else(|| null("7D0730 +98 of a stale researcher"))?;
    let holder = if w.word_98(r)? != 0 {
        w.resolve(researcher)?
    } else {
        w.virtual_100(r)?
    };
    let Some(holder) = holder else {
        return Ok(());
    };
    let Some(record) = w.sai_record(holder)? else {
        return Ok(());
    };
    if let Some(table) = w.record_table(record)? {
        let key = w.first_word(key_ptr)?;
        let progress = table_value(&table, key)?;
        goal.priority = store(num(n(progress)?.add(&n(goal.priority)?, p))?)?;
    }
    Ok(())
}

/// Complete `7BE260` (ATTACH, `+3C` actor GTID, `+40` chosen HQ GTID).
pub fn attach(
    w: &mut impl ClassPriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let actor_id = goal.word(0x3c)?;
    let actor = w
        .resolve(actor_id)?
        .ok_or_else(|| null("7BE2A3 +7C of a stale actor"))?;
    let organization = w
        .component_7c(actor)?
        .ok_or_else(|| null("7BE2A6 +D0 of a null +7C"))?;
    let attached = w.component_d0(organization)?;
    if w.resolve(attached)?.is_some() {
        return Ok(());
    }
    let ego = ctx.ego;
    if ego.image[0x7a4] == 0 {
        let a = w
            .resolve(actor_id)?
            .ok_or_else(|| null("7BE306 +100 of a stale actor"))?;
        if w.flags_100(a)? & 0x10 != 0 {
            return Ok(());
        }
        let a = w
            .resolve(actor_id)?
            .ok_or_else(|| null("4CFE90 of a stale actor"))?;
        if w.has_118(a)? {
            return Ok(());
        }
        let a = w
            .resolve(actor_id)?
            .ok_or_else(|| null("7BE336 +128 of a stale actor"))?;
        if w.word_128(a)? != 0 {
            return Ok(());
        }
    }
    goal.set_word(0x40, 0)?;
    let kingdom = ctx.actor.ok_or_else(|| null("7BE34D +2E0 of a null player"))?;
    for hq in w.hq_slots(kingdom)?.into_iter().flatten() {
        if w.flags(hq)? & 1 != 0 {
            continue;
        }
        let a = w.resolve(actor_id)?;
        let component = w.component_d8(hq)?;
        if w.hq_accepts(component, a)? {
            let id = w.object_gtid(hq)?;
            goal.set_word(0x40, id)?;
            break;
        }
    }
    if w.resolve(goal.word(0x40)?)?.is_none() {
        return Ok(());
    }
    let a = w
        .resolve(actor_id)?
        .ok_or_else(|| null("4D01E0 of a stale actor"))?;
    let value = w.value_4d01e0(a, false)?;
    goal.priority = store(num(value.mul(&n(ego.u32(0x7a8))?, p))?)?;
    let a = w
        .resolve(actor_id)?
        .ok_or_else(|| null("7BE45B +4 of a stale actor"))?;
    let definition = w.definition(a)?;
    let extra = w.definition_value(definition, ctx.player, false)?;
    goal.priority = store(num(extra.add(&n(goal.priority)?, p))?)?;
    Ok(())
}
