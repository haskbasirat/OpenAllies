//! Native86BA10 projection mode2 and its unconditional transformed-plane tail.
//!
//! The caller lends the same TextureEffect numeric fields. This helper owns no
//! source texture, native pointer, effect allocation or view completion state.
//! Other modes and nonfinite arithmetic remain explicit required boundaries.
use aa_numeric::{Binary, Precision};

type Result<T> = std::result::Result<T, String>;

/// A direct loan of the actual effect fields, preserving reached reads and
/// prior writes if a required field or finite arithmetic operation refuses.
pub(crate) trait Fields {
    fn word(&mut self, offset: u32) -> Result<u32>;
    fn put(&mut self, offset: u32, word: u32);
}

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| e.to_string())
}
fn read(fields: &mut impl Fields, offset: u32) -> Result<Binary> {
    b(fields.word(offset)?)
}
fn mul(a: Binary, c: Binary, pc: Precision) -> Result<Binary> {
    a.mul(&c, pc).map_err(|e| e.to_string())
}
fn add(a: Binary, c: Binary, pc: Precision) -> Result<Binary> {
    a.add(&c, pc).map_err(|e| e.to_string())
}
fn f(value: Binary) -> Result<u32> {
    value.store_f32().map_err(|e| e.to_string())
}
fn product(fields: &mut impl Fields, a: u32, c: u32, pc: Precision) -> Result<Binary> {
    let a = read(fields, a)?;
    mul(a, read(fields, c)?, pc)
}

/// Executes only the native mode2 path, including86BBB7..86BC25. All current
/// numeric loads are lazy; scratch vector components have the native binary32
/// spills. The final distance keeps distance*scale and each addition extended
/// until its sole store. Rust trap/exception semantics are outside this finite
/// successful-arithmetic domain; masked x87 rounding precision is explicit.
pub(crate) fn update_mode2_86ba10(fields: &mut impl Fields, pc: Precision) -> Result<()> {
    if fields.word(0x164)? != 2 {
        return Err("TextureEffect86BA10 requires actual projection mode2".into());
    }
    fields.put(0x14c, 0x3f000000);
    fields.put(0x148, 0x3f000000);
    fields.put(0x150, 0);
    for (a, c, zero, positive, negative) in [
        (0x70, 0x74, 0x13c, 0x124, 0x130),
        (0x7c, 0x80, 0x140, 0x128, 0x134),
        (0x88, 0x8c, 0x144, 0x12c, 0x138),
    ] {
        let a = read(fields, a)?;
        let c = read(fields, c)?;
        fields.put(zero, 0);
        fields.put(positive, f(mul(c, b(0x3f000000)?, pc)?)?);
        fields.put(negative, f(mul(a, b(0xbf000000)?, pc)?)?);
    }

    //870720 evaluates Z, Y, X, with asymmetric term order, then stores X/Y/Z
    //to scratch. Repeated plane loads remain repeated current field reads.
    let z = add(
        add(product(fields, 0x8c, 0x174, pc)?, product(fields, 0x84, 0x16c, pc)?, pc)?,
        product(fields, 0x88, 0x170, pc)?, pc,
    )?;
    let y = add(
        add(product(fields, 0x80, 0x174, pc)?, product(fields, 0x78, 0x16c, pc)?, pc)?,
        product(fields, 0x7c, 0x170, pc)?, pc,
    )?;
    let x = add(
        add(product(fields, 0x74, 0x174, pc)?, product(fields, 0x70, 0x170, pc)?, pc)?,
        //The final native FLD is plane.x, followed by FMUL rotation00.
        product(fields, 0x16c, 0x6c, pc)?, pc,
    )?;
    let x = f(x)?;
    let y = f(y)?;
    let z = f(z)?;
    let distance = product(fields, 0x178, 0x9c, pc)?;
    let y_product = mul(b(y)?, read(fields, 0x94)?, pc)?;
    let distance = add(distance, y_product, pc)?;
    let z_product = mul(b(z)?, read(fields, 0x98)?, pc)?;
    let distance = add(distance, z_product, pc)?;
    let x_product = mul(b(x)?, read(fields, 0x90)?, pc)?;
    fields.put(0x17c, x);
    fields.put(0x180, y);
    let counter = fields.word(0xd0)?;
    let distance = add(distance, x_product, pc)?;
    fields.put(0xd0, counter.wrapping_add(1));
    fields.put(0x188, f(distance)?);
    fields.put(0x184, z);
    Ok(())
}

#[cfg(test)]
#[path = "view_environment_projection_tests.rs"]
mod tests;
