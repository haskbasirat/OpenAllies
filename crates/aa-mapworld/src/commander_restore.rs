//! Lossless adapter from recovered Kingdom wire records to the independently
//! verified concrete Commander constructor/Load implementation.
use crate::{
    Error, Result,
    kingdoms::{AbilityState, CommanderState},
};
use aa_body::F32;
use aa_combat::commander;

pub fn saved_ability(source: &AbilityState) -> commander::SavedAbility {
    commander::SavedAbility {
        version: source.version,
        use_count: source.word_0c,
        last_use: F32(source.word_10),
        word14: source.word_14,
        enabled_byte: source.byte_18,
        summoned: source.words.as_ref().map(|words| commander::SavedWords {
            serialized_count: words.serialized_count,
            values: words.values.clone(),
        }),
    }
}
pub fn saved_state(source: &CommanderState) -> commander::SavedState {
    commander::SavedState {
        version: source.version,
        experience: F32(source.word_0c),
        serialized_count: source.abilities.serialized_count,
        abilities: source.abilities.values.iter().map(saved_ability).collect(),
    }
}
/// None in the original presence flag bypasses replacement altogether. Every
/// actual replacement destroys its predecessor before constructing a new one.
pub fn apply(
    slot: &mut Option<commander::State>,
    source: &crate::kingdoms::Commander,
    kingdom: u32,
    definition: Option<(&commander::Definition, F32)>,
) -> Result<Option<commander::LoadOutcome>> {
    if source.presence == Some(0) {
        return Ok(None);
    }
    // 4E0F00 with the new definition's FactionData+3C cap.
    commander::replace_with_maximum(slot, definition, kingdom)
        .map_err(|e| Error::Format(e.to_string()))?;
    if let Some(saved) = &source.state {
        let runtime = slot.as_mut().ok_or_else(|| {
            Error::Format("saved Commander payload has no resolved Commander instance".into())
        })?;
        let outcome = commander::apply_saved(runtime, &saved_state(saved))
            .map_err(|e| Error::Format(e.to_string()))?;
        Ok(Some(outcome))
    } else {
        Ok(None)
    }
}
