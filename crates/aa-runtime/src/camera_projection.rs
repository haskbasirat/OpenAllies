//! Bounded original-AA projection producer `892B7A..892D5E`.
//!
//! This is not complete SetCameraData: the view/basis setup, device lifecycle,
//! actual virtual `+54`, D3D SetTransform, culling and client binding are absent.
//! Inputs describe a stable read-only frustum, not arbitrarily reentrant native
//! pointers. Finite supported arithmetic and explicit x87 PC only; overflowing
//! binary32 temporary stores/reloads are outside that domain. Each native
//! store commits immediately; errors retain its already-written prefix.
use aa_numeric::{Binary, Precision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frustum {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub near: u32,
    pub far: u32,
    pub orthographic: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("projection service: {0}")]
    Service(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Required services/storage of this bounded producer, never success defaults.
pub trait World {
    /// The actual renderer helper's virtual `+54` result. No default or guessed
    /// engine-wide meaning is implied; it selects the native horizontal branch.
    fn renderer_flag_54(&mut self) -> Result<bool>;
    fn store_word(&mut self, renderer_offset: u32, bits: u32) -> Result<()>;
    /// Native rereads of matrix `+708` and `+718`, in their original order.
    fn load_word(&mut self, renderer_offset: u32) -> Result<u32>;
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn temporary(value: Binary) -> Result<Binary> {
    b(value.store_f32()?)
}
fn put(world: &mut impl World, offset: u32, value: &Binary) -> Result<()> {
    world.store_word(offset, value.store_f32()?)
}

/// AA `892B7A..892D5E`, stopping before the actual D3D projection submission.
/// Matrix words are `+6E0..+71C` in native row order. Perspective has M34=1,
/// M44=0 and depth far/(far-near), -far*near/(far-near); orthographic has
/// M34=0, M44=1 and depth 1/(far-near), -near/(far-near). These are the AA/D3D
/// conventions, not the inspection camera's right-handed wgpu projection.
pub fn build(frustum: &Frustum, pc: Precision, world: &mut impl World) -> Result<()> {
    world.store_word(0x34c, frustum.near)?;
    let span = b(frustum.far)?.sub(&b(frustum.near)?, pc)?;
    put(world, 0x350, &span)?;
    // The span remains on the x87 stack after its binary32 auxiliary store.
    let width = temporary(b(frustum.right)?.sub(&b(frustum.left)?, pc)?)?;
    let horizontal_sum = temporary(b(frustum.left)?.add(&b(frustum.right)?, pc)?)?;
    let height = temporary(b(frustum.top)?.sub(&b(frustum.bottom)?, pc)?)?;
    let vertical_sum = temporary(b(frustum.top)?.add(&b(frustum.bottom)?, pc)?)?;
    // Original AA constants at 9F32E0 (1) and A0083C (-2).
    let one = b(0x3f80_0000)?;
    let inverse_span = temporary(one.div(&span, pc)?)?;
    let flag = world.renderer_flag_54()?;
    let inverse_width = one.div(&width, pc)?;
    world.store_word(if frustum.orthographic { 0x700 } else { 0x710 }, 0)?;
    world.store_word(0x6f0, 0)?;
    let horizontal = if flag {
        b(0xc000_0000)?.mul(&inverse_width, pc)?
    } else {
        inverse_width.add(&inverse_width, pc)?
    };
    put(world, 0x6e0, &horizontal)?;
    let horizontal_translation = inverse_width.mul(&horizontal_sum, pc)?;
    let horizontal_translation = if flag {
        horizontal_translation
    } else {
        horizontal_translation.negated()
    };
    put(
        world,
        if frustum.orthographic { 0x710 } else { 0x700 },
        &horizontal_translation,
    )?;
    if frustum.orthographic {
        world.store_word(0x6e4, 0)?;
        let inverse_height = one.div(&height, pc)?;
        world.store_word(0x704, 0)?;
        world.store_word(0x6e8, 0)?;
        world.store_word(0x6f8, 0)?;
        put(world, 0x708, &inverse_span)?;
        put(world, 0x6f4, &inverse_height.add(&inverse_height, pc)?)?;
        put(
            world,
            0x714,
            &inverse_height.mul(&vertical_sum, pc)?.negated(),
        )?;
        let translation = inverse_span.mul(&b(frustum.near)?, pc)?;
        world.store_word(0x70c, 0)?;
        world.store_word(0x71c, 0x3f80_0000)?;
        finish(world, translation)
    } else {
        world.store_word(0x6e4, 0)?;
        world.store_word(0x714, 0)?;
        let inverse_height = one.div(&height, pc)?;
        world.store_word(0x6e8, 0)?;
        world.store_word(0x6f8, 0)?;
        put(world, 0x6f4, &inverse_height.add(&inverse_height, pc)?)?;
        put(
            world,
            0x704,
            &inverse_height.mul(&vertical_sum, pc)?.negated(),
        )?;
        put(world, 0x708, &inverse_span.mul(&b(frustum.far)?, pc)?)?;
        let translation = b(frustum.far)?.mul(&b(frustum.near)?, pc)?;
        world.store_word(0x70c, 0x3f80_0000)?;
        world.store_word(0x71c, 0)?;
        finish(world, translation.mul(&inverse_span, pc)?)
    }
}

fn finish(world: &mut impl World, translation: Binary) -> Result<()> {
    let scale = world.load_word(0x708)?;
    put(world, 0x718, &translation.negated())?;
    let translation = world.load_word(0x718)?;
    world.store_word(0x3fc, scale)?;
    world.store_word(0x6ec, 0)?;
    world.store_word(0x6fc, 0)?;
    world.store_word(0x40c, translation)
}
