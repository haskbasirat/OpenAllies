type Result<T> = std::result::Result<T,String>;
mod load_creation_fx {use super::*;
pub(crate) fn require_source_view_attachment(word_c: u32) -> Result<()> {
    if word_c == 0 {
        return Ok(());
    }
    Err(
        "Existing FX nonnull Thing+0C requires actual View+54/virtualBC attachment and45C740 owner"
            .into(),
    )
}
}
#[test]
fn existing_fx_attachment_uses_current_thing_view_without_data_flag_shortcuts() {
    assert!(crate::load_creation_fx::require_source_view_attachment(0).is_ok());
    for current_view in [1, 0x12345678, u32::MAX] {
        assert!(
            crate::load_creation_fx::require_source_view_attachment(current_view)
                .unwrap_err()
                .contains("View+54/virtualBC")
        );
    }
}

