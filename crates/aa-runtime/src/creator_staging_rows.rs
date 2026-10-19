//! The selected Creator's KingdomParameter stores reached by the custom-game
//! staging processes (AA.exe): KKC_Session::SetPlayerKingdom 0x777F00
//! (`+0x24` handicap carried for AI players), SetAIHandicap 0x771880
//! (`+0x24`, `+0x68`), SetKingdomNation 0x777D90 -> 0x777CD0 (`+0x18`,
//! `+0x68`) and SetKingdomFaction 0x777700 -> 0x777610 (`+0x1C`, `+0x68`).
//!
//! Definition pointers are the canonical IDS of the definition the native
//! registry Find returned (empty = NULL), as everywhere in KingdomParameter.
//! Each store is one native write on the SAME validated Creator allocation;
//! callers keep the native order and partial effects.
use crate::{catalog::Catalog, definitions::Bank, resource_creator::Creator};

/// `0x50DC00`: the first record whose `+0` id equals `id` up to its first
/// NUL (wcscmp), or None.
pub fn kingdom_index(creator: &Creator, id: &[u16]) -> Option<usize> {
    let key = prefix(id);
    creator.parameters().kingdoms.iter().position(|k| prefix(&k.id) == key)
}

fn prefix(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&u| u == 0).unwrap_or(s.len())]
}

fn record<'a>(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &'a mut Creator,
    index: usize,
) -> Result<(&'a mut aa_mapworld::header::KingdomParameter, &'a mut u8), String> {
    creator.validate(catalog, bank)?;
    let (parameters, flags) = creator.human_assignment_fields_mut();
    let row = parameters.kingdoms.get_mut(index).ok_or("staging Creator record is absent")?;
    let flag = flags.get_mut(index).ok_or("staging Creator modified68 record is absent")?;
    Ok((row, flag))
}

/// `+0x24 = handicap` (0x777F00's AI carry 0x7780FD, or its 0x778000 clear
/// for a human on a playable record). No `+0x68` store.
pub fn store_handicap(catalog: &Catalog<'_>, bank: &Bank, creator: &mut Creator, index: usize, ids: &[u16]) -> Result<(), String> {
    let (row, _) = record(catalog, bank, creator, index)?;
    row.handicap = prefix(ids).to_vec();
    Ok(())
}

/// 0x771967..0x77196E: `+0x24 = handicap`, then `+0x68 = 1`.
pub fn store_handicap_and_mark(catalog: &Catalog<'_>, bank: &Bank, creator: &mut Creator, index: usize, ids: &[u16]) -> Result<(), String> {
    let (row, flag) = record(catalog, bank, creator, index)?;
    row.handicap = prefix(ids).to_vec();
    *flag = 1;
    Ok(())
}

/// 0x777D51: `+0x18 = Nation Find(ids)` (empty = NULL).
pub fn store_nation(catalog: &Catalog<'_>, bank: &Bank, creator: &mut Creator, index: usize, ids: &[u16]) -> Result<(), String> {
    let (row, _) = record(catalog, bank, creator, index)?;
    row.nation = prefix(ids).to_vec();
    Ok(())
}

/// 0x777610's `+0x1C` stores (0x777685, 0x7776B5, 0x7776E8, 0x7776F5),
/// with or without the `+0x68 = 1` of 0x7776B8 / 0x7776F1.
pub fn store_faction(
    catalog: &Catalog<'_>,
    bank: &Bank,
    creator: &mut Creator,
    index: usize,
    ids: &[u16],
    mark: bool,
) -> Result<(), String> {
    let (row, flag) = record(catalog, bank, creator, index)?;
    if mark {
        *flag = 1;
    }
    row.faction = prefix(ids).to_vec();
    Ok(())
}

/// `+0x68 = 1` (0x777D67 / 0x777D80).
pub fn mark_modified(catalog: &Catalog<'_>, bank: &Bank, creator: &mut Creator, index: usize) -> Result<(), String> {
    let (_, flag) = record(catalog, bank, creator, index)?;
    *flag = 1;
    Ok(())
}
