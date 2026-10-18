//! Narrow777F00 ordinary human store over the SAME current Creator record.
//! Selection and player writes belong to the retained Session adapter.
use crate::{catalog::Catalog, definitions::Bank, resource_creator::Creator};

///777FFC..778003 clears the new enabled record's Handicap reference for a
///human. This does not initialize Nation/Faction, change unrelated source68
///flags or certify the rest of SetPlayerKingdom.
pub fn clear_handicap(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (parameters, _) = creator.label_fields_mut();
    let record = parameters
        .kingdoms
        .get_mut(index)
        .ok_or("human assignment current Creator record is absent")?;
    if record.byte_28 == 0 {
        return Err("human assignment reached disabled Creator record".into());
    }
    record.handicap.clear();
    Ok(())
}

///777D51, after actual Find(empty) returned nativeNULL. No inferred Nation
///or Faction reference is introduced. Caller performs the nested diagnostic
///before the following Faction/modified stores, preserving partial effects.
pub fn store_null_nation(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (parameters, _) = creator.human_assignment_fields_mut();
    parameters
        .kingdoms
        .get_mut(index)
        .ok_or("current Creator Nation record is absent")?
        .nation
        .clear();
    Ok(())
}
///777685 writes FactionNULL when the SAME current Nation is null. Native
///does not mark68 on this early return; the caller777CD0 does so afterward.
pub fn clear_faction_null_nation(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (parameters, _) = creator.human_assignment_fields_mut();
    let record = parameters
        .kingdoms
        .get_mut(index)
        .ok_or("current Creator Faction record is absent")?;
    if !record.nation.is_empty() {
        return Err("Faction-null-Nation leaf requires actual prior NationNULL store".into());
    }
    record.faction.clear();
    Ok(())
}
///777D67/777D80 actual caller's later+68 store, after the nested function.
pub fn mark_modified(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (_, flags) = creator.human_assignment_fields_mut();
    *flags
        .get_mut(index)
        .ok_or("current Creator modified68 record is absent")? = 1;
    Ok(())
}
///7776AA..7776B5: actual Find(empty) returnedNULL after a nonnull Nation.
///The FactionNULL store precedes modified68=1; no callback lies between them.
pub fn clear_faction_and_mark(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
) -> Result<(), String> {
    creator.validate(catalog, bank)?;
    let (parameters, flags) = creator.human_assignment_fields_mut();
    parameters
        .kingdoms
        .get_mut(index)
        .ok_or("current Creator Faction record is absent")?
        .faction
        .clear();
    *flags
        .get_mut(index)
        .ok_or("current Creator modified68 record is absent")? = 1;
    Ok(())
}
