//!58B0E0/58AE10(0) on the same held Supply allocation. Scheduling, Zone0 and
//!SVar words remain in their actual owners. Relay collection and reached state
//!effects without a concrete binding fail after their native completed prefix.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
    supply_state::SupplyState,
};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required<T>(s: &'static str) -> Result<T> {
    Err(Error::Unsupported(s))
}

///4B3C10 updates the already registered Bool in native two-file order. The
///AA row is untyped; its private plain-string view is used solely to validate
///the literal/source span, never to invent a new String or Bool declaration.
pub(crate) fn under_siege_source(
    catalog: &Catalog<'_>,
    pc: Precision,
) -> std::result::Result<u8, String> {
    use aa_rules::native::TypedValue;
    let name = "SupplyProvideUnderSiege";
    let rows = catalog
        .settings
        .as_ref()
        .ok_or("Supply source Settings absent")?
        .declarations
        .get(&name.to_ascii_lowercase())
        .ok_or("Supply source Bool assignments absent")?;
    if rows.len() != 2 {
        return Err("Supply source Bool requires exact two-file assignment ownership".into());
    }
    let mut private = Vec::new();
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?
    {
        let value = match node.field(name, pc) {
            Ok(Some(TypedValue::Boolean { value, source, .. })) => Some((value, source, true)),
            Ok(None) => None,
            Ok(Some(_)) => {
                return Err("Supply source registered Bool has different private type".into());
            }
            Err(_) => match node.plain_string_field(name).map_err(|e| e.to_string())? {
                Some(TypedValue::String { value, source, .. }) => Some((
                    match value.to_ascii_lowercase().as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err("Supply untyped source is not a literal Bool".into()),
                    },
                    source,
                    false,
                )),
                None => None,
                _ => return Err("Supply private untyped assignment differs".into()),
            },
        };
        if let Some(value) = value {
            private.push(value);
        }
    }
    if private.len() != 2 {
        return Err("Supply private Bool source assignment universe differs".into());
    }
    let mut selected = None;
    for (index, path) in ["Game/SVars.tgi", "Game/SVars_AA.tgi"]
        .into_iter()
        .enumerate()
    {
        let public: Vec<_> = rows
            .iter()
            .filter(|r| r.source.path.eq_ignore_ascii_case(path))
            .collect();
        let [row] = public.as_slice() else {
            return Err("Supply source Bool file assignment is ambiguous".into());
        };
        let typed = row
            .field_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("flag") || t.eq_ignore_ascii_case("bool"));
        if (index == 0 && !typed) || (index == 1 && row.field_type.is_some()) {
            return Err(
                "Supply source requires original typed Bool then exact untyped AA assignment"
                    .into(),
            );
        }
        let matches: Vec<_> = private
            .iter()
            .filter(|(_, source, is_typed)| *source == row.source && *is_typed == typed)
            .collect();
        let [(value, _, _)] = matches.as_slice() else {
            return Err("Supply public Bool value/span differs from private source".into());
        };
        let raw = match row.raw.to_ascii_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => return Err("Supply public source is not a literal Bool".into()),
        };
        if raw != *value {
            return Err("Supply public Bool literal differs from private source".into());
        }
        selected = Some(u8::from(*value));
    }
    selected.ok_or_else(|| "Supply native source Bool assignment missing".into())
}

pub(crate) fn recharge_source(
    catalog: &Catalog<'_>,
    pc: Precision,
) -> std::result::Result<u32, String> {
    let name = "SupplyStockpileRechargeRate";
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("Supply current SVar Settings owner absent")?;
    let (bits, provenance) = settings.unique_fixed(name, pc).map_err(|e| e.to_string())?;
    let mut matches = 0;
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?
    {
        if let Some(aa_rules::native::TypedValue::Scalar(value)) =
            node.field(name, pc).map_err(|e| e.to_string())?
            && value.field_type == aa_rules::native::ScalarType::Fixed
            && value.source == *provenance
            && value.bits == bits
        {
            matches += 1;
        }
    }
    if matches != 1 {
        return Err("Supply recharge SVar public value/span differs from private source".into());
    }
    Ok(bits)
}

fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    let pc = crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|m| m & (1 << ComponentSlot::Supply.index()) == 0)
    {
        return required("58B0E0 actual Supply outside canonical source factory");
    }
    supply(record)?;
    Ok(pc)
}
fn supply<'a>(record: &'a Record<'_>) -> Result<&'a SupplyState> {
    match record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(ComponentSlot::Supply))
    {
        Some(RestoredComponent::Supply(s)) if s.header.owner == record.owner => Ok(s),
        _ => required("58B0E0 actual Supply allocation/owner differs"),
    }
}
fn supply_mut<'a>(record: &'a mut Record<'_>) -> Result<&'a mut SupplyState> {
    match record
        .shell
        .actor
        .as_mut()
        .and_then(|a| a.components.get_mut(ComponentSlot::Supply))
    {
        Some(RestoredComponent::Supply(s)) if s.header.owner == record.owner => Ok(s),
        _ => required("58AE10 actual Supply allocation/owner differs"),
    }
}
fn source(
    catalog: &Catalog<'_>,
    record: &Record<'_>,
) -> Result<aa_rules::construction::SupplyFields> {
    let pc = checked(catalog, record)?;
    catalog
        .actors
        .native_supply_fields(&record.definition.proof.id, pc)
        .map_err(error)?
        .ok_or(Error::Unsupported(
            "58AE10 actual completed SupplyData absent",
        ))
}
fn low(record: &Record<'_>) -> Result<u32> {
    Ok(record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Supply actual Actor"))?
        .components
        .state
        .low)
}
fn zone<'a>(record: &'a Record<'_>) -> Result<Option<&'a aa_zones::zone::Zone>> {
    Ok(record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Supply actual Actor"))?
        .zones_148
        .zones()
        .slots
        .first()
        .and_then(Option::as_ref))
}

pub struct Context<'a, E> {
    pub zones: &'a mut aa_zones::tile::ZoneMap,
    pub identities: &'a mut crate::load_detection_activation::Identities,
    pub effects: &'a mut E,
    pub variables: &'a crate::load_view_variables::State,
    pub settings_mode_owner: &'a Option<u32>,
    pub global_settings: crate::live_placement::SettingsMode,
}

pub(crate) trait InitWorld {
    fn schedule(&mut self) -> Result<()>;
    fn zone_present(&mut self) -> Result<bool>;
    fn create_zone(&mut self) -> Result<()>;
    fn tick(&mut self) -> Result<()>;
}
pub(crate) fn initialize(world: &mut impl InitWorld) -> Result<bool> {
    world.schedule()?;
    if !world.zone_present()? {
        world.create_zone()?;
    }
    world.tick()?;
    Ok(true)
}

pub(crate) trait TickWorld {
    fn relay(&mut self) -> Result<bool>;
    fn suspended(&mut self) -> Result<bool>;
    fn clear_relay(&mut self) -> Result<()>;
    fn search_relays(&mut self) -> Result<()>;
    fn update_supply_state(&mut self) -> Result<()>;
    fn out_of_supply(&mut self) -> Result<bool>;
    fn stockpile(&mut self) -> Result<u32>;
    fn maximum(&mut self) -> Result<u32>;
    fn recharge(&mut self) -> Result<u32>;
    fn store_stockpile(&mut self, bits: u32) -> Result<()>;
    fn refresh_economy(&mut self) -> Result<()>;
    fn provides(&mut self) -> Result<bool>;
    fn zone_active(&mut self) -> Result<bool>;
    fn clear_zone(&mut self) -> Result<()>;
    fn dirty(&mut self) -> Result<bool>;
    fn radius(&mut self) -> Result<u32>;
    fn position(&mut self) -> Result<[u32; 2]>;
    fn move_zone(&mut self, position: [u32; 2], radius: u32) -> Result<()>;
    fn clear_dirty(&mut self) -> Result<()>;
}
fn binary(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(error)
}
pub(crate) trait ProvideWorld {
    fn owner(&mut self) -> Result<u32>;
    fn nonmajor(&mut self, owner: u32) -> Result<u32>;
    fn high(&mut self) -> Result<u32>;
    fn low(&mut self) -> Result<u32>;
    fn destroyed(&mut self) -> Result<bool>;
    fn under_siege_enabled(&mut self) -> Result<bool>;
    fn under_siege(&mut self) -> Result<bool>;
    fn is_relay(&mut self) -> Result<bool>;
    fn stockpile_supplied(&mut self) -> Result<bool>;
    fn range(&mut self) -> Result<u32>;
    fn city(&mut self) -> Result<bool>;
}
///58ACE0. All getters remain lazy and the actual owner is deliberately read twice.
pub(crate) fn provides(world: &mut impl ProvideWorld) -> Result<bool> {
    if world.owner()? == 0 {
        return Ok(false);
    }
    let owner = world.owner()?;
    if world.nonmajor(owner)? == 1 {
        return Ok(false);
    }
    //58AD11..27 reads both state words before branching on the high bit.
    let high = world.high()?;
    let low = world.low()?;
    if high & 0x80 != 0 || low & 0x200000 != 0 || world.destroyed()? {
        return Ok(false);
    }
    if !world.under_siege_enabled()? && world.under_siege()? {
        return Ok(false);
    }
    if world.is_relay()? && !world.stockpile_supplied()? {
        return Ok(false);
    }
    //58AD79..84 FCOMP/FNSTSW; TEST AH,5; JNP rejects ordered less.
    //Unordered gives AH&5=5 (even parity), so it continues to the City read.
    if f32::from_bits(world.range()?) < 0.0 {
        return Ok(false);
    }
    Ok(!world.city()?)
}
pub(crate) trait PrimaryWorld {
    fn out(&mut self) -> Result<bool>;
    fn supplied_state(&mut self) -> Result<()>;
    fn refresh_actions(&mut self) -> Result<()>;
}
///4D7CE0 after4D0A80's real primary-Supply immediate true return.
pub(crate) fn primary_update(world: &mut impl PrimaryWorld) -> Result<bool> {
    if !world.out()? {
        return Ok(false);
    }
    world.supplied_state()?;
    world.refresh_actions()?;
    Ok(true)
}
///58AE10, including the intermediate binary32 stockpile stores and reads.
///The immutable maximum remains x87-live across recharge's fallible services.
pub(crate) fn tick(world: &mut impl TickWorld, seconds: u32, pc: Precision) -> Result<()> {
    if world.relay()? && !world.suspended()? {
        world.clear_relay()?;
        world.search_relays()?;
    }
    world.update_supply_state()?;
    if world.relay()? {
        if world.out_of_supply()? {
            if f32::from_bits(world.stockpile()?) > 0.0 {
                let v = binary(world.stockpile()?)?
                    .sub(&binary(seconds)?, pc)
                    .map_err(error)?
                    .store_f32()
                    .map_err(error)?;
                world.store_stockpile(v)?;
                if f32::from_bits(world.stockpile()?) < 0.0 {
                    world.store_stockpile(0)?;
                }
                if f32::from_bits(world.stockpile()?) <= 0.0 {
                    world.refresh_economy()?;
                }
            }
        } else {
            let maximum = world.maximum()?;
            if f32::from_bits(maximum) > f32::from_bits(world.stockpile()?) {
                let v = binary(world.recharge()?)?
                    .mul(&binary(seconds)?, pc)
                    .map_err(error)?
                    .add(&binary(world.stockpile()?)?, pc)
                    .map_err(error)?
                    .store_f32()
                    .map_err(error)?;
                world.store_stockpile(v)?;
                if f32::from_bits(maximum) < f32::from_bits(world.stockpile()?) {
                    //Native FSTP quiets any supported x87 payload; nonfinite
                    //arithmetic is rejected by Binary instead of invented.
                    world.store_stockpile(binary(maximum)?.store_f32().map_err(error)?)?;
                }
            }
        }
    }
    let provides = world.provides()?;
    let active = world.zone_active()?;
    if provides != active {
        if active {
            world.clear_zone()?;
        } else {
            let radius = world.radius()?;
            let position = world.position()?;
            world.move_zone(position, radius)?;
            world.clear_dirty()?;
            return Ok(());
        }
    }
    if !active || !world.dirty()? {
        return Ok(());
    }
    let radius = world.radius()?;
    let position = world.position()?;
    world.move_zone(position, radius)?;
    world.clear_dirty()
}

struct Live<'a, 'record, 'assets, 'context, E> {
    catalog: &'a Catalog<'assets>,
    record: &'a mut Record<'record>,
    context: &'a mut Context<'context, E>,
    rng: &'a mut NativeRng,
    pc: Precision,
}
impl<E: crate::load_detection_activation::Effects> InitWorld for Live<'_, '_, '_, '_, E> {
    fn schedule(&mut self) -> Result<()> {
        crate::load_detection_init_world::schedule_existing(
            self.catalog,
            self.record,
            ComponentSlot::Supply,
            self.rng,
            self.pc,
        )
    }
    fn zone_present(&mut self) -> Result<bool> {
        Ok(zone(self.record)?.is_some())
    }
    fn create_zone(&mut self) -> Result<()> {
        crate::load_detection_activation::create_slot(self.record, self.context.identities, 0)
            .map(|_| ())
            .map_err(error)
    }
    fn tick(&mut self) -> Result<()> {
        tick(self, 0, self.pc)
    }
}
impl<E: crate::load_detection_activation::Effects> TickWorld for Live<'_, '_, '_, '_, E> {
    fn relay(&mut self) -> Result<bool> {
        Ok(source(self.catalog, self.record)?.is_relay)
    }
    fn suspended(&mut self) -> Result<bool> {
        let mode = crate::live_placement::read_settings_mode(
            true,
            None,
            self.context.global_settings,
            Some(self.context.settings_mode_owner),
        )
        .map_err(error)?
        .ok_or(Error::NullDereference("4F0460 actual World+28 MapResource"))?;
        Ok(mode != 0)
    }
    fn clear_relay(&mut self) -> Result<()> {
        supply_mut(self.record)?.word_1c = 0;
        Ok(())
    }
    fn search_relays(&mut self) -> Result<()> {
        required("58AE64 actual owner/shape/ZoneMap relay-list collection")
    }
    fn update_supply_state(&mut self) -> Result<()> {
        //4D0A80 returns immediately for an actual non-relay Supply allocation.
        //Other branches need actual Element/Kingdom/territory ownership.
        if source(self.catalog, self.record)?.is_relay {
            return required("4D0A80 actual relay/Element/Kingdom/territory services");
        }
        primary_update(self).map(|_| ())
    }
    fn out_of_supply(&mut self) -> Result<bool> {
        Ok(low(self.record)? & 0x80000 != 0)
    }
    fn stockpile(&mut self) -> Result<u32> {
        Ok(supply(self.record)?.stockpile.0)
    }
    fn maximum(&mut self) -> Result<u32> {
        source(self.catalog, self.record)?
            .stockpile_max_bits
            .ok_or(Error::Unsupported(
                "58AFD5 actual relay stockpile maximum absent",
            ))
    }
    fn recharge(&mut self) -> Result<u32> {
        self.context
            .variables
            .validate_catalog(self.catalog)
            .map_err(error)?;
        self.context.variables.require_published().map_err(error)?;
        Ok(self.context.variables.supply_recharge_rate_bits())
    }
    fn store_stockpile(&mut self, bits: u32) -> Result<()> {
        supply_mut(self.record)?.stockpile = aa_body::F32(bits);
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        required("58AFC7 actual Supply stockpile Economy refresh")
    }
    fn provides(&mut self) -> Result<bool> {
        provides(self)
    }
    fn zone_active(&mut self) -> Result<bool> {
        Ok(zone(self.record)?.is_some_and(|z| z.radius_bits != 0xbf800000))
    }
    fn clear_zone(&mut self) -> Result<()> {
        crate::load_detection_activation::operate_slot(
            self.record,
            self.context.zones,
            self.context.identities,
            0,
            None,
            self.pc,
            self.context.effects,
        )
        .map_err(error)
    }
    fn dirty(&mut self) -> Result<bool> {
        Ok(supply(self.record)?.byte_14 != 0)
    }
    fn radius(&mut self) -> Result<u32> {
        Ok(supply(self.record)?.supply_range.0)
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        Ok([
            self.record.shell.thing.position_bits[0],
            self.record.shell.thing.position_bits[1],
        ])
    }
    fn move_zone(&mut self, position: [u32; 2], radius: u32) -> Result<()> {
        crate::load_detection_activation::operate_slot(
            self.record,
            self.context.zones,
            self.context.identities,
            0,
            Some((position, radius)),
            self.pc,
            self.context.effects,
        )
        .map_err(error)
    }
    fn clear_dirty(&mut self) -> Result<()> {
        supply_mut(self.record)?.byte_14 = 0;
        Ok(())
    }
}

impl<E: crate::load_detection_activation::Effects> PrimaryWorld for Live<'_, '_, '_, '_, E> {
    fn out(&mut self) -> Result<bool> {
        Ok(low(self.record)? & 0x80000 != 0)
    }
    fn supplied_state(&mut self) -> Result<()> {
        let actor = self.record.shell.actor.as_mut().unwrap();
        crate::saved_actor_state::set_fields(
            self.record.owner,
            &mut actor.components.state,
            &mut self.record.shell.thing.flags,
            0x13,
            false,
        )
        .map_err(error)
    }
    fn refresh_actions(&mut self) -> Result<()> {
        required("4D7CE0 actual action refresh following supplied-state change")
    }
}

impl<E: crate::load_detection_activation::Effects> ProvideWorld for Live<'_, '_, '_, '_, E> {
    fn owner(&mut self) -> Result<u32> {
        self.record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&0xe8)
            .copied()
            .ok_or(Error::Unsupported(
                "58ACE8 actual current Actor+E8 uninitialized",
            ))
    }
    fn nonmajor(&mut self, owner: u32) -> Result<u32> {
        let owner = aa_actors::ActorId::from_raw(owner);
        let value = &self
            .record
            .registry
            .get(owner)
            .ok_or(Error::Unsupported(
                "58AD01 actual owner Kingdom generation absent",
            ))?
            .value;
        let aa_mapworld::restore::RestoredRecord::Kingdom(k) = value else {
            return required("58AD01 current owner is not a Kingdom");
        };
        let runtime = k
            .runtime
            .as_ref()
            .ok_or(Error::Unsupported("58AD01 actual Kingdom runtime absent"))?;
        if runtime.definition.native_class != 4 || runtime.base.actor.is_none() {
            return required("58AD01 actual Kingdom constructor differs");
        }
        runtime
            .derived
            .as_ref()
            .and_then(|d| d.words.get(&0x190))
            .copied()
            .ok_or(Error::Unsupported("58AD01 actual Kingdom190 uninitialized"))
    }
    fn high(&mut self) -> Result<u32> {
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .state
            .high)
    }
    fn low(&mut self) -> Result<u32> {
        low(self.record)
    }
    fn destroyed(&mut self) -> Result<bool> {
        Ok(self.record.shell.thing.flags & 1 != 0)
    }
    fn under_siege_enabled(&mut self) -> Result<bool> {
        self.context
            .variables
            .validate_catalog(self.catalog)
            .map_err(error)?;
        self.context.variables.require_published().map_err(error)?;
        Ok(self.context.variables.supply_provide_under_siege())
    }
    fn under_siege(&mut self) -> Result<bool> {
        Ok(low(self.record)? & 0x100000 != 0)
    }
    fn is_relay(&mut self) -> Result<bool> {
        Ok(source(self.catalog, self.record)?.is_relay)
    }
    fn stockpile_supplied(&mut self) -> Result<bool> {
        Ok(low(self.record)? & 0x80000 == 0
            || f32::from_bits(supply(self.record)?.stockpile.0) > 0.0)
    }
    fn range(&mut self) -> Result<u32> {
        Ok(supply(self.record)?.supply_range.0)
    }
    fn city(&mut self) -> Result<bool> {
        match self
            .record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::City)
        {
            None => Ok(false),
            Some(RestoredComponent::City(c)) if c.header.owner == self.record.owner => {
                Ok(c.byte_20 != 0)
            }
            _ => required("58AD93 actual City component class/owner differs"),
        }
    }
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    rng: &mut NativeRng,
    pc: Precision,
    context: &mut Context<'_, impl crate::load_detection_activation::Effects>,
) -> Result<bool> {
    checked(catalog, record)?;
    initialize(&mut Live {
        catalog,
        record,
        context,
        rng,
        pc,
    })
}
///Actual A400C0+14=4B2830 RET, without an invented Supply update.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}

#[cfg(test)]
#[path = "load_supply_init_world_tests.rs"]
mod tests;
