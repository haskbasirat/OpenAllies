//! Original Kingdom `+2C4` list removal `4E16F0` (thiscall Kingdom,
//! `(actor)`, `ret 4`; called by `589280`).
//!
//! The array `+2C4` (count `+2C8`, signed) is searched from index 0 for the
//! first equal word. The count is decremented unconditionally and the words
//! after the found index shift down by one. When the word is absent the index
//! is -1: the shift then starts below the array, so the word at index -1
//! receives the old element 0 and element 0 is dropped. With an empty array
//! the count becomes -1 and nothing is written.

/// What `4E16F0` did besides shortening `list`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Removal {
    /// The word was at this index and was removed.
    Removed(usize),
    /// Absent: element 0 was dropped and written to the word below the
    /// array (`+2C4` storage index -1).
    DroppedFirst { below: u32 },
    /// Absent from an empty array: the count `+2C8` is now -1.
    EmptyUnderflow,
}

/// `4E16F0` over the `+2C4` words in order.
pub fn remove_structure(list: &mut Vec<u32>, value: u32) -> Removal {
    match list.iter().position(|&word| word == value) {
        Some(index) => {
            list.remove(index);
            Removal::Removed(index)
        }
        None if list.is_empty() => Removal::EmptyUnderflow,
        None => Removal::DroppedFirst { below: list.remove(0) },
    }
}
