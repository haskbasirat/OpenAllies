//!Exact derived42B6E0 types2/4/5/6/7/9 after common42C1B0.
//!Texture fields retain actual Resource handles and required current effects.
use super::{Draft, Inputs, Node, NodeId, Result};
use aa_numeric::{Binary, Precision};
use aa_rules::resources::Provenance;

pub(super) fn construct(kind: u8, source: Vec<Provenance>) -> Result<Node> {
    if !matches!(kind, 2 | 4 | 5 | 6 | 7 | 9) {
        return Err("unexpected FX subtype constructor".into());
    }
    let mut node = Node::common(source);
    if kind == 5 {
        node.texts.insert(0x3c, String::new());
    } //42B270 actual CString
    if kind == 7 {
        //42B090 writes all8 bytes; their completed DWORDs are zero.
        node.words.insert(0x40, 0);
        node.words.insert(0x44, 0);
    }
    Ok(node)
}
pub(super) fn load<I: Inputs>(
    kind: u8,
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    match kind {
        2 => equipment(input, source, draft, id)?,
        4 => stop_sfx(input, source, draft, id)?,
        5 => change_aspect(input, source, draft, id)?,
        6 => change_skin(input, source, draft, id)?,
        7 => create_rfx(input, source, draft, id)?,
        9 => hit_effect(input, source, draft, id)?,
        _ => return Err("unexpected FX derived loader".into()),
    }
    draft.nodes[id.0].complete = true;
    Ok(id)
}
///42C9D0: every destination store precedes its dependent validation.
fn equipment<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let action = input.text(s, "action")?;
    let value = input.enum_value("EquipmentAction", &action)?;
    d.store(id, 0x3c, value as u32);
    if value < 0 {
        return Err(format!(
            "Unknown ACTION '{action}' when modifying equipment"
        ));
    }
    let location = input.text(s, "location")?;
    let value = input
        .find("Bone", &location)?
        .and_then(|r| r.ordinal)
        .map(|v| v as i32)
        .unwrap_or(-1);
    d.store(id, 0x40, value as u32);
    if value < 0 {
        return Err(format!(
            "Unknown LOCATION '{location}' when modifying equipment"
        ));
    }
    d.reference(id, 0x44, None);
    let equipment_type = input.text(s, "equipment_type_ids")?;
    if !equipment_type.is_empty() {
        let r = input.find("EquipmentType", &equipment_type)?;
        let missing = r.is_none();
        d.reference(id, 0x44, r);
        if missing {
            return Err(format!("Unknown equipment type '{equipment_type}'"));
        }
    }
    d.reference(id, 0x48, None);
    let equipment = input.text(s, "equipment_IDS")?;
    if matches!(d.nodes[id.0].word(0x3c), Some(4 | 5)) {
        if equipment.is_empty() {
            return Err(format!("Must specify equipment for action '{action}'"));
        }
        let r = input.find("Equipment", &equipment)?;
        let missing = r.is_none();
        d.reference(id, 0x48, r);
        if missing {
            return Err(format!("Could not find equipment data '{equipment}'"));
        }
    } else if !equipment.is_empty() {
        return Err(format!("Action '{action}' should not specify equipment"));
    }
    Ok(())
}
///42C530 stores even a null Audio result before native throws.
fn stop_sfx<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "audio_ids")?;
    let r = input.find("Audio", &text)?;
    let missing = r.is_none();
    d.reference(id, 0x3c, r);
    if missing {
        return Err(format!("Could not find audio data: {text}"));
    }
    Ok(())
}
///42CC30 assigns CString before its required nonempty test.
fn change_aspect<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "aspect_ids")?;
    let empty = text.is_empty();
    d.nodes[id.0].texts.insert(0x3c, text);
    if empty {
        return Err("CHANGE_ASPECT requires nonempty aspect_ids".into());
    }
    Ok(())
}
///42CC80: empty is an actual null store; missing positive texture warns.
fn change_skin<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "skin")?;
    let r = if text.is_empty() {
        None
    } else {
        input.texture_reference(s, &text)?
    };
    let missing = !text.is_empty() && r.is_none();
    d.nodes[id.0].textures.insert(0x3c, r);
    if missing {
        d.warnings.push(id);
    }
    Ok(())
}
///42CD60 requires the same newly found Texture before policy/colour reads.
fn create_rfx<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "radar_icon")?;
    if text.is_empty() {
        return Err("No radar_icon defined for CreateRFX".into());
    }
    let r = input.texture_reference(s, &text)?;
    d.nodes[id.0].textures.insert(0x3c, r);
    let texture = d.nodes[id.0]
        .textures
        .get(&0x3c)
        .and_then(Option::as_ref)
        .ok_or("CreateRFX5A6D10 requires its actual nonnull Texture allocation")?;
    input.radar_texture_policy(texture)?;
    for (name, offset) in [("src_color", 0x40), ("dst_color", 0x44)] {
        let word = input.color_490e50(s, name, 0xff)?;
        d.store(id, offset, word);
    }
    //490E90 divisor1/remainder0 is always satisfied, including signedbits.
    let count = input.integer(s, "blink_count")?;
    d.store(id, 0x48, count as u32);
    let at = input.text(s, "at")?;
    let at = input.enum_value("AT", &at)?;
    d.store(id, 0x4c, at as u32);
    if at < 0 {
        return Err("Unknown AT value for CreateRFX".into());
    }
    Ok(())
}
///42CF40 stores pivot before null validation, then rawPoint and six Fixeds.
fn hit_effect<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let pivot = input.text(s, "pivot")?;
    let r = input.find("Bone", &pivot)?;
    let missing = r.is_none();
    d.reference(id, 0x3c, r);
    if missing {
        return Err(format!("Pivot '{pivot}' unrecognized"));
    }
    let axis = input.point(s, "axis")?;
    if axis.iter().any(|v| !f32::from_bits(*v).is_finite()) {
        return Err("HitEffect finite axis domain unproved".into());
    }
    for (i, value) in axis.into_iter().enumerate() {
        d.store(id, 0x40 + 4 * i as u32, value);
    }
    for (name, offset) in [
        ("max_angle", 0x4c),
        ("max_impulse", 0x50),
        ("max_damage", 0x54),
        ("vel_decay", 0x58),
        ("vel_falloff", 0x5c),
        ("stiffness", 0x60),
    ] {
        let value = input.fixed(s, name)?;
        if !f32::from_bits(value).is_finite() {
            return Err(format!("HitEffect {name} finite fixed domain unproved"));
        }
        d.store(id, offset, value);
    }
    Ok(())
}
///490D30→490E50: all range checks precede three x87*255/truncate reads.
pub(super) fn color_490e50(point: [u32; 3], alpha: u8, pc: Precision) -> Result<u32> {
    for bits in point {
        let v = f32::from_bits(bits);
        if !v.is_finite() || !(0.0..=1.0).contains(&v) {
            return Err("native color channel must be finite0..1".into());
        }
    }
    let mut bytes = [0u8; 4];
    bytes[3] = alpha;
    for (i, bits) in point.into_iter().enumerate() {
        //490E50 swaps the inner RGB byte order: DWORD is nativeAARRGGBB.
        bytes[2 - i] = Binary::from_f32_bits(bits)
            .and_then(|v| v.mul(&Binary::from_f32_bits(0x437f0000)?, pc))
            .and_then(|v| v.truncate_i32())
            .map_err(|e| e.to_string())? as u8;
    }
    Ok(u32::from_le_bytes(bytes))
}
#[cfg(test)]
#[path = "fx_graph_subtypes_tests.rs"]
mod tests;
