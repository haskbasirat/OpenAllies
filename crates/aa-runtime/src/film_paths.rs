//! Native CString path helpers reached by7757F0 film startup.
//! They operate on counted UTF16 words, including embedded NULs; these are
//! string operations only, with no filesystem normalization or sandbox policy.

///85AB30: append a dot and the entire nonempty extension, even when the
/// current name already contains an extension. An empty extension does nothing.
pub fn append_extension(name: &mut Vec<u16>, extension: &[u16]) {
    if !extension.is_empty() {
        name.push(u16::from(b'.'));
        name.extend_from_slice(extension);
    }
}

///85ABB0/85A1A0: counted concatenation. When the right side starts with a
/// separator, remove exactly ONE trailing separator from the left side. This
/// does not implement absolute-path replacement or collapse repeated slashes.
pub fn join(left: &[u16], right: &[u16]) -> Vec<u16> {
    let separator = |word: u16| word == u16::from(b'/') || word == u16::from(b'\\');
    let mut result = left.to_vec();
    if right.first().is_some_and(|&word| separator(word)) {
        if result.last().is_some_and(|&word| separator(word)) {
            result.pop();
        }
    } else if result.last().is_some_and(|&word| !separator(word)) {
        result.push(u16::from(b'/'));
    }
    result.extend_from_slice(right);
    result
}
