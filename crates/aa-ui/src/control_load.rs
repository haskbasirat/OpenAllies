//! `LoadData` of the control classes (data part; layout and activation
//! live in `ui_controls.rs`).
//!
//! * Slider `0x5DE800`: `sound_clicked`, `usage_area_pcg`,
//!   `usage_area_absolute`, `min_value`, `max_value`, `step_size`,
//!   `page_size`, `grip_display_size`, the `Background` view element
//!   (`0x5E4B40` into the slider's root group), then `SetValue(min, snap)`.
//! * Advanced slider `0x5DE940`: `show_minmax` (Min/Max labels visible),
//!   `show_current_val` (CurrentValue label visible); both false is a
//!   warning.
//! * List box `0x5D58E0`: the node clone (`+0x98`), `num_visible_items`,
//!   `item_height` ("Bad values" when either is below 1), `sorted`,
//!   `vertical_item_offset`, `multiselect`, `ignore_redundant_select`,
//!   `resize_to_fit`, the `ItemEntry` list; the scroll bar slider gets
//!   `SetMin(0)`, step 1 and page `num_visible_items`; resize_to_fit sets
//!   the height.
//! * Drop-down `0x5D6200`: `dropup`, `sound_open`, `sound_close`; max
//!   visible = the list box's `num_visible_items`.
//! * Edit box `0x5D23E0`: `max_characters`, `text`, `integer_only`,
//!   `integer_max`.

use crate::controls::{self as k, Control, ListEntry};
use crate::node::UiNode;
use crate::state::widget_flag;
use crate::ve::ViewElement;
use crate::widget::{Widget, WidgetClass as C};

/// Class data read before the children load.
pub fn load_control(w: &mut Widget, node: &UiNode, strings: &dyn Fn(&str) -> String) {
    match w.class {
        C::Slider | C::SliderAdvanced => {
            let mut diags = Vec::new();
            let wname = w.name.clone();
            let bg = node.child("Background").map(|b| ViewElement::load_localized(b, strings));
            if let Some(s) = w.slider_mut() {
                s.sound_clicked = node.string_or_empty("sound_clicked");
                let p2 = |k: &str, d: [f32; 2]| node.point2(k).unwrap_or(d);
                s.usage_pcg = p2("usage_area_pcg", s.usage_pcg);
                s.usage_abs = p2("usage_area_absolute", s.usage_abs);
                let f = |k: &str, d: f32| node.float(k).unwrap_or(d);
                s.min = f("min_value", s.min);
                s.max = f("max_value", s.max);
                s.step = f("step_size", s.step);
                s.page = f("page_size", s.page);
                s.grip_size = f("grip_display_size", s.grip_size);
                if let Some(adv) = &mut s.advanced {
                    adv.0 = node.bool_or("show_minmax", true).unwrap_or(true);
                    adv.1 = node.bool_or("show_current_val", true).unwrap_or(true);
                    if !adv.0 && !adv.1 {
                        diags.push(format!("Slider '{}' shows neither its bounds nor its value", wname));
                    }
                }
                // SetValue(min, snap) on the unconnected slider
                let v = k::snap_clamp(s, s.min, true);
                if (v - s.value).abs() >= k::EPS {
                    s.value = v;
                }
            }
            match bg {
                Some(Ok(ve)) => w.sub_ve = Some(ve),
                Some(Err(e)) => diags.push(e),
                None => diags.push(format!("SubNode not found: 'background' in '{}'", node.name)),
            }
            if let Some((minmax, cur, _)) = w.slider().and_then(|s| s.advanced) {
                for (name, vis) in [("MinBoundLabel", minmax), ("MaxBoundLabel", minmax), ("CurrentValueLabel", cur)] {
                    if let Some(c) = w.child_mut(name) {
                        c.set_visible(vis);
                    }
                }
            }
            w.diagnostics.extend(diags);
        }
        C::Listbox => {
            let n = node.int("num_visible_items").unwrap_or(0);
            let h = node.float("item_height").unwrap_or(0.0);
            if n < 1 || h < 1.0 {
                w.diagnostics.push(format!("Bad values for list box '{}': num_visible_items {n}, item_height {h}", w.name));
            }
            let sorted = node.bool_or("sorted", true).unwrap_or(true);
            if sorted {
                w.flags |= widget_flag::SORTED;
            } else {
                w.flags &= !widget_flag::SORTED;
            }
            let entries: Vec<ListEntry> = node
                .children_named("ItemEntry")
                .map(|e| ListEntry::new(&strings(&e.string_or_empty("text")), &e.string_or_empty("param")))
                .collect();
            if let Some(lb) = w.listbox_mut() {
                lb.node = Some(node.clone());
                lb.template_visible = n;
                lb.num_visible = n;
                lb.item_height = h;
                lb.vertical_item_offset = node.float("vertical_item_offset").unwrap_or(0.0);
                lb.multiselect = node.bool_or("multiselect", false).unwrap_or(false);
                lb.ignore_redundant_select = node.bool_or("ignore_redundant_select", true).unwrap_or(true);
                lb.resize_to_fit = node.bool_or("resize_to_fit", true).unwrap_or(true);
            }
            for e in entries {
                add_entry(w, e);
            }
        }
        C::DropDown => {
            if let Some(Control::DropDown(d)) = w.control.as_deref_mut() {
                d.dropup = node.bool_or("dropup", false).unwrap_or(false);
                d.sound_open = node.string_or_empty("sound_open");
                d.sound_close = node.string_or_empty("sound_close");
            }
        }
        // MiniMap LoadData 0x5D7230
        C::MiniMap => {
            let mut m = crate::minimap::MiniMap {
                dynamic_position: node.bool_or("dynamic_position", false).unwrap_or(false),
                ..Default::default()
            };
            if let Ok([x, y]) = node.point2("minimap_size") {
                m.size = (x as i32, y as i32);
            }
            match node.child("MiniMapVE") {
                Some(ve) => {
                    if let Ok([x, y]) = ve.point2("orientation") {
                        m.orientation = crate::minimap::normalize(x, y);
                    } else {
                        m.orientation = crate::minimap::normalize(1.0, -1.0);
                    }
                    m.square = ve.bool_or("minimap_is_square", true).unwrap_or(true);
                    match ViewElement::load_localized(ve, strings) {
                        Ok(v) => w.sub_ve = Some(v),
                        Err(e) => w.diagnostics.push(e),
                    }
                }
                None => w.diagnostics.push(format!("SubNode not found: 'MiniMapVE' in '{}'", node.name)),
            }
            w.control = Some(Box::new(Control::MiniMap(m)));
        }
        // MessageLog LoadData 0x66FD70: the ItemPanel node clone, expands_down
        // (bottom anchored unless set)
        C::MessageLog => {
            let item = node.child("ItemPanel").cloned();
            let down = node.bool_or("expands_down", false).unwrap_or(false);
            if let Some(Control::MessageLog(s)) = w.control.as_deref_mut() {
                s.node = item;
                s.expands_down = down;
                s.vp.bottom_anchored = !down;
            }
        }
        C::Editbox => {
            let text = strings(&node.string_or_empty("text"));
            if let Some(e) = w.editbox_mut() {
                e.max_characters = node.int("max_characters").unwrap_or(0);
                e.integer_only = node.bool_or("integer_only", false).unwrap_or(false);
                e.integer_max = node.int("integer_max").unwrap_or(-1);
                e.caret = text.chars().count();
                e.text = text;
            }
        }
        _ => {}
    }
}

/// Class data that needs the loaded children.
pub fn after_children(w: &mut Widget) {
    match w.class {
        C::Listbox => {
            let (n, resize) = match w.listbox() {
                Some(lb) => (lb.template_visible, lb.resize_to_fit),
                None => return,
            };
            if let Some(s) = w.child_mut("Scrollbar").and_then(|sb| sb.child_mut("Slider")).and_then(|s| s.slider_mut()) {
                // SetMin(0) on the unconnected slider, step 1, page n
                s.min = 0.0;
                if s.value < 0.0 {
                    s.value = 0.0;
                }
                s.step = 1.0;
                s.page = n as f32;
            }
            if resize {
                k::apply_list_height(w, n);
            }
        }
        C::DropDown => {
            let n = w.child("Listbox").and_then(|l| l.listbox()).map(|l| l.template_visible).unwrap_or(0);
            if let Some(Control::DropDown(d)) = w.control.as_deref_mut() {
                d.max_visible = n;
            }
        }
        _ => {}
    }
}

/// Add an entry (`0x5D4250` sorted binary insert / append). Returns its
/// index.
pub fn add_entry(w: &mut Widget, e: ListEntry) -> usize {
    let sorted = w.flags & widget_flag::SORTED != 0;
    let Some(lb) = w.listbox_mut() else { return 0 };
    let at = if sorted {
        // upper bound: equal keys keep insertion order (placement among
        // equal keys not verified natively)
        lb.items.partition_point(|x| k::compare_entries(x, &e) != std::cmp::Ordering::Greater)
    } else {
        lb.items.len()
    };
    let mut e = e;
    e.flags |= 2;
    lb.items.insert(at, e);
    // selection indices after the insert point move down
    for s in &mut lb.selection {
        if *s >= at {
            *s += 1;
        }
    }
    for x in lb.items.iter_mut().skip(at) {
        x.flags |= 2;
    }
    lb.first_dirty = if lb.first_dirty < 0 { at as i32 } else { lb.first_dirty.min(at as i32) };
    at
}

/// Remove every entry.
pub fn clear_entries(w: &mut Widget) {
    if let Some(lb) = w.listbox_mut() {
        lb.items.clear();
        lb.selection.clear();
        lb.top = 0;
        lb.first_dirty = 0;
    }
}
