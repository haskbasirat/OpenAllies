//! Original `4F14A0(direction)` on a tile cursor: the address of the
//! neighbouring tile record in the cursor's grid.
//!
//! The cursor holds the tile x (+0), y (+4) and its grid (+18), whose +1C is
//! the row width and +28 the base of its 0x30-byte tile records. The
//! direction indexes the `[B322E0]` (dx, dy) table; the address is
//! `((y + dy) * width + x + dx) * 0x30 + base` in 32-bit wrapping
//! arithmetic.

/// `0xB322E0`: (dx, dy) per direction 0..=8 (read from AA.exe).
pub const NEIGHBOUR_DELTAS: [[i32; 2]; 9] = [
    [0, 0],
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
    [1, 0],
    [1, -1],
    [0, -1],
];
/// Size of one grid tile record.
pub const TILE_RECORD_SIZE: u32 = 0x30;

/// Complete `4F14A0(direction)` for a cursor at (`x`, `y`) on a grid of row
/// `width` and record `base`; `None` for a direction outside the table.
pub fn neighbour_address(x: i32, y: i32, width: i32, base: u32, direction: usize) -> Option<u32> {
    let [dx, dy] = *NEIGHBOUR_DELTAS.get(direction)?;
    let index = width
        .wrapping_mul(y.wrapping_add(dy))
        .wrapping_add(x)
        .wrapping_add(dx);
    Some((index as u32).wrapping_mul(TILE_RECORD_SIZE).wrapping_add(base))
}
