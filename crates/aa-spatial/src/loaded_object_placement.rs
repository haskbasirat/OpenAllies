//! Original `4F4590`: temporary placement descriptor for an existing object.
//!
//! Saved `4EF6A0(NULL)` supplies no position/facing override and two zero words.
//! This wrapper is not `4F3C40`: the complete `4F3640` coordinator is a required
//! host operation. Original/v14 evidence is under
//! `research/skirmish-20261016-loaded-placement/`. SDK code is not used.
use crate::Result;
use aa_numeric::Precision;

/// Original descriptor class, metadata only; never dereferenced by Rust.
pub const NATIVE_DESCRIPTOR_VTABLE: u32 = 0x00a2_5b28;

/// Defined fields of the original 0x54-byte stack descriptor. The uninitialized
/// byte +53 is deliberately absent. Handles are portable host identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor<D, N> {
    pub definition: D,
    pub position_bits: [u32; 3],
    pub facing_bits: [u32; 2],
    /// Original +1C. The coordinator may replace it before cleanup.
    pub temporary_name: N,
    /// Original dwords +20..+4C. +28 receives virtual+108 when +B4 is true.
    pub words_20_to_4c: [u32; 12],
    /// Defined bytes +50..+52; +52 is !Thing.flags.bit10 after the virtuals.
    pub bytes_50_to_52: [u8; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Request<O, V> {
    pub object: O,
    pub position: Option<V>,
    pub facing: Option<V>,
    pub argument_4: u32,
    pub argument_5: u32,
}

/// The remaining six of 4F3640's seven stack arguments, after the descriptor.
/// The world is its `this`; the object is passed twice, then two zero words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoordinatorArguments<O> {
    pub source: O,
    pub other: O,
    pub zero_4: u32,
    pub zero_5: u32,
    pub argument_6: u32,
    pub argument_7: u32,
}

/// Required current-storage access and owned services. There is no default
/// success, synthetic definition, permissive collision, or temporary-name owner.
/// Word getters perform no external callbacks; each reads only its current word.
pub trait World {
    type Object: Copy;
    type Vector: Copy;
    type Definition: Copy;
    type Name: Copy;
    /// 856370(2), allocator virtual+C, then the descriptor's empty CString.
    fn empty_temporary_name(&mut self) -> Result<Self::Name>;
    fn definition(&mut self, object: Self::Object) -> Result<Self::Definition>;
    fn object_word(&mut self, object: Self::Object, offset: u32) -> Result<u32>;
    fn vector_word(&mut self, vector: Self::Vector, offset: u32) -> Result<u32>;
    fn virtual_b4(&mut self, object: Self::Object) -> Result<bool>;
    fn virtual_108(&mut self, object: Self::Object) -> Result<u32>;
    fn coordinator_4f3640(
        &mut self,
        descriptor: &mut Descriptor<Self::Definition, Self::Name>,
        arguments: CoordinatorArguments<Self::Object>,
        precision: Precision,
    ) -> Result<u32>;
    /// Normal-return CString release: the CURRENT descriptor+1C, including its
    /// refcount decrement and allocator release when required. Native exception
    /// unwinding is outside this wrapper's error-prefix contract.
    fn release_temporary_name(&mut self, name: Self::Name) -> Result<()>;
}

/// Execute 4F4590, preserving bit patterns and read/callback order. The wrapper
/// does no FP arithmetic. Callback errors stop after the retained prefix;
/// native SEH/C++ unwinding and allocation failure are not emulated here.
pub fn check<W: World>(
    request: Request<W::Object, W::Vector>,
    precision: Precision,
    world: &mut W,
) -> Result<u32> {
    let temporary_name = world.empty_temporary_name()?;
    let definition = world.definition(request.object)?;
    // Original copies facing first, then all three position words, before +B4.
    let mut facing_bits = [0; 2];
    for (index, value) in facing_bits.iter_mut().enumerate() {
        let offset = index as u32 * 4;
        *value = match request.facing {
            Some(vector) => world.vector_word(vector, offset)?,
            None => world.object_word(request.object, 0x2c + offset)?,
        };
    }
    let mut position_bits = [0; 3];
    for (index, value) in position_bits.iter_mut().enumerate() {
        let offset = index as u32 * 4;
        *value = match request.position {
            Some(vector) => world.vector_word(vector, offset)?,
            None => world.object_word(request.object, 0x20 + offset)?,
        };
    }
    let word_28 = if world.virtual_b4(request.object)? {
        world.virtual_108(request.object)?
    } else {
        0
    };
    let flags = world.object_word(request.object, 8)?;
    let mut descriptor = Descriptor {
        definition,
        position_bits,
        facing_bits,
        temporary_name,
        words_20_to_4c: [u32::MAX, 0, word_28, 0, 0, u32::MAX, 0, 0, 0, 0, 0, 0],
        bytes_50_to_52: [0, 0, ((!flags >> 10) & 1) as u8],
    };
    let result = world.coordinator_4f3640(
        &mut descriptor,
        CoordinatorArguments {
            source: request.object,
            other: request.object,
            zero_4: 0,
            zero_5: 0,
            argument_6: request.argument_4,
            argument_7: request.argument_5,
        },
        precision,
    )?;
    world.release_temporary_name(descriptor.temporary_name)?;
    Ok(result)
}

/// The exact caller input from saved-object 4EF6A0(NULL).
pub fn check_saved<W: World>(
    object: W::Object,
    precision: Precision,
    world: &mut W,
) -> Result<u32> {
    check(
        Request {
            object,
            position: None,
            facing: None,
            argument_4: 0,
            argument_5: 0,
        },
        precision,
        world,
    )
}
