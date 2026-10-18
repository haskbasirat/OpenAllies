use aa_actors::actor_creation::{Error,Result};
use aa_numeric::{Binary,Precision};
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
fn required<T>(s:&'static str)->Result<T>{Err(Error::Unsupported(s))}
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


#[path="../../crates/aa-runtime/src/load_supply_init_world_tests.rs"]
mod tests;
