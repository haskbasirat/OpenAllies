//! Native ability target/admission sequence4C8F20, eligibility4C8740 and
//! availability4C8A40. Actor geometry and definition references remain typed
//! required inputs; rejection codes are the original values1..10.
use crate::{Error, Result, ability::Ability, performer::Context};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Filter {
    pub required: u32,
    pub forbidden: u32,
}
impl Filter {
    pub fn accepts(self, flags: u32) -> bool {
        flags & self.required == self.required && flags & self.forbidden == 0
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition {
    pub minimum_range: F32,
    pub recharge_time: F32,
    pub range: F32,
    pub targets: Filter,
    pub area: F32,
    pub projectile: bool,
    pub mana_cost: F32,
    pub volley_area: F32,
    pub flags: u32,
    pub effects: Vec<u32>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LocalDefinition {
    /// Actual4BF990 computed max(animation length+reload, definition recharge).
    pub recharge_time: F32,
    pub fuzzy_range: F32,
    pub turret_index: i32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub flags8: u32,
    pub flags100: u32,
    pub flags104: u32,
    pub position: [F32; 3],
    pub has_regiment: bool,
    pub has_organization: bool,
    pub has_properties: bool,
    pub has_summoner: bool,
    pub has_mind: bool,
    pub mana: Option<F32>,
    pub body: Option<[F32; 2]>,
    pub kingdom_kind: u32,
}
/// Native actor/object graphs and derived values are supplied at each read.
/// Definition/effect objects remain alive and immutable during the operation.
pub trait World {
    fn definition(&self, id: u32) -> Result<Definition>;
    fn local_definition(&self, id: u32) -> Result<LocalDefinition>;
    fn actor(&self, id: u32) -> Result<Actor>;
    fn is_actor(&mut self, id: u32) -> Result<bool>;
    fn is_kingdom(&mut self, id: u32) -> Result<bool>;
    fn allegiance(&mut self, id: u32) -> Result<Option<u32>>;
    fn friendly(&mut self, source: u32, target: Option<u32>) -> Result<bool>;
    fn allied(&mut self, source: u32, target: Option<u32>) -> Result<bool>;
    /// Actual4CF410 category/flag extraction, independent of relation flags.
    fn class_flags(&mut self, actor: u32) -> Result<u32>;
    fn injured(&mut self, actor: u32) -> Result<bool>;
    fn organization_leader(&mut self, actor: u32) -> Result<Option<u32>>;
    fn summoner_accepts(&mut self, source: u32, target: u32) -> Result<bool>;
    fn modifier(&mut self, actor: u32) -> Result<Option<u32>>;
    fn recharge_modified(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    fn projectile_range_modified(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    fn bombard_range_modified(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    fn effect_kind(&self, effect: u32) -> Result<u32>;
    fn effect_property(&mut self, effect: u32) -> Result<Option<u32>>;
    fn property_immunity_class(&self, property: u32) -> Result<u32>;
    fn immune_to(&mut self, target: u32, class: u32) -> Result<bool>;
    /// Native501920 merged definition/source effect iterator, in original order.
    /// ABILITY_EFFECT modifier effects come from the source's
    /// `ModifierState::ability_effects[ability type index]` (head first;
    /// maintained by `aa_body::modifier_tables`).
    fn effects_for(&mut self, definition: u32, source: Option<u32>) -> Result<Vec<u32>>;
    fn has_property(&mut self, target: u32, property: Option<u32>) -> Result<bool>;
    fn mind_allows_full_health_heal(&mut self, source: u32) -> Result<bool>;
    /// Exact original shape/shape, shape/point or point/point predicate. The
    /// inverse flag selects the original minimum-range branch, not a negation
    /// invented around a replacement center-distance test.
    fn range_test(&mut self, context: &Context, range: F32, inverse: bool) -> Result<bool>;
    fn turret_covers(
        &mut self,
        local_definition: u32,
        source: u32,
        target: [F32; 2],
    ) -> Result<bool>;
    fn can_pay(&mut self, allegiance: u32, ability: &Ability) -> Result<bool>;
    fn resolve_id(&mut self, id: u32) -> Result<Option<u32>>;
    fn owned_actors(&mut self, kingdom: u32) -> Result<Vec<u32>>;
    fn summon_filter(&self, effect: u32) -> Result<Filter>;
    fn free_recharge_enabled(&self) -> Result<bool>;
    fn now(&self) -> Result<F32>;
}
/// The exact subset used by4C8270/57EC80. A range-only runtime does not need to
/// pretend that payment, summons or other admission services are implemented.
#[derive(Clone, Copy, Debug)]
pub struct RangeDefinition {
    pub range: F32,
    pub projectile: bool,
    pub flags: u32,
}
impl From<&Definition> for RangeDefinition {
    fn from(value: &Definition) -> Self {
        Self {
            range: value.range,
            projectile: value.projectile,
            flags: value.flags,
        }
    }
}
pub trait RangeWorld {
    fn range_definition(&self, id: u32) -> Result<RangeDefinition>;
    fn range_local_fuzzy(&self, id: u32) -> Result<F32>;
    fn range_modifier(&mut self, actor: u32) -> Result<Option<u32>>;
    fn modify_projectile_range(&mut self, modifier: u32, base: F32) -> Result<Binary>;
    fn modify_bombard_range(&mut self, modifier: u32, base: F32) -> Result<Binary>;
}
impl<T: World> RangeWorld for T {
    fn range_definition(&self, id: u32) -> Result<RangeDefinition> {
        Ok(RangeDefinition::from(&self.definition(id)?))
    }
    fn range_local_fuzzy(&self, id: u32) -> Result<F32> {
        Ok(self.local_definition(id)?.fuzzy_range)
    }
    fn range_modifier(&mut self, actor: u32) -> Result<Option<u32>> {
        self.modifier(actor)
    }
    fn modify_projectile_range(&mut self, modifier: u32, base: F32) -> Result<Binary> {
        self.projectile_range_modified(modifier, base)
    }
    fn modify_bombard_range(&mut self, modifier: u32, base: F32) -> Result<Binary> {
        self.bombard_range_modified(modifier, base)
    }
}
fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn positive(v: F32) -> Result<bool> {
    Ok(n(v)?.compare(&Binary::from_i32(0)) == Ordering::Greater)
}
fn nonzero(v: F32) -> Result<bool> {
    Ok(!n(v)?.is_zero())
}

/// Native4C5840's relation and actor attributes, used by4C5A20 filters.
pub fn target_flags(
    filter: Filter,
    source: Option<u32>,
    mut allegiance: Option<u32>,
    target: u32,
    world: &mut impl World,
) -> Result<u32> {
    let mut flags = world.class_flags(target)? | 0x2000;
    let target_owner = if world.is_kingdom(target)? {
        Some(target)
    } else {
        world.allegiance(target)?
    };
    if allegiance.is_none()
        && let Some(source) = source
    {
        allegiance = if world.is_kingdom(source)? {
            Some(source)
        } else {
            world.allegiance(source)?
        };
    }
    if let Some(owner) = allegiance {
        flags |= if world.friendly(owner, target_owner)? {
            1
        } else if world.allied(owner, target_owner)? {
            2
        } else {
            4
        };
        if Some(owner) == target_owner {
            flags |= 0x10;
        }
    }
    if world.actor(target)?.has_regiment {
        flags |= 0x200;
    }
    if world.actor(target)?.flags100 & 0x200000 != 0 {
        flags |= 0x400;
    }
    if world.actor(target)?.body.is_some() && world.injured(target)? {
        flags |= 0x800;
    }
    if let Some(source) = source {
        if source == target {
            flags |= 0x20;
        }
        let source_view = world.actor(source)?;
        let leader = if source_view.has_organization {
            world.organization_leader(source)?
        } else {
            source_view.has_regiment.then_some(source)
        };
        if let Some(leader) = leader {
            let target_view = world.actor(target)?;
            let target_leader = if target_view.has_organization {
                world.organization_leader(target)?
            } else {
                target_view.has_regiment.then_some(target)
            };
            if Some(leader) == target_leader {
                flags |= 0x40;
            }
        }
        if (filter.required | filter.forbidden) & 0x100 != 0
            && (!world.actor(source)?.has_summoner || !world.summoner_accepts(source, target)?)
        {
            flags |= 0x100;
        }
    }
    if target_owner.is_none()
        || world
            .actor(target_owner.expect("checked owner"))?
            .kingdom_kind
            == 1
    {
        flags |= 8;
    }
    if world.actor(target)?.body.is_some() {
        flags |= 0x1000;
    }
    if world.actor(target)?.flags8 & 0x800 != 0 {
        flags |= 0x4000;
    }
    Ok(flags)
}
pub fn target_matches(
    filter: Filter,
    context: &Context,
    target: u32,
    world: &mut impl World,
) -> Result<bool> {
    if filter.required == 0 && filter.forbidden == 0 {
        return Ok(true);
    }
    Ok(filter.accepts(target_flags(
        filter,
        context.source,
        context.allegiance,
        target,
        world,
    )?))
}
fn accepts_location(def: &Definition) -> Result<bool> {
    Ok(positive(def.area)? || positive(def.volley_area)? || def.flags & 0x20 != 0)
}
/// Native4C8740, including two actor-eligibility calls and targetability flag14.
pub fn eligible(
    def: &Definition,
    context: &Context,
    targetable: bool,
    world: &mut impl World,
) -> Result<bool> {
    let Some(target) = context.target else {
        return accepts_location(def);
    };
    if !world.is_actor(target)?
        || !world.is_actor(target)?
        || (targetable && world.actor(target)?.flags8 & 0x4000 == 0)
    {
        return Ok(false);
    }
    if world.actor(target)?.flags104 & 0x80 != 0 {
        let mut found = false;
        for &effect in &def.effects {
            if effect != 0 && world.effect_kind(effect)? == 3 {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    if def.flags & 4 != 0 && !world.actor(target)?.has_properties {
        return Ok(false);
    }
    target_matches(def.targets, context, target, world)
}
fn recharge_value(
    ability: &Ability,
    def: &Definition,
    modifier: Option<u32>,
    world: &mut impl World,
) -> Result<Binary> {
    match ability.local_definition {
        Some(local) => {
            let time = world.local_definition(local)?.recharge_time;
            match modifier {
                Some(m) => world.recharge_modified(m, time),
                None => n(time),
            }
        }
        None => n(def.recharge_time),
    }
}
///4C83E0 retains its extended subtraction until the caller's comparison.
pub fn recharge_remaining(
    ability: &Ability,
    source: Option<u32>,
    world: &mut impl World,
    precision: Precision,
) -> Result<Binary> {
    if ability.recharge_bypass && world.free_recharge_enabled()? {
        return Ok(Binary::from_i32(0));
    }
    let modifier = match source {
        Some(s) => world.modifier(s)?,
        None => None,
    };
    let def = world.definition(ability.definition)?;
    let time = recharge_value(ability, &def, modifier, world)?;
    let remaining = time.sub(
        &n(world.now()?)?.sub(&n(ability.last_use)?, precision)?,
        precision,
    )?;
    Ok(
        if remaining.compare(&Binary::from_i32(0)) == Ordering::Greater {
            remaining
        } else {
            Binary::from_i32(0)
        },
    )
}
/// Native4BE910: base definition range, then the projectile and bombard
/// modifier getters in order (the projectile result is stored to binary32).
fn range_value(
    def: &RangeDefinition,
    modifier: Option<u32>,
    world: &mut impl RangeWorld,
) -> Result<Binary> {
    let mut result = n(def.range)?;
    if let Some(modifier) = modifier {
        let mut base = def.range;
        if def.projectile {
            result = world.modify_projectile_range(modifier, base)?;
            base = F32(result.store_f32()?);
        }
        if def.flags & 8 != 0 {
            result = world.modify_bombard_range(modifier, base)?;
        }
    }
    Ok(result)
}
/// Native4C8270: modified shared range plus the local/instance fuzzy product.
/// The product and final addition remain extended; callers choose the store.
pub fn effective_range(
    ability: &Ability,
    modifier: Option<u32>,
    world: &mut impl RangeWorld,
    precision: Precision,
) -> Result<Binary> {
    let definition = world.range_definition(ability.definition)?;
    let range = range_value(&definition, modifier, world)?;
    if let Some(local) = ability.local_definition {
        n(world.range_local_fuzzy(local)?)?
            .mul(&n(ability.fuzzy_range)?, precision)?
            .add(&range, precision)
            .map_err(Into::into)
    } else {
        Ok(range)
    }
}
/// Native57EC80 over a stable owned Performer ability table. Only enabled
/// entries participate. The winning range is evaluated twice, then stored to
/// binary32; comparison uses the first extended result against the stored max.
pub fn maximum_range(
    abilities: &[Ability],
    owner: u32,
    world: &mut impl RangeWorld,
    precision: Precision,
) -> Result<Binary> {
    let mut maximum = 0;
    for ability in abilities.iter().filter(|a| a.enabled) {
        let modifier = world.range_modifier(owner)?;
        let range = effective_range(ability, modifier, world, precision)?;
        if range.compare(&Binary::from_f32_bits(maximum)?) == Ordering::Greater {
            maximum = effective_range(ability, modifier, world, precision)?.store_f32()?;
        }
    }
    Ok(Binary::from_f32_bits(maximum)?)
}
/// Native4C89F0 cleanup and4C8A40 summon/mana/recharge checks.
pub fn available(
    ability: &mut Ability,
    source: Option<u32>,
    world: &mut impl World,
    precision: Precision,
) -> Result<u32> {
    let mut index = 0;
    while index < ability.summoned_ids.len() {
        if world.resolve_id(ability.summoned_ids[index])?.is_none() {
            ability.summoned_ids.remove(index);
        } else {
            index += 1;
        }
    }
    let def = world.definition(ability.definition)?;
    if def.flags & 0x100 == 0 && !ability.summoned_ids.is_empty() {
        return Ok(9);
    }
    if def.flags & 0x200 != 0 {
        let mut summon = None;
        for &effect in &def.effects {
            if world.effect_kind(effect)? == 0 {
                summon = Some(effect);
                break;
            }
        }
        if let Some(effect) = summon {
            let source = source.ok_or(Error::Invalid("unique summon requires source"))?;
            let owner = if world.is_kingdom(source)? {
                Some(source)
            } else {
                world.allegiance(source)?
            }
            .ok_or(Error::Invalid("unique summon requires kingdom"))?;
            for actor in world.owned_actors(owner)? {
                if world.actor(actor)?.flags8 & 1 == 0
                    && world
                        .summon_filter(effect)?
                        .accepts(world.class_flags(actor)?)
                {
                    return Ok(10);
                }
            }
        }
    }
    if let Some(source) = source
        && let Some(mana) = world.actor(source)?.mana
        && n(mana)?.compare(&n(def.mana_cost)?) == Ordering::Less
    {
        return Ok(7);
    }
    if recharge_remaining(ability, source, world, precision)?.compare(&Binary::from_i32(0))
        == Ordering::Greater
    {
        return Ok(8);
    }
    Ok(0)
}

/// Native4C8F20 mutates the supplied context before some rejection branches.
/// Definition storage stays stable; previous context/list writes are not rolled back.
pub fn can_use(
    ability: &mut Ability,
    context: &mut Context,
    check_range: bool,
    world: &mut impl World,
    precision: Precision,
) -> Result<u32> {
    let def = world.definition(ability.definition)?;
    if let Some(source) = context.source
        && Some(source) != context.allegiance
    {
        context.allegiance = world.allegiance(source)?;
    }
    if let Some(target) = context.target
        && world.actor(target)?.flags8 & 1 != 0
        && accepts_location(&def)?
    {
        let position = world.actor(target)?.position;
        context.target_position = [position[0], position[1], F32::ZERO];
        context.target = None;
    }
    if let Some(target) = context.target
        && world.actor(target)?.flags8 & 1 != 0
    {
        return Ok(1);
    }
    if context.phase == 0
        && let Some(source) = context.source
    {
        let modifier = world.modifier(source)?;
        if recharge_value(ability, &def, modifier, world)?.compare(&Binary::from_i32(0))
            != Ordering::Greater
            && let Some(mana) = world.actor(source)?.mana
            && n(mana)?.compare(&n(def.mana_cost)?) == Ordering::Less
        {
            return Ok(1);
        }
    }
    if context.phase != 3 && context.phase != 1 && !eligible(&def, context, true, world)? {
        return Ok(1);
    }
    let target_actor = match context.target {
        Some(t) if world.is_actor(t)? => Some(t),
        _ => None,
    };
    if def.flags & 0x20000 != 0
        && let Some(target) = target_actor
        && world.actor(target)?.has_properties
        && !world.friendly(target, context.allegiance)?
    {
        let mut permitted = false;
        for &effect in &def.effects {
            let property = world.effect_property(effect)?;
            if let Some(property) = property {
                let class = world.property_immunity_class(property)?;
                if !world.immune_to(target, class)? {
                    permitted = true;
                    break;
                }
            } else {
                permitted = true;
                break;
            }
        }
        if !permitted {
            return Ok(1);
        }
    }
    if context.phase == 0 {
        if check_range
            && nonzero(def.minimum_range)?
            && !world.range_test(context, def.minimum_range, true)?
        {
            return Ok(2);
        }
        if let Some(source) = context.source
            && let Some(local) = ability.local_definition
            && world.local_definition(local)?.turret_index != -1
        {
            let point = match context.target {
                Some(t) => {
                    let p = world.actor(t)?.position;
                    [p[0], p[1]]
                }
                None => [context.target_position[0], context.target_position[1]],
            };
            if !world.turret_covers(local, source, point)? {
                return Ok(3);
            }
        }
        if let Some(owner) = context.allegiance
            && !world.can_pay(owner, ability)?
        {
            return Ok(4);
        }
        if check_range {
            let modifier = match context.source {
                Some(s) => world.modifier(s)?,
                None => None,
            };
            let mut range =
                F32(range_value(&RangeDefinition::from(&def), modifier, world)?.store_f32()?);
            if let Some(local) = ability.local_definition {
                range = F32(n(world.local_definition(local)?.fuzzy_range)?
                    .mul(&n(ability.fuzzy_range)?, precision)?
                    .add(&n(range)?, precision)?
                    .store_f32()?);
            }
            if !world.range_test(context, range, false)? {
                return Ok(5);
            }
        }
    }
    if let Some(target) = target_actor {
        if def.flags & 4 != 0 && def.flags & 0x20000 != 0 {
            for effect in world.effects_for(ability.definition, context.source)? {
                if world.effect_kind(effect)? == 6 {
                    if let Some(t) = context.target {
                        let _ = world.is_actor(t)?;
                    }
                    let property = world.effect_property(effect)?;
                    if world.has_property(target, property)? {
                        return Ok(6);
                    }
                }
            }
        }
        if def.flags & 0x40 != 0 {
            let heal_override = match context.source {
                Some(s) if world.actor(s)?.has_mind => world.mind_allows_full_health_heal(s)?,
                _ => false,
            };
            if !heal_override
                && let Some([health, maximum]) = world.actor(target)?.body
                && n(health)?.compare(&n(maximum)?) != Ordering::Less
            {
                return Ok(6);
            }
        }
    }
    if context.phase == 0 {
        available(ability, context.source, world, precision)
    } else {
        Ok(0)
    }
}
