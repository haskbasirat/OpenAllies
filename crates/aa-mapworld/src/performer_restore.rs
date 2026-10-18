//! Original57F970 saved Performer state applied to the constructor's owned
//! component and the same world Ability arena. No action instance is recreated.
use crate::{Error, Result, saved_components::PerformerPayload};
use aa_body::F32;
use aa_combat::{ability::Ability, constructors::OwnedPerformer};
use aa_numeric::{Binary, Precision};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct LoadOutcome {
    pub turret_count_mismatch: bool,
    pub ability_count_mismatch: bool,
    pub abilities_loaded: usize,
    pub abilities_discarded: usize,
}

/// Count mismatches are native diagnostics, not failures: excess turret records
/// overwrite the final existing turret; excess Ability records are discarded.
/// Negative signed counts execute no iterations. The immutable source payload
/// remains available to report those diagnostics after restoration.
pub fn apply(
    state: &mut OwnedPerformer,
    source: &PerformerPayload,
    pc: Precision,
    apply_ability: &crate::restore_components::ApplyPerformerAbility<'_>,
) -> Result<LoadOutcome> {
    if !(5..=9).contains(&source.version) {
        return Err(Error::Format("unsupported saved Performer version".into()));
    }
    state.state.stage = source.word_10 as i32;
    let mana = Binary::from_f32_bits(source.multiplier_20)
        .and_then(|v| v.mul(&Binary::from_f32_bits(state.maximum_mana.0)?, pc))
        .and_then(|v| v.store_f32())
        .map_err(|e| Error::Format(e.to_string()))?;
    state.state.mana = F32(mana);
    // Original FCOMP/test-AH1 then test-AH41/JNP preserves negative finite
    // balances. Only the unordered branch writes zero; unsupported NaN
    // arithmetic is rejected by Binary above rather than guessed here.
    state.state.target_id = source.word_28;
    state.state.target_position = source.words_2c_30.map(F32);
    state.state.active_index = source.ability_index;
    if source.ability_index >= 0 && source.ability_index as usize >= state.state.abilities.len() {
        return Err(Error::Format(
            "native Performer active Ability index exceeds constructed count".into(),
        ));
    }
    state.state.active_ability = if source.ability_index < 0 {
        None
    } else {
        Some(state.state.abilities[source.ability_index as usize])
    };
    state.state.started_at = F32(source.words_3c_40[0]);
    state.state.animation_length = F32(source.words_3c_40[1]);
    let turret_count_mismatch = source.states.serialized_count as i32 != state.turrets.len() as i32;
    if source.states.values.len() != (source.states.serialized_count as i32).max(0) as usize {
        return Err(Error::Format(
            "decoded Performer turret count differs from wire count".into(),
        ));
    }
    for (index, saved) in source.states.values.iter().enumerate() {
        let last = state.turrets.len().checked_sub(1);
        if let Some(last) = last {
            let turret = &mut state.turrets[index.min(last)];
            turret.current_angle = F32(saved.words_00_04[0]);
            turret.target_angle = F32(saved.words_00_04[1]);
            if let Some(time) = saved.word_08 {
                turret.time = F32(time);
            }
            if saved.present_byte.is_some_and(|v| v != 0) {
                turret.pending = true;
                let xy = saved
                    .words_10_14
                    .ok_or_else(|| Error::Format("Performer turret target missing".into()))?;
                turret.target[0] = F32(xy[0]);
                turret.target[1] = F32(xy[1]);
            }
            if let Some(z) = saved.word_18 {
                turret.target[2] = F32(z);
            }
        }
    }
    state.state.turret_index = source.word_58 as i32;
    if let Some(event) = source.word_44 {
        state.state.event_kind = event;
    }
    let ability_count_mismatch =
        source.abilities.serialized_count as i32 != state.state.abilities.len() as i32;
    if source.abilities.values.len() != (source.abilities.serialized_count as i32).max(0) as usize {
        return Err(Error::Format(
            "decoded Performer Ability count differs from wire count".into(),
        ));
    }
    for (index, payload) in source.abilities.values.iter().enumerate() {
        let saved = crate::commander_restore::saved_ability(payload);
        if let Some(handle) = state.state.abilities.get(index) {
            apply_ability(*handle, &saved).map_err(Error::Format)?;
        } else {
            // Native4C8610/4C8B40 consumes this record into a temporary action.
            let mut discarded = Ability::new(0, None, F32::ZERO);
            aa_combat::commander::apply_ability(&mut discarded, &saved)
                .map_err(|e| Error::Format(e.to_string()))?;
        }
    }
    if let Some(pending) = source.byte_5c {
        state.turret_update_pending = pending != 0;
    }
    Ok(LoadOutcome {
        turret_count_mismatch,
        ability_count_mismatch,
        abilities_loaded: source
            .abilities
            .values
            .len()
            .min(state.state.abilities.len()),
        abilities_discarded: source
            .abilities
            .values
            .len()
            .saturating_sub(state.state.abilities.len()),
    })
}
