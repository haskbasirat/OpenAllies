//! Exact x87 helpers shared by the input ports. Values that the native code
//! keeps on the FPU stack are [`Binary`] at the caller's precision control;
//! every `fstp dword` is [`st`]. Non-finite inputs are outside the finite
//! arithmetic domain and are errors, never silently replaced.
use aa_numeric::{Binary, Precision};

pub type Result<T> = std::result::Result<T, String>;

pub fn b(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| format!("binary32 {bits:#010x}: {e}"))
}
pub fn st(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| e.to_string())
}
pub fn add(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.add(c, p).map_err(|e| e.to_string())
}
pub fn sub(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.sub(c, p).map_err(|e| e.to_string())
}
pub fn mul(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.mul(c, p).map_err(|e| e.to_string())
}
pub fn div(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.div(c, p).map_err(|e| e.to_string())
}
pub fn sqrt(a: &Binary, p: Precision) -> Result<Binary> {
    a.sqrt(p).map_err(|e| e.to_string())
}
/// Ordered `a < c` (the `fcomp; test ah,5; jnp` / `test ah,0x41` idioms are
/// expressed by their callers in terms of this and [`le`]).
pub fn lt(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == std::cmp::Ordering::Less
}
pub fn le(a: &Binary, c: &Binary) -> bool {
    a.compare(c) != std::cmp::Ordering::Greater
}

/// 0x436480 (and its inline copies in 0x437B20): `fsqrt((dz*dz + dy*dy) +
/// dx*dx)` with `d = a - b` per axis, all in registers. Returns ST0.
pub fn distance_436480(a: [u32; 3], c: [u32; 3], p: Precision) -> Result<Binary> {
    let dx = sub(&b(a[0])?, &b(c[0])?, p)?;
    let dy = sub(&b(a[1])?, &b(c[1])?, p)?;
    let dz = sub(&b(a[2])?, &b(c[2])?, p)?;
    let zz = mul(&dz, &dz, p)?;
    let yy = mul(&dy, &dy, p)?;
    let xx = mul(&dx, &dx, p)?;
    sqrt(&add(&add(&zz, &yy, p)?, &xx, p)?, p)
}
