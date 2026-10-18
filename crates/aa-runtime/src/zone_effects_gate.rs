//! Exact54F7F0/54EF10 prefix; both getters borrow their live field owners.
pub(crate) fn enabled<E>(
    session_e4: impl FnOnce() -> Result<u32, E>,
    world48: impl FnOnce() -> Result<u32, E>,
) -> Result<bool, E> {
    if session_e4()? != 3 {
        return Ok(true);
    }
    Ok(world48()? != 0)
}
