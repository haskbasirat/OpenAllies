//! Actual Mind+114/+118 leaves reached by57CBA0/579A00.
//! Embedded status lives only in Arena; other words live in Installed.
use aa_mind::construct::Kind;
type Result<T> = std::result::Result<T, String>;
pub(crate) trait Fields {
    fn embedded_status(&mut self, offset: u32) -> Result<u32>;
    fn move_speed_changed(&mut self, old: u32, new: u32) -> Result<()>;
    fn put_word(&mut self, offset: u32, word: u32) -> Result<()>;
}
pub(crate) fn speed_changed(
    kind: Kind,
    fields: &mut impl Fields,
    old: u32,
    new: u32,
) -> Result<()> {
    let offset = match kind {
        Kind::Regiment => 0x68,
        Kind::Element => 0x7c,
        Kind::Base | Kind::Aircraft => return Ok(()),
    };
    if fields.embedded_status(offset)? == 2 {
        fields.move_speed_changed(old, new)?;
    }
    Ok(())
}
pub(crate) fn leader_changed(kind: Kind, fields: &mut impl Fields) -> Result<()> {
    if kind == Kind::Regiment {
        fields.put_word(0x50, 0)?;
        fields.put_word(0xb8, 0)?;
    }
    Ok(())
}
pub(crate) fn position_active(kind: Kind, slot: u32, fields: &mut impl Fields) -> Result<bool> {
    let state = match (kind, slot) {
        (Kind::Regiment, 0x64) => 0x68,
        (Kind::Regiment, 0x7c) => 0x280,
        (Kind::Element, 0x64) => 0x7c,
        (Kind::Element, 0x7c) => 0x188,
        (Kind::Aircraft, 0x64) => 0x34,
        (Kind::Base | Kind::Aircraft, 0x7c) => return Ok(false),
        (Kind::Base, 0x64) => return Err("native Base Mind+64 purecall96CAC2".into()),
        _ => return Err("unexpected native Mind position predicate".into()),
    };
    Ok(fields.embedded_status(state)? == 2)
}
#[cfg(test)]
#[path = "organization_mind_callbacks_tests.rs"]
mod tests;
