//! The press pick of KKC_UI `0x5C3BC0` (AA.exe): which widget a mouse press
//! lands on (UI+0x60), and so whether the press reaches the world (VWorld,
//! `DAT_00B36538`), the minimap or a HUD widget.
//!
//! Native path, read in the original machine code and replayed under Unicorn
//! (research/development10/hud-orders/native_press_pick.py, every executed
//! instruction byte-identical in the original and the v14 image):
//!
//! * `0x5C3BC0`: for a left press (0x100 with flag bit0), a left double click
//!   (0x108) or a right press (0x101 with bit0) the event pixel (int16) is
//!   loaded as binary32 and the root widget's (UI+0x74) root view group
//!   (`widget+0x10`) is asked `vt+0x38(x, y, 1)`. When the result differs
//!   from UI+0x60, the old chain loses widget+0x24 bit 3 (`0x5EE360(0)`) and
//!   the result becomes UI+0x60 only when `0x5EFA80` admits it (else NULL),
//!   its chain gaining bit 3.
//! * view element `vt+0x38`: every element class except GROUP uses
//!   `0x5E1680` (vtables A50BD8 DUMMY/base, A50C48 STATUS_BAR, A50D28,
//!   A50E30 LABEL, A515B8 DECAL, A51628, A516A0 DECAL_TILED, A51718
//!   DECAL_TILED_DIRECTIONAL, A52098); GROUP (A50CB8) uses `0x5E2B50`.
//! * `0x5E1680(x, y, client)`: rect = client ? VE+0x54 : VE+0x20; the element
//!   must have VE+0x88 bit 2 (VISIBLE) and `L <= x <= R`, `T <= y <= B`
//!   (x87 `fcomp`, both edges inclusive, unordered fails); the result is the
//!   owner widget VE+0x0C unless that widget's `vt+0x7C` holds (`0x5EDD30`
//!   asks the +8 parents in turn; only the tooltip class A508B0 answers true,
//!   `0x6A71D0`).
//! * `0x5E2B50(x, y, flag)`: its own `0x5E1680` with `client = flag` when the
//!   group's +0xD0 (`transparent`; 1 for a widget's root group, ctor
//!   `0x5E3D80`) is clear, else the outer rect. A miss is NULL. On a hit an
//!   owner with widget+0x24 bit 21 (`intercept_pick`) is the result; else the
//!   children are asked tail to head (`+0xCC` tail, node `+4` previous) with
//!   flag 1 and the first non-NULL answer wins; none is NULL.
//! * `0x5EFA80(w)`: `w+0x14` (the nearest TopLevel or Interface at or above
//!   `w`: their ctor `0x5F32F0` stores the widget itself and vt+0x24 is
//!   `0x4B2830`; every other class copies its parent's in `0x5EEE20`) must be
//!   the focus head `[UI+0x8C]` (NULL when the list is empty) or UI+0x74.
//!
//! VE+0x88 bit 2 itself is rewritten by every element's `vt+0x30`
//! (`0x5E25C0`, from the draw traversal `0x5E1700`/`0x5E1B70`) to
//! `0x5EE2C0(owner) && (when & state) == when` -- exactly the predicate this
//! crate's draw list applies ([`crate::state::ve_visible`]), so an element
//! picks here exactly when the draw list draws it.
use crate::state::{ve_visible, widget_shown};
use crate::ui::{Ui, child_segments, detached};
use crate::ve::{VeKind, ViewElement};
use crate::widget::{Widget, WidgetClass};

/// widget+0x24 bit 21: `intercept_pick` (LoadData `0x5F2070`).
pub const INTERCEPT_PICK: u32 = 0x20_0000;

/// One view element as the pick reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct PickVe {
    /// `Some(+0xD0)` for a GROUP element (vt+0x38 = 0x5E2B50), `None` for
    /// every other class (0x5E1680).
    pub group_d0: Option<bool>,
    /// VE+0x88 bit 2.
    pub visible_88: bool,
    /// VE+0x20..+0x2C: L, T, R, B.
    pub outer_20: [f32; 4],
    /// VE+0x54..+0x60.
    pub client_54: [f32; 4],
    /// VE+0x0C: the owner widget (index into [`Arena::widgets`]).
    pub owner_0c: Option<usize>,
    /// The group's child list from head (+0xC8) to tail (+0xCC).
    pub children: Vec<usize>,
}

/// One widget as the pick reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct PickWidget {
    /// widget+8.
    pub parent_08: Option<usize>,
    /// widget+0x24.
    pub flags_24: u32,
    /// The class's own vt+0x7C answers true (tooltip class A508B0).
    pub vt_7c_true: bool,
    /// widget+0x14.
    pub top_14: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arena {
    pub ves: Vec<PickVe>,
    pub widgets: Vec<PickWidget>,
}

impl Arena {
    /// Widget vt+0x7C (`0x5EDD30` walks the +8 parents).
    pub fn vt_7c(&self, widget: usize) -> bool {
        let mut at = Some(widget);
        while let Some(w) = at {
            if self.widgets[w].vt_7c_true {
                return true;
            }
            at = self.widgets[w].parent_08;
        }
        false
    }
    /// `0x5E1680`.
    pub fn pick_5e1680(&self, ve: usize, x: f32, y: f32, client: bool) -> Option<usize> {
        let v = &self.ves[ve];
        let r = if client { v.client_54 } else { v.outer_20 };
        if !v.visible_88 {
            return None;
        }
        // `test ah,1; jne` (x < L or unordered) and `test ah,41h; jp`
        // (x > R or unordered), then the same for y.
        if !(x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3]) {
            return None;
        }
        let owner = v.owner_0c?;
        if self.vt_7c(owner) {
            return None;
        }
        Some(owner)
    }
    /// `0x5E2B50`.
    pub fn pick_5e2b50(&self, ve: usize, x: f32, y: f32, flag: bool) -> Option<usize> {
        let v = &self.ves[ve];
        let client = !v.group_d0.unwrap_or(false) && flag;
        self.pick_5e1680(ve, x, y, client)?;
        if let Some(owner) = v.owner_0c {
            if self.widgets[owner].flags_24 & INTERCEPT_PICK != 0 {
                return Some(owner);
            }
        }
        v.children.iter().rev().find_map(|&c| self.pick_38(c, x, y, true))
    }
    /// View element vt+0x38.
    pub fn pick_38(&self, ve: usize, x: f32, y: f32, flag: bool) -> Option<usize> {
        match self.ves[ve].group_d0 {
            Some(_) => self.pick_5e2b50(ve, x, y, flag),
            None => self.pick_5e1680(ve, x, y, flag),
        }
    }
    /// `0x5EFA80`: `focus_8c` is `[UI+0x8C]` (None for an empty list).
    pub fn admits_5efa80(&self, widget: usize, focus_8c: Option<usize>, root_74: usize) -> bool {
        let top = self.widgets[widget].top_14;
        top == focus_8c || top == Some(root_74)
    }
    /// The UI+0x60 update of `0x5C3BC0` for one press at the event pixel.
    pub fn press_5c3bc0(
        &self,
        root_group: usize,
        root_74: usize,
        focus_8c: Option<usize>,
        current_60: Option<usize>,
        x: i16,
        y: i16,
    ) -> Option<usize> {
        let hit = self.pick_38(root_group, f32::from(x), f32::from(y), true);
        if hit == current_60 {
            return current_60;
        }
        hit.filter(|&h| self.admits_5efa80(h, focus_8c, root_74))
    }
}

/// The aa-ui tree as the native pick sees it.
#[derive(Clone, Debug)]
pub struct UiPick {
    pub arena: Arena,
    /// Pick/draw path of every widget (`Root/Game/...`).
    pub paths: Vec<String>,
    pub classes: Vec<WidgetClass>,
    /// The root widget (UI+0x74) and its root group.
    pub root: usize,
    pub root_group: usize,
    /// `[UI+0x8C]`.
    pub focus_8c: Option<usize>,
}

/// What a press landed on (UI+0x60 after `0x5C3BC0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PressTarget {
    /// UI+0x60 NULL.
    None,
    /// The VWorld widget `DAT_00B36538` (`UI/Shared/VWorld.tgi:VWorld`, the
    /// Game interface's head child after `0x60BAE0` -> `0x43B490`).
    VWorld,
    /// A widget whose vt+0x90 is true: the minimap class (vtable A4ED20).
    MiniMap { path: String },
    /// Any other widget.
    Widget { path: String, class: WidgetClass },
}

impl UiPick {
    pub fn target(&self, widget: Option<usize>) -> PressTarget {
        let Some(w) = widget else { return PressTarget::None };
        let path = self.paths[w].clone();
        if self.classes[w] == WidgetClass::MiniMap {
            return PressTarget::MiniMap { path };
        }
        if path.eq_ignore_ascii_case(VWORLD_PATH) {
            return PressTarget::VWorld;
        }
        PressTarget::Widget { path, class: self.classes[w] }
    }
    /// `0x5C3BC0` with UI+0x60 = `current`.
    pub fn press(&self, current: Option<usize>, x: i16, y: i16) -> Option<usize> {
        self.arena.press_5c3bc0(self.root_group, self.root, self.focus_8c, current, x, y)
    }
    pub fn index_of(&self, path: &str) -> Option<usize> {
        self.paths.iter().position(|p| p.eq_ignore_ascii_case(path))
    }
}

/// The VWorld widget's path once the Game interface has connected it.
pub const VWORLD_PATH: &str = "Root/Game/VWorld";

fn rect(r: crate::layout::Rect) -> [f32; 4] {
    [r.l, r.t, r.r, r.b]
}

struct Builder<'a> {
    ui: &'a Ui,
    out: UiPick,
}

impl Builder<'_> {
    fn widget(&mut self, w: &Widget, path: String, parent: Option<usize>) -> usize {
        let index = self.out.arena.widgets.len();
        let top = if matches!(w.class, WidgetClass::TopLevel | WidgetClass::Interface) {
            Some(index)
        } else {
            parent.and_then(|p| self.out.arena.widgets[p].top_14)
        };
        self.out.arena.widgets.push(PickWidget { parent_08: parent, flags_24: w.flags, vt_7c_true: false, top_14: top });
        self.out.paths.push(path.clone());
        self.out.classes.push(w.class);
        let group = self.out.arena.ves.len();
        // the root group (0x5E3D80, +0xD0 = 1): vt+0x30 0x5E2AE0 -> 0x5E25C0
        // with no `when`, i.e. 0x5EE2C0 of the owner
        self.out.arena.ves.push(PickVe {
            group_d0: Some(true),
            visible_88: w.loaded && widget_shown(w.flags, w.render_threshold, self.ui.render_level),
            outer_20: rect(w.root.outer.rect),
            client_54: rect(w.root.client.rect),
            owner_0c: Some(index),
            children: Vec::new(),
        });
        let mut children = Vec::new();
        // the widget's own view elements carry the key -FLT_MAX and lead the
        // list (0x5E37A0 -> 0x5EF900); child widget groups follow by key
        for ve in w.view.iter().chain(w.sub_ve.iter()) {
            children.push(self.element(ve, w, index));
        }
        if w.class.internals_ported() {
            for (c, seg) in w.children.iter().zip(child_segments(w)) {
                if detached(w, c) {
                    continue;
                }
                let g = self.widget(c, format!("{path}/{seg}"), Some(index));
                children.push(g);
            }
        }
        self.out.arena.ves[group].children = children;
        group
    }
    fn element(&mut self, ve: &ViewElement, owner: &Widget, owner_index: usize) -> usize {
        let index = self.out.arena.ves.len();
        let group_d0 = match ve.kind {
            VeKind::Group { transparent, .. } => Some(transparent),
            _ => None,
        };
        self.out.arena.ves.push(PickVe {
            group_d0,
            visible_88: ve.layout.flags & crate::layout::ve_flag::VISIBLE != 0
                && ve_visible(ve.when, owner.flags, owner.render_threshold, self.ui.render_level),
            outer_20: rect(ve.layout.outer.rect),
            client_54: rect(ve.layout.client.rect),
            owner_0c: Some(owner_index),
            children: Vec::new(),
        });
        if group_d0.is_some() {
            let children = ve.children.iter().map(|c| self.element(c, owner, owner_index)).collect();
            self.out.arena.ves[index].children = children;
        }
        index
    }
}

impl Ui {
    /// The current tree for `0x5C3BC0`. `[UI+0x8C]`, the focus list head:
    /// TopLevel/Interface widgets join it when activated (`0x5EF290` from
    /// vt+0x48: TopLevel `0x5EF290`, Interface `0x5C08E0` -> `0x5EF290`)
    /// and leave it when deactivated (`0x5EF2D0`), sorted by descending key
    /// with the latest activation first (`0x5C43C0`); a message box on
    /// screen is the latest. Here: the modal message box, else the last
    /// active TopLevel/Interface below Root in tree order (activation order
    /// of this crate's connect paths).
    pub fn press_pick(&self) -> UiPick {
        let mut b = Builder {
            ui: self,
            out: UiPick { arena: Arena::default(), paths: Vec::new(), classes: Vec::new(), root: 0, root_group: 0, focus_8c: None },
        };
        let root_path = self.root.name.trim_start_matches('/').to_string();
        let root_group = b.widget(&self.root, root_path.clone(), None);
        // open drop-down list boxes (the UI overlay) draw above the tree:
        // they are the root group's tail
        for dd in &self.overlay {
            if let Some(lb) = self.widget(dd).and_then(|d| d.child("Listbox")) {
                let parent = b.out.index_of(dd);
                let g = b.widget(lb, format!("{dd}/Listbox"), parent);
                b.out.arena.ves[root_group].children.push(g);
            }
        }
        b.out.root = 0;
        b.out.root_group = root_group;
        let focus = match &self.modal {
            Some(m) => {
                let path = format!("{root_path}/{}", m.trim_start_matches('/'));
                b.out.index_of(&path).or_else(|| b.out.paths.iter().position(|p| p.rsplit('/').next().is_some_and(|s| s.eq_ignore_ascii_case(m.trim_start_matches('/')))))
            }
            None => (1..b.out.paths.len()).rev().find(|&i| {
                matches!(b.out.classes[i], WidgetClass::TopLevel | WidgetClass::Interface)
                    && b.out.arena.widgets[i].flags_24 & crate::state::widget_flag::ACTIVE != 0
            }),
        };
        b.out.focus_8c = focus;
        b.out
    }

    /// UI+0x60 after a press at native client pixel `(x, y)` on the current
    /// tree with UI+0x60 = NULL before it.
    pub fn press_target_5c3bc0(&self, x: i16, y: i16) -> PressTarget {
        let pick = self.press_pick();
        let hit = pick.press(None, x, y);
        pick.target(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ve(group: Option<bool>, visible: bool, r: [f32; 4], owner: usize, children: Vec<usize>) -> PickVe {
        PickVe { group_d0: group, visible_88: visible, outer_20: r, client_54: r, owner_0c: Some(owner), children }
    }
    fn w(parent: Option<usize>, flags: u32, top: Option<usize>) -> PickWidget {
        PickWidget { parent_08: parent, flags_24: flags, vt_7c_true: false, top_14: top }
    }

    #[test]
    fn tail_children_win_and_groups_gate_them() {
        // root group 0 (widget 0) -> [decal 1 (widget 0), group 2 (widget 1) -> [decal 3]]
        let mut a = Arena::default();
        a.widgets = vec![w(None, 0x80007, Some(0)), w(Some(0), 0x80007, Some(0))];
        a.ves = vec![
            ve(Some(true), true, [0.0, 0.0, 800.0, 600.0], 0, vec![1, 2]),
            ve(None, true, [0.0, 0.0, 800.0, 600.0], 0, vec![]),
            ve(Some(true), true, [100.0, 100.0, 200.0, 200.0], 1, vec![3]),
            ve(None, true, [150.0, 150.0, 300.0, 300.0], 1, vec![]),
        ];
        assert_eq!(a.pick_38(0, 160.0, 160.0, true), Some(1));
        // inside the decal but outside its group: the group gates it
        assert_eq!(a.pick_38(0, 250.0, 250.0, true), Some(0));
        // both edges are inclusive
        assert_eq!(a.pick_38(0, 200.0, 200.0, true), Some(1));
        assert_eq!(a.pick_38(0, f32::NAN, 10.0, true), None);
        // an empty visible group never picks itself
        a.ves[3].visible_88 = false;
        assert_eq!(a.pick_38(0, 160.0, 160.0, true), Some(0));
        a.widgets[1].flags_24 |= INTERCEPT_PICK;
        assert_eq!(a.pick_38(0, 160.0, 160.0, true), Some(1));
        a.widgets[1].vt_7c_true = true;
        assert_eq!(a.pick_38(0, 160.0, 160.0, true), Some(0));
    }
}
