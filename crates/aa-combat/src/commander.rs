//! Original kingdom Commander replacement4E0F00, constructor4CE1D0 and
//! saved-state application4CE0B0/4C8B40. Definition resolution remains explicit.
//!
//! Commander experience (Commander+0xC, kingdom+0x230) is ONE native field. Its
//! single authoritative Rust store is the aa-economy type
//! `aa_economy::tick::CommanderExperience` held in [`State::experience`]; the
//! economy tick (4CDFD0), faction-ability affordability (4E24B0), payment
//! (4CDD90, [`crate::payment`]) and Load (4CE0B0) all read/write that one value.
//! There is no second experience copy in this crate.
use crate::{Error, Result, ability::Ability, constructors::LocalAbility};
use aa_body::F32;
use aa_economy::{F32Bits, tick::CommanderExperience};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition {
    pub id: u32,
    /// Native FactionData+24/+28, already resolved in original table order.
    pub abilities: Vec<LocalAbility>,
}
/// Marks FactionData+0x3C (`experience_max`, loader reference 48BE55) as not
/// yet supplied by the FactionData loader. It is deliberately non-finite:
/// aa-economy's Commander income (4CDFD0) rejects it, so no cap is invented.
/// Use [`State::with_maximum`] / [`replace_with_maximum`] with the loaded value
/// (a negative loaded value is the native "no cap").
pub const MAXIMUM_EXPERIENCE_UNBOUND: F32 = F32(0x7fc0_0000);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub definition: u32,
    pub kingdom: u32,
    /// Native+0C balance plus the definition cap, as the aa-economy view type.
    /// This is the authoritative store; bind `&mut state.experience` to the
    /// economy tick and to payment rather than copying it.
    pub experience: CommanderExperience,
    pub abilities: Vec<Ability>,
}
impl State {
    /// 4CE1D0 without a loaded FactionData+0x3C cap ([`MAXIMUM_EXPERIENCE_UNBOUND`]).
    pub fn new(definition: &Definition, kingdom: u32) -> Result<Self> {
        Self::with_maximum(definition, MAXIMUM_EXPERIENCE_UNBOUND, kingdom)
    }
    /// 4CE1D0: zero balance; `maximum_experience` is FactionData+0x3C, read by
    /// the economy tick through Commander+4 and exposed here for aa-economy.
    pub fn with_maximum(
        definition: &Definition,
        maximum_experience: F32,
        kingdom: u32,
    ) -> Result<Self> {
        let mut abilities = Vec::new();
        abilities
            .try_reserve_exact(definition.abilities.len())
            .map_err(|e| Error::World(format!("Commander ability allocation failed: {e}")))?;
        for input in &definition.abilities {
            let mut ability = Ability::new(
                input.definition,
                Some(input.local_definition),
                input.main_power,
            );
            ability.recharge_bypass = true;
            abilities.push(ability);
        }
        Ok(Self {
            definition: definition.id,
            kingdom,
            experience: CommanderExperience {
                balance: F32Bits::ZERO,
                maximum: F32Bits(maximum_experience.0),
            },
            abilities,
        })
    }
}
/// Destruction releases owned action/list storage; summoned actors themselves
/// are not destroyed. A failed new allocation leaves the old slot cleared.
pub fn replace(
    slot: &mut Option<State>,
    definition: Option<&Definition>,
    kingdom: u32,
) -> Result<()> {
    replace_with_maximum(
        slot,
        definition.map(|d| (d, MAXIMUM_EXPERIENCE_UNBOUND)),
        kingdom,
    )
}
/// 4E0F00 with the loaded FactionData+0x3C cap of the new definition. The old
/// Commander (and its experience store) is destroyed first, as natively.
pub fn replace_with_maximum(
    slot: &mut Option<State>,
    definition: Option<(&Definition, F32)>,
    kingdom: u32,
) -> Result<()> {
    *slot = None;
    if let Some((definition, maximum)) = definition {
        *slot = Some(State::with_maximum(definition, maximum, kingdom)?);
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedWords {
    pub serialized_count: u32,
    pub values: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedAbility {
    pub version: u32,
    pub use_count: u32,
    pub last_use: F32,
    pub word14: u32,
    pub enabled_byte: u8,
    pub summoned: Option<SavedWords>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedState {
    pub version: u32,
    pub experience: F32,
    pub serialized_count: u32,
    pub abilities: Vec<SavedAbility>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadOutcome {
    pub count_mismatch: bool,
    pub loaded: usize,
    pub discarded: usize,
}
fn count(words: u32) -> usize {
    (words as i32).max(0) as usize
}
/// Apply a fully decoded original action record, not an invented save format.
/// Existing definition/power/recharge-bypass fields survive Load. The intrusive
/// native list tail address is represented by Vec ownership, not copied into
/// Ability.word28's legacy zero placeholder. No binary save writer is claimed.
pub fn apply_ability(ability: &mut Ability, saved: &SavedAbility) -> Result<()> {
    if saved.version > 1 {
        return Err(Error::Invalid("unsupported saved Ability version"));
    }
    ability.use_count = saved.use_count;
    ability.last_use = if saved.use_count == 0 {
        F32(0xff7fffff)
    } else {
        saved.last_use
    };
    ability.word14 = saved.word14;
    ability.enabled = saved.enabled_byte != 0;
    if saved.version >= 1 {
        let words = saved
            .summoned
            .as_ref()
            .ok_or(Error::Invalid("saved Ability list missing"))?;
        if words.values.len() != count(words.serialized_count) {
            return Err(Error::Invalid(
                "saved Ability list count does not match decoded payload",
            ));
        }
        ability
            .summoned_ids
            .try_reserve(words.values.len())
            .map_err(|e| Error::World(format!("saved Ability list allocation failed: {e}")))?;
        ability
            .summoned_ids
            .splice(0..0, words.values.iter().rev().copied());
    }
    Ok(())
}
///4CE0B0 emits a diagnostic and consumes/discards every saved action when the
/// count differs. The caller must surface count_mismatch as a reconstruction
/// diagnostic. It must not claim those discarded action states were restored.
/// Input is the codec's complete payload. Prior field writes survive errors.
pub fn apply_saved(state: &mut State, saved: &SavedState) -> Result<LoadOutcome> {
    if saved.version != 0 {
        return Err(Error::Invalid("unsupported saved Commander version"));
    }
    state.experience.balance = F32Bits(saved.experience.0);
    if saved.abilities.len() != count(saved.serialized_count) {
        return Err(Error::Invalid(
            "saved Commander count does not match decoded payload",
        ));
    }
    let mismatch = saved.serialized_count as i32 != state.abilities.len() as i32;
    if mismatch {
        for payload in &saved.abilities {
            let mut discarded = Ability::new(0, None, F32::ZERO);
            apply_ability(&mut discarded, payload)?;
        }
    } else {
        for (ability, payload) in state.abilities.iter_mut().zip(&saved.abilities) {
            apply_ability(ability, payload)?;
        }
    }
    Ok(LoadOutcome {
        count_mismatch: mismatch,
        loaded: if mismatch { 0 } else { saved.abilities.len() },
        discarded: if mismatch { saved.abilities.len() } else { 0 },
    })
}
