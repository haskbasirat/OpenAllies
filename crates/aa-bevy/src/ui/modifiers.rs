//! Property modifier descriptions (the modifier classes' vfunc +0x14, used
//! by the property and research tooltips through `0x63BFD0`). Each line is
//! its modifier_desc_* string, coloured with UVar UIColorModifierPositive /
//! Negative / Neutral (`0x4FEF60`, slots 0x8C / 0x8D / 0x8E).
//!
//! The modifier kinds and their native enums (`0x4C1000` registers them):
//! * FLAG (`0x4FF070`): TRAILBLAZING, PARALYZE, ENTANGLE, BERSERK, SUMMONED,
//!   SUPPLIED, INVISIBLE_TO_ENEMY, REVEAL_ENEMY_RZOD, MINIMAP_NO_FOG,
//!   NO_UPKEEP, NOT_CAPTURABLE, NOT_DISPOSABLE, each with its own key and
//!   colour;
//! * ADD (`0x4FFAA0`): attributes 0..12 (AV ... TERRAIN_SPEED_CAP),
//!   "%+g" of the amount; positive colour above 0, negative below, neutral
//!   at 0, reversed for MORALE_BREAKPOINT and TERRAIN_SPEED_CAP;
//! * MULTIPLY (`0x5002B0`): attributes 13..24 (SPEED_MAX ... RECHARGE_TIME),
//!   "%+g%%" of 100 x the amount; STRUCTURE_COST and RECRUIT_COST count a
//!   rise as negative;
//! * RESIST_DAMAGE (`0x501080` -> `0x500AB0`): the damage type's glyph and
//!   "%+g%%" of 100 x the amount, positive above 0, else negative;
//! * RESOURCE_PRODUCTION (`0x4FE3F0` -> `0x5C4CB0` / `0x5C4BD0`): every
//!   non-zero resource as glyph and "%+g" (a limited resource "%.0f" of
//!   its absolute value), no separator, uncoloured.
//! Enchantment immunity, ability effect, periodic damage / heal and reverse
//! ability modifiers, ADD TERRAIN_SPEED_CAP and MULTIPLY ATTACK_EFFICIENCY_*
//! are not used by Axis & Allies' properties and give no line here.
use aa_play::data::Modifier;
use aa_ui::ui::Ui;

const ADD: [&str; 13] = [
    "AV",
    "AV_MELEE",
    "AV_RANGED",
    "DV",
    "ABILITY_HEAL",
    "SUPPLY_RANGE",
    "DETECTION_RANGE",
    "MORALE",
    "MORALE_BREAKPOINT",
    "PROJECTILE_ABILITY_RANGE",
    "BOMBARD_ABILITY_RANGE",
    "AUTO_UPGRADE",
    "TERRAIN_SPEED_CAP",
];

const MULTIPLY: [&str; 12] = [
    "SPEED_MAX",
    "RESUPPLY_RATE",
    "ENTRENCHMENT_RATE",
    "MORALE_RECOVERY_RATE",
    "MORALE_LOSS_RATE",
    "STRUCTURE_COST",
    "HEALTH",
    "CONTROL_RANGE",
    "ATTACK_EFFICIENCY_ACTUAL",
    "ATTACK_EFFICIENCY_DESIRED",
    "RECRUIT_COST",
    "RECHARGE_TIME",
];

#[derive(Clone, Copy)]
enum Tone {
    Positive,
    Negative,
    Neutral,
}

/// FLAG index order, its key and colour (the `0x4FFA64` cases).
const FLAGS: [(&str, &str, Tone); 12] = [
    ("TRAILBLAZING", "#modifier_desc_flag_trailblazing", Tone::Positive),
    ("PARALYZE", "#modifier_desc_flag_paralyze", Tone::Negative),
    ("ENTANGLE", "#modifier_desc_flag_entangle", Tone::Negative),
    ("BERSERK", "#modifier_desc_flag_berserk", Tone::Neutral),
    ("SUMMONED", "#modifier_desc_flag_summoned", Tone::Neutral),
    ("SUPPLIED", "#modifier_desc_flag_supplied", Tone::Positive),
    ("INVISIBLE_TO_ENEMY", "#modifier_desc_flag_invisible_to_enemy", Tone::Positive),
    ("REVEAL_ENEMY_RZOD", "#modifier_desc_flag_reveal_enemy_rzod", Tone::Positive),
    ("MINIMAP_NO_FOG", "#modifier_desc_flag_minimap_no_fog", Tone::Positive),
    ("NO_UPKEEP", "#modifier_desc_flag_no_upkeep", Tone::Positive),
    ("NOT_CAPTURABLE", "#modifier_desc_flag_not_capturable", Tone::Neutral),
    ("NOT_DISPOSABLE", "#modifier_desc_flag_not_disposable", Tone::Neutral),
];

/// A colour UVar, or the native registration default when the data has
/// none.
fn uvar_color(ui: &Ui, name: &str, default: [f32; 3]) -> [f32; 3] {
    if ui.uvars.get_str(name).trim().is_empty() { default } else { ui.uvars.get_point3(name) }
}

fn tone(ui: &Ui, text: &str, t: Tone) -> String {
    let rgb = match t {
        Tone::Positive => uvar_color(ui, "UIColorModifierPositive", [0.0, 1.0, 0.0]),
        Tone::Negative => uvar_color(ui, "UIColorModifierNegative", [1.0, 0.0, 0.0]),
        Tone::Neutral => uvar_color(ui, "UIColorModifierNeutral", [1.0, 1.0, 1.0]),
    };
    aa_ui::text::color_wrap(text, rgb)
}

/// C printf "%+g".
fn plus_g(v: f64) -> String {
    let s = aa_ui::staging::fmt_g(v);
    if v >= 0.0 && !s.starts_with('-') { format!("+{s}") } else { s }
}

/// Replace the first printf conversion of `pattern` (`%+g`, `%g`, `%s`,
/// `%lc`, `%d`) by `value`.
fn arg(text: &str, pattern: &str, value: &str) -> String {
    text.replacen(pattern, value, 1)
}

fn resolve(ui: &Ui, key: &str) -> String {
    ui.strings.resolve(key)
}

/// The tone of a signed amount: positive above 0, negative below, neutral
/// at 0 (`reverse` swaps the two signs).
fn signed(v: f32, reverse: bool) -> Tone {
    if v > 0.0 {
        if reverse { Tone::Negative } else { Tone::Positive }
    } else if v < 0.0 {
        if reverse { Tone::Positive } else { Tone::Negative }
    } else {
        Tone::Neutral
    }
}

/// The description line of one modifier (`repair`: the AddModifier
/// `repair` field, modifier +0x14 bit 0, choosing the repair key for
/// ABILITY_HEAL), or `None` for a kind without one.
pub fn describe(ui: &Ui, data: &aa_play::GameData, resources: &[aa_ui::hud::HudResource], m: &Modifier, repair: bool) -> Option<String> {
    let attr = m.attribute.as_deref().unwrap_or("").to_ascii_uppercase();
    match m.kind.as_str() {
        "FLAG" => {
            let flag = m.flag.as_deref().unwrap_or("").to_ascii_uppercase();
            let (_, key, t) = FLAGS.iter().find(|(f, _, _)| *f == flag)?;
            Some(tone(ui, &resolve(ui, key), *t))
        }
        "ADD" => {
            let i = ADD.iter().position(|a| *a == attr)?;
            let v = m.amount as f64;
            let (text, reverse) = match i {
                4 if repair => (arg(&resolve(ui, "#modifier_desc_add_ability_repair|f"), "%+g", &plus_g(v)), false),
                // AUTO_UPGRADE: the description of a property (no actor)
                // is the immediate one
                11 => (resolve(ui, "#modifier_desc_add_auto_upgrade_immediate"), false),
                // TERRAIN_SPEED_CAP names a terrain type: not used by the
                // Axis & Allies properties
                12 => return None,
                _ => {
                    let key = [
                        "#modifier_desc_add_av|f",
                        "#modifier_desc_add_av_melee|f",
                        "#modifier_desc_add_av_ranged|f",
                        "#modifier_desc_add_dv|f",
                        "#modifier_desc_add_ability_heal|f",
                        "#modifier_desc_add_supply_range|f",
                        "#modifier_desc_add_detection_range|f",
                        "#modifier_desc_add_morale|f",
                        "#modifier_desc_add_morale_breakpoint|f",
                        "#modifier_desc_add_projectile_ability_range|f",
                        "#modifier_desc_add_bombard_ability_range|f",
                    ][i];
                    (arg(&resolve(ui, key), "%+g", &plus_g(v)), i == 8)
                }
            };
            Some(tone(ui, &text, signed(m.amount, reverse)))
        }
        "MULTIPLY" => {
            let i = MULTIPLY.iter().position(|a| *a == attr)?;
            let mut v = m.amount * 100.0;
            let key = match i {
                0 => "#modifier_desc_multiply_speed_max|f",
                1 => "#modifier_desc_multiply_resupply_rate|f",
                2 => "#modifier_desc_multiply_entrenchment_rate|f",
                3 => "#modifier_desc_multiply_morale_recovery_rate|f",
                4 => "#modifier_desc_multiply_morale_loss_rate|f",
                5 => "#modifier_desc_multiply_structure_cost|f",
                6 => "#modifier_desc_multiply_health|f",
                7 => "#modifier_desc_multiply_control_range|f",
                // ATTACK_EFFICIENCY_ACTUAL / _DESIRED choose between four
                // keys by the actor's current efficiency (0x500461): not
                // used by the Axis & Allies properties
                8 | 9 => return None,
                10 => "#modifier_desc_multiply_recruit_cost|f",
                _ => "#modifier_desc_multiply_recharge_time|f",
            };
            let text = arg(&resolve(ui, key), "%+g", &plus_g(v as f64)).replace("%%", "%");
            // 0x500765: a cost rise is negative
            if i == 5 || i == 10 {
                v = -v;
            }
            Some(tone(ui, &text, signed(v, false)))
        }
        "RESIST_DAMAGE" => {
            let glyph = m
                .damage_type
                .and_then(|d| damage_glyph(ui, data, d as usize))
                .unwrap_or_default();
            // 100 x the resistance, -100 x for a vulnerability
            let (key, t, v) = if m.amount > 0.0 {
                ("#modifier_desc_resist_damage_positive|cf", Tone::Positive, m.amount * 100.0)
            } else {
                ("#modifier_desc_resist_damage_negative|cf", Tone::Negative, m.amount * -100.0)
            };
            let text = arg(&arg(&resolve(ui, key), "%lc", &glyph), "%+g", &plus_g(v as f64)).replace("%%", "%");
            Some(tone(ui, &text, t))
        }
        "RESOURCE_PRODUCTION" => {
            // the resource registry order (HUD resources), the production
            // in GameData::resources order
            let mut out = String::new();
            for r in resources {
                let v = data.resource_index(&r.ids).and_then(|j| m.production.get(j).copied()).unwrap_or(0.0);
                if v == 0.0 {
                    continue;
                }
                let glyph = ui.strings.glyphs.get(r.icon.to_ascii_lowercase().as_str()).copied().map(String::from).unwrap_or_default();
                if r.kind == 2 {
                    out.push_str(&format!("{glyph}{:.0}", v.abs()));
                } else {
                    out.push_str(&format!("{glyph}{}", plus_g(v as f64)));
                }
            }
            Some(out)
        }
        _ => None,
    }
}

/// A damage type's glyph (DamageType `glyph`, damage type +0x28 -> +0x18).
fn damage_glyph(ui: &Ui, data: &aa_play::GameData, index: usize) -> Option<String> {
    let ids = data.damage_types.get(index)?;
    let glyph = super::datafile::load(&ui.store.root, "Game/damage_types.tgi")
        .iter()
        .filter(|n| n.name.eq_ignore_ascii_case("DamageType"))
        .find(|n| n.get("IDS").is_some_and(|i| i.eq_ignore_ascii_case(ids)))
        .and_then(|n| n.get("glyph").map(|g| g.trim().to_ascii_lowercase()))?;
    ui.strings.glyphs.get(glyph.as_str()).copied().map(String::from)
}
