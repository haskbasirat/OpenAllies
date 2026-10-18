//!5687E0/568070 over the SAME held Denizen allocation and canonical source.
//!Each source node OR commits immediately; version/head/source/queue gates
//!remain lazy. Positive Zone5 operations require the actual shared zone owner.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::Precision;
use aa_simulation::NativeRng;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    let pc = crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|m| m & (1 << ComponentSlot::Denizen.index()) == 0)
    {
        return Err(Error::Unsupported(
            "5687E0 Denizen outside actual source factory",
        ));
    }
    match record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(ComponentSlot::Denizen))
    {
        Some(RestoredComponent::Denizen(d)) if d.header.owner == record.owner => Ok(pc),
        _ => Err(Error::Unsupported(
            "5687E0 current Denizen allocation/owner differs",
        )),
    }
}
fn mask(record: &Record<'_>) -> Result<u32> {
    match record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(ComponentSlot::Denizen))
    {
        Some(RestoredComponent::Denizen(d)) if d.header.owner == record.owner => {
            d.words.get(&0x38).copied().ok_or(Error::Unsupported(
                "5687E0 actual initialized Denizen38 absent",
            ))
        }
        _ => Err(Error::Unsupported(
            "5687E0 current Denizen allocation/owner differs",
        )),
    }
}
fn guard(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<u32> {
    let pc = checked(catalog, record)?;
    catalog
        .actors
        .native_denizen_guards(&record.definition.proof.id, pc)
        .map_err(error)?
        .map(|v| v.finalized_bits[0])
        .ok_or(Error::Unsupported(
            "568070 actual DenizenData364 unavailable",
        ))
}
struct Row {
    ids: String,
    bits: u32,
}
struct SourceRows;
impl SourceRows {
    fn read(catalog: &Catalog<'_>, record: &Record<'_>, list: u32) -> Result<Vec<u32>> {
        let pc = checked(catalog, record)?;
        let root = catalog
            .actors
            .resolved_actor(&record.definition.proof.id)
            .map_err(error)?;
        let source = root
            .child("DenizenComponent")
            .map_err(error)?
            .ok_or(Error::Unsupported("5687E0 source DenizenComponent absent"))?;
        if list == 0x34c {
            let mut organization = Vec::new();
            for row in source.children("Organization").map_err(error)? {
                let ids = match row.field("organization_ids", pc).map_err(error)? {
                    Some(aa_rules::native::TypedValue::String { value, .. }) => value,
                    _ => return Err(Error::Unsupported("467177 Organization IDS source absent")),
                };
                let bits = behavior(&row, pc)?;
                if bits == 0 {
                    return Err(Error::Unsupported(
                        "463740 Organization behavior cannot be empty",
                    ));
                }
                //4671BD..4672AD groups repeated case-sensitive IDS after its last run.
                aa_rules::native::denizen::native_inventory_insert(
                    &mut organization,
                    Row { ids, bits },
                    |r| &r.ids,
                );
            }
            return Ok(organization.into_iter().map(|r| r.bits).collect());
        }
        if list != 0x354 {
            return Err(Error::Unsupported("5687E0 source list differs"));
        }
        let mut groups = Vec::new();
        //4672E1..46734E appends DenizenGroup nodes through45F820/463800.
        for row in source.children("DenizenGroup").map_err(error)? {
            let bits = behavior(&row, pc)?;
            if bits == 0 {
                return Err(Error::Unsupported(
                    "463800 DenizenGroup behavior cannot be empty",
                ));
            }
            groups.push(bits)
        }
        Ok(groups)
    }
}
fn behavior(row: &aa_rules::actors::ResolvedNode<'_>, pc: Precision) -> Result<u32> {
    match row.field("behavior", pc).map_err(error)? {
        Some(aa_rules::native::TypedValue::String { value, .. }) => {
            aa_rules::native::denizen::native_behavior_flags(&value).map_err(error)
        }
        _ => Err(Error::Unsupported(
            "463520 actual behavior source string absent",
        )),
    }
}
pub(crate) trait MaskWorld {
    type Node: Copy;
    fn head(&mut self, list: u32) -> Result<Option<Self::Node>>;
    fn value(&mut self, node: Self::Node) -> Result<u32>;
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    fn or_mask(&mut self, value: u32) -> Result<()>;
}
pub(crate) fn merge(world: &mut impl MaskWorld) -> Result<()> {
    for list in [0x34c, 0x354] {
        let mut node = world.head(list)?;
        while let Some(n) = node {
            let bits = world.value(n)?;
            world.or_mask(bits)?;
            node = world.next(n)?;
        }
    }
    Ok(())
}
struct Merge<'a, 'r, 'source> {
    catalog: &'a Catalog<'source>,
    record: &'a mut Record<'r>,
    rows: std::collections::BTreeMap<u32, Vec<u32>>,
}
impl Merge<'_, '_, '_> {
    fn values(&self, list: u32) -> Result<&[u32]> {
        self.rows
            .get(&list)
            .map(|v| v.as_slice())
            .ok_or(Error::Unsupported("5687E0 source head has not been read"))
    }
}
impl MaskWorld for Merge<'_, '_, '_> {
    type Node = (u32, usize);
    fn head(&mut self, list: u32) -> Result<Option<Self::Node>> {
        //Do not bind the second source list before first-list stores complete.
        self.rows
            .insert(list, SourceRows::read(self.catalog, self.record, list)?);
        Ok((!self.values(list)?.is_empty()).then_some((list, 0)))
    }
    fn value(&mut self, (list, index): Self::Node) -> Result<u32> {
        self.values(list)?
            .get(index)
            .copied()
            .ok_or(Error::Unsupported("5687E0 canonical node absent"))
    }
    fn next(&mut self, (list, index): Self::Node) -> Result<Option<Self::Node>> {
        let n = index + 1;
        Ok((n < self.values(list)?.len()).then_some((list, n)))
    }
    fn or_mask(&mut self, bits: u32) -> Result<()> {
        let Some(RestoredComponent::Denizen(d)) = self
            .record
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .get_mut(ComponentSlot::Denizen)
        else {
            unreachable!()
        };
        if d.header.owner != self.record.owner {
            return Err(Error::Unsupported("5687E0 current Denizen owner changed"));
        }
        let word = d.words.get_mut(&0x38).ok_or(Error::Unsupported(
            "5687E0 actual initialized Denizen38 absent",
        ))?;
        *word |= bits;
        Ok(())
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    rng: &mut NativeRng,
    pc: Precision,
    world_version: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    checked(catalog, record)?;
    merge(&mut Merge {
        catalog,
        record,
        rows: std::collections::BTreeMap::new(),
    })?;
    crate::load_detection_init_world::schedule_old_existing_when(
        catalog,
        record,
        ComponentSlot::Denizen,
        rng,
        pc,
        world_version,
        || {
            if f32::from_bits(guard(catalog, record)?) > 0.0 {
                return Ok(true);
            }
            checked(catalog, record)?;
            let root = catalog
                .actors
                .resolved_actor(&record.definition.proof.id)
                .map_err(error)?;
            let source = root
                .child("DenizenComponent")
                .map_err(error)?
                .ok_or(Error::Unsupported("568848 source Denizen absent"))?;
            //The head is nonnull even when every authored count is zero/negative.
            Ok(!source.children("Inventory").map_err(error)?.is_empty())
        },
    )
}

pub trait ZoneServices {
    fn create_zone5(&mut self, record: &mut Record<'_>) -> Result<bool>;
    fn zone5_present(&mut self, record: &Record<'_>) -> Result<bool>;
    fn set_zone5_flag(&mut self, record: &mut Record<'_>, bit: u32, on: u32) -> Result<()>;
    fn move_zone5(
        &mut self,
        record: &mut Record<'_>,
        xy: [u32; 2],
        radius: u32,
        pc: Precision,
    ) -> Result<()>;
}
pub struct RequiredZones;
impl ZoneServices for RequiredZones {
    fn create_zone5(&mut self, _: &mut Record<'_>) -> Result<bool> {
        Err(Error::Unsupported("568099 actual shared CreateZone5 owner"))
    }
    fn zone5_present(&mut self, _: &Record<'_>) -> Result<bool> {
        Err(Error::Unsupported(
            "5680A6 actual current Actor Zone5 vector",
        ))
    }
    fn set_zone5_flag(&mut self, _: &mut Record<'_>, _: u32, _: u32) -> Result<()> {
        Err(Error::Unsupported("5680C3 actual Zone5 flag owner"))
    }
    fn move_zone5(&mut self, _: &mut Record<'_>, _: [u32; 2], _: u32, _: Precision) -> Result<()> {
        Err(Error::Unsupported(
            "5680FF actual same Zone5 move/activation owner",
        ))
    }
}
trait AfterWorld {
    fn guard(&mut self) -> Result<u32>;
    fn flags(&mut self) -> Result<u32>;
    fn create(&mut self) -> Result<bool>;
    fn mask(&mut self) -> Result<u32>;
    fn zone_present(&mut self) -> Result<bool>;
    fn zone_flag(&mut self) -> Result<()>;
    fn position(&mut self) -> Result<[u32; 2]>;
    fn move_zone(&mut self, xy: [u32; 2], radius: u32) -> Result<()>;
}
fn after(world: &mut impl AfterWorld) -> Result<()> {
    if f32::from_bits(world.guard()?) == 0.0 || world.flags()? & 1 != 0 {
        return Ok(());
    }
    let _ = world.create()?; //56809B ignores CreateZone's AL.
    if world.mask()? & 0x20 != 0 {
        if !world.zone_present()? {
            return Err(Error::NullDereference("515940 on a null Zone5"));
        }
        world.zone_flag()?;
    }
    let radius = world.guard()?;
    let xy = world.position()?;
    //5680F4 selects the current slot (possibly NULL) inside the move service.
    //515F40 reads GMap before dereferencing it; do not invent an earlier error.
    world.move_zone(xy, radius)
}
struct After<'a, 'r, 'source, S> {
    catalog: &'a Catalog<'source>,
    record: &'a mut Record<'r>,
    services: &'a mut S,
    pc: Precision,
}
impl<S: ZoneServices> AfterWorld for After<'_, '_, '_, S> {
    fn guard(&mut self) -> Result<u32> {
        guard(self.catalog, self.record)
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(self.record.shell.thing.flags)
    }
    fn create(&mut self) -> Result<bool> {
        self.services.create_zone5(self.record)
    }
    fn mask(&mut self) -> Result<u32> {
        mask(self.record)
    }
    fn zone_present(&mut self) -> Result<bool> {
        self.services.zone5_present(self.record)
    }
    fn zone_flag(&mut self) -> Result<()> {
        self.services.set_zone5_flag(self.record, 1, 1)
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        Ok([
            self.record.shell.thing.position_bits[0],
            self.record.shell.thing.position_bits[1],
        ])
    }
    fn move_zone(&mut self, xy: [u32; 2], radius: u32) -> Result<()> {
        self.services.move_zone5(self.record, xy, radius, self.pc)
    }
}
pub fn after_init(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    services: &mut impl ZoneServices,
    pc: Precision,
) -> Result<()> {
    checked(catalog, record)?;
    after(&mut After {
        catalog,
        record,
        services,
        pc,
    })
}

#[cfg(test)]
#[path = "load_denizen_init_world_tests.rs"]
mod tests;
