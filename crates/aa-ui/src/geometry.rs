//! Code-side widget geometry (the setters the original's dialogs use to
//! size and place widgets at run time). All values are in interface
//! reference units (1024 x 768 space); each setter rewrites the widget's
//! root bounds and the original then re-lays it out (`0x5E1960`).
//!
//! * `0x5EF060` SetSize(w, h): R = w, B = h as sizes (flags & 0xFFFFFF33 |
//!   0x300).
//! * `0x5EEF10` SetPosition(x, y): L = x (flags & 0xFFFFFFEE), R = current
//!   width / sx (flags & 0xFFFFFF99 | 0x100), T = y, B = current height /
//!   sy (flags & 0xFFFFFF77 | 0x200).
//! * `0x5EEFA0` SetLeft(x), `0x5EF000` SetTop(y) keep the current size.
//! * `0x5EF0A0` SetWidth(w), `0x5EF0D0` SetHeight(h).
//! * `0x5EE420` Width() = (R - L) / sx, `0x5EE440` Height() = (B - T) / sy.

use crate::layout::{Precision, Reference, Screen, ui_scale};
use crate::widget::Widget;

fn scale(screen: Screen, reference: Reference, p: Precision) -> (f32, f32) {
    ui_scale(screen, reference, p)
}

/// `0x5EE420`.
pub fn width(w: &Widget, screen: Screen, reference: Reference, p: Precision) -> f32 {
    let r = w.root.outer.rect;
    (r.r - r.l) / scale(screen, reference, p).0
}

/// `0x5EE440`.
pub fn height(w: &Widget, screen: Screen, reference: Reference, p: Precision) -> f32 {
    let r = w.root.outer.rect;
    (r.b - r.t) / scale(screen, reference, p).1
}

/// `0x5EF060`.
pub fn set_size(w: &mut Widget, width: f32, height: f32) {
    let b = &mut w.root.outer;
    b.raw[2] = width;
    b.raw[3] = height;
    b.flags = (b.flags & 0xFFFF_FF33) | 0x300;
}

/// `0x5EEF10` (needs the widget laid out).
pub fn set_position(w: &mut Widget, x: f32, y: f32, screen: Screen, reference: Reference, p: Precision) {
    let (sx, sy) = scale(screen, reference, p);
    let r = w.root.outer.rect;
    let b = &mut w.root.outer;
    b.raw[0] = x;
    b.flags &= 0xFFFF_FFEE;
    b.raw[1] = y;
    b.flags = (b.flags & 0xFFFF_FF99) | 0x100;
    b.raw[2] = (r.r - r.l) / sx;
    b.flags = (b.flags & 0xFFFF_FF77) | 0x200;
    b.raw[3] = (r.b - r.t) / sy;
}

/// `0x5EEFA0`.
pub fn set_left(w: &mut Widget, x: f32, screen: Screen, reference: Reference, p: Precision) {
    let (sx, _) = scale(screen, reference, p);
    let r = w.root.outer.rect;
    let b = &mut w.root.outer;
    b.raw[0] = x;
    b.flags &= 0xFFFF_FFEE;
    b.flags = (b.flags & 0xFFFF_FFBB) | 0x100;
    b.raw[2] = (r.r - r.l) / sx;
}

/// `0x5EF000`.
pub fn set_top(w: &mut Widget, y: f32, screen: Screen, reference: Reference, p: Precision) {
    let (_, sy) = scale(screen, reference, p);
    let r = w.root.outer.rect;
    let b = &mut w.root.outer;
    b.raw[1] = y;
    b.flags &= 0xFFFF_FFDD;
    b.flags = (b.flags & 0xFFFF_FF77) | 0x200;
    b.raw[3] = (r.b - r.t) / sy;
}

/// `0x5EF0A0`.
pub fn set_width(w: &mut Widget, width: f32) {
    let b = &mut w.root.outer;
    b.raw[2] = width;
    b.flags = (b.flags & 0xFFFF_FFBB) | 0x100;
}

/// `0x5EF0D0`.
pub fn set_height(w: &mut Widget, height: f32) {
    let b = &mut w.root.outer;
    b.raw[3] = height;
    b.flags = (b.flags & 0xFFFF_FF77) | 0x200;
}
