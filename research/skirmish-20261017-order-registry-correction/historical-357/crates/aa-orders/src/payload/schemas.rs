//! Descriptor ordinal -> payload schema. Request and decree descriptors of one
//! native order class share a schema; the table covers all 357 descriptors.
use super::{Schema, complex, editor, game, lobby};
use crate::{Error, Result};

/// Number of native descriptors (0x96EC76 CRT registration, phase4 evidence).
pub const SCHEMAS: usize = 357;

pub fn schema(descriptor: usize) -> Result<Schema> {
    complex::schema(descriptor)
        .or_else(|| game::schema(descriptor))
        .or_else(|| lobby::schema(descriptor))
        .or_else(|| editor::schema(descriptor))
        .ok_or(Error::UnsupportedNativeDescriptor(descriptor))
}
