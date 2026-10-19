//! Regiment formations: slot positions around the anchor (the original
//! layouts and aa_movement::formation). Owner: movement.
use glam::Vec2;

/// World position of slot `offset` for an organization at `anchor` facing `heading`.
pub fn slot_position(anchor: Vec2, heading: f32, offset: [f32; 2]) -> Vec2 {
    let (s, c) = heading.sin_cos();
    anchor + Vec2::new(offset[0] * c - offset[1] * s, offset[0] * s + offset[1] * c)
}
