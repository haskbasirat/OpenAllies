//! Saved actor commands: constructor 4D9C80 and virtual+8 Load 4DACE0.
//!
//! This is the byte-oriented saved-map format, not the bit-oriented network
//! command format (61EBA0). References are serialized CString registry names;
//! the restoration owner must resolve them before creating a live command.
use crate::saved_load::{Error, Reader, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub version: u32,
    /// +08, KKC_ActorCommandData, 4DAB60 (manager B3AA58).
    pub definition: Option<Vec<u16>>,
    pub flags: u32,
    pub word10: u32,
    /// +14, KKC_ActorData, 4C2E60 (4BDBA0 category 0x2B).
    pub object_definition: Option<Vec<u16>>,
    pub position_bits: [u32; 2],
    pub auxiliary_bits: [u32; 2],
    pub scalar_bits: u32,
    /// +2C, KKC_FormationData, 4DAC20 (manager B3AB08), version >= 1.
    pub parameter_definition: Option<Vec<u16>>,
    /// +38, KKC_SharedPropertyData, 4DAAA0 (manager B3AA2C), version >= 2.
    pub extra_definition: Option<Vec<u16>>,
    /// Native stores a byte without normalization. Absent before version 3.
    pub boolean: u8,
    /// +3C, always read, regardless of the command flags.
    pub trailing: u8,
}

fn reference(r: &mut Reader<'_>) -> Result<Option<Vec<u16>>> {
    let units = r.units()?;
    // The native readers test CString length, not its first code unit. A
    // nonempty string beginning in NUL still invokes registry lookup.
    Ok((!units.is_empty()).then_some(units))
}

pub(crate) fn read(r: &mut Reader<'_>) -> Result<Command> {
    let version = r.version(3)?;
    let definition = reference(r)?;
    let flags = r.u32()?;
    let word10 = r.u32()?;
    let object_definition = reference(r)?;
    let position_bits = [r.u32()?, r.u32()?];
    let auxiliary_bits = [r.u32()?, r.u32()?];
    let scalar_bits = r.u32()?;
    let trailing = r.u8()?;
    let boolean = if version >= 3 { r.u8()? } else { 0 };
    let parameter_definition = if version >= 1 { reference(r)? } else { None };
    if flags & (1 << 5) != 0 {
        return Err(Error::Unsupported(
            "saved command ParameterSet 4C2110/virtual+8 Load (flag bit 5)",
        ));
    }
    let extra_definition = if version >= 2 { reference(r)? } else { None };
    Ok(Command {
        version,
        definition,
        flags,
        word10,
        object_definition,
        position_bits,
        auxiliary_bits,
        scalar_bits,
        parameter_definition,
        extra_definition,
        boolean,
        trailing,
    })
}

/// Decode one original saved command, retaining unresolved source references.
/// No registry name is substituted, and unsupported owned payloads fail.
pub fn decode(bytes: &[u8]) -> Result<(Command, usize)> {
    let mut reader = Reader { bytes, at: 0 };
    let command = read(&mut reader)?;
    Ok((command, reader.at))
}
