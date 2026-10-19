//! The game UIWorldState: the UI-side selection manager of the battle.
//!
//! Constructor 0x640640 (vtable 0xA61E40) over the base 0x438E10 (vtable
//! 0x9FD648); VWorld+0x13C points at it while the game interface is active
//! (0x43B490, called by KKC_GameInterface 0x60BAE0). Slots are the game
//! override where one exists, otherwise the base implementation:
//!
//! | slot | native | here |
//! |---|---|---|
//! | +0x00 | 0x436600 | [`State::first_voiced_436600`] |
//! | +0x04 | 0x6409E0 (base 0x4365A0) | [`State::select`] |
//! | +0x08 | 0x6406A0 (base 0x437A40) | [`State::clear`] |
//! | +0x0C | 0x438DF0 | [`State::deselect_438df0`] |
//! | +0x10 | 0x640660 | [`State::selectable_640660`] |
//! | +0x14 | 0x438370 | [`State::add_438370`] |
//! | +0x18 | 0x436950 | [`State::can_send_436950`] |
//! | +0x24 | 0x6A71F0 (AL = 0) | the `include` argument of FindObjectAt |
//! | +0x34 | 0x4363C0 | [`State::clear_hover_4363c0`] |
//! | +0x38 | 0x436650 | [`State::update_hover_436650`] |
//! | +0x3C | 0x640CF0 | [`State::click_640cf0`] |
//! | +0x40 | 0x640E10 | [`State::click_add_640e10`] |
//! | +0x44 | 0x6406B0 (base 0x437DE0) | [`State::clear_candidates_6406b0`] |
//! | +0x48 | 0x4377B0 | [`State::box_select_4377b0`] |
//! | +0x4C | 0x436D70 | [`State::hover_choice_436d70`] |
//! | +0x50 | 0x437B20 | [`State::click_hits_437b20`] |
//! | +0x54 | 0x6409A0 | [`State::pick_target_6409a0`] |
//! | +0x58 | 0x640B40 | [`State::select_owner_640b40`] |
//! | +0x60 | 0x640A40 | [`State::candidate_640a40`] |
//! | +0x64 | 0x640B20 | [`State::compatible_640b20`] |
//! | +0x68 | 0x4365D0 | [`State::highlight_4365d0`] |
//! | +0x6C | 0x640FA0 (base 0x4368C0) | [`State::select_list_640fa0`] |
//!
//! Native lists are singly linked with head insertion; here they are vectors
//! in native traversal order (index 0 is the head node). View objects are
//! portable handles; every field, virtual and global the native code reads
//! goes through [`Host`], which has no successful defaults.
use super::event::Event;
use super::requests::Requests;
use super::x87::{self, b, st};
use aa_numeric::Precision;
use aa_orders::commands::CommandWire;

pub type Result<T> = std::result::Result<T, String>;

/// View flags (`VThing+0x54`) read or written here.
pub mod view_flag {
    /// 0x4365A0 never selects such an object; 0x5C19E0 never picks it.
    pub const UNSELECTABLE: u32 = 0x80;
    /// Thing flag 14 mirrored by 0x45BBA0; the 0x45B470 fallback.
    pub const THING_4000: u32 = 0x4000;
    /// Hover/box highlight (0x4365D0).
    pub const HIGHLIGHT: u32 = 0x8000;
    /// On the box-candidate list +0x14 (0x437490 / 0x437DE0).
    pub const CANDIDATE: u32 = 0x10000;
    /// On the selection list +0x08 (0x438370 / 0x438430 / 0x437A40).
    pub const SELECTED: u32 = 0x20000;
    /// Compared by 0x6409A0 and 0x5C19E0; 0x5C19E0 rejects it.
    pub const BIT21: u32 = 0x200000;
}
fn bit21(flags: u32) -> u32 {
    flags >> 21 & 1
}

/// UIWorldState+0x40: the static group matching the selection (0x4374D0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticRef {
    None,
    /// +0x44, used when more than one object is selected.
    Wildcard,
    /// An entry of the +0x34 list.
    Group(usize),
}
/// One +0x34 UI group (0x24 bytes, ctor 0x439010): the control groups the
/// SaveGroup/SelectGroup hotkeys name ("group_1".."group_10").
///
/// Native members are view pointers kept in ascending POINTER order by the
/// lower-bound insert 0x869D30 (comparator 0x4E1AB0). Heap addresses are not
/// reproducible, so this port keeps them in ascending `V` order; with view
/// handles allocated in creation order this is the allocation-order
/// equivalent. Membership (0x4374D0) does not depend on the order;
/// SelectGroup's selection order does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticGroup<V> {
    /// Allocation identity (the native group pointer).
    pub id: u64,
    /// +0x00 name (the CString compared with wcscmp 0x96CB83).
    pub name: Vec<u16>,
    /// +0x0C/+0x10/+0x14 members.
    pub members: Vec<V>,
    /// +0x18 behaviour byte (0x439010 stores 0; a rebuilt group gets 1, then
    /// the previous current group's value).
    pub byte_18: bool,
    /// +0x1C
    pub word_1c: u32,
}
impl<V> StaticGroup<V> {
    /// The 0x439010 values of the fields this port keeps (display text +4
    /// and digits +8 are presentation, built by the HUD from `name`).
    pub fn new(id: u64, name: Vec<u16>, members: Vec<V>) -> Self {
        Self { id, name, members, byte_18: false, word_1c: 0 }
    }
}

/// The fields of the game UIWorldState this port owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State<V> {
    selected: Vec<V>,
    hovered: Option<V>,
    candidates: Vec<V>,
    changed_1c: bool,
    command_masks_20: [u32; 4],
    groups: Vec<StaticGroup<V>>,
    static_40: StaticRef,
    /// +0x44: the wildcard group (0x438E10 builds it with byte +0x18 = 0).
    wildcard_44: StaticGroup<V>,
    /// +0x48: the group SaveGroup is rebuilding (0x4385B0 never deletes it).
    rebuilding_48: Option<u64>,
    /// `VThing+0x150` back links (view -> group id) for the views in groups.
    links_150: Vec<(V, u64)>,
    next_group_id: u64,
    candidate_class_50: u32,
    selection_class_54: u32,
    /// +0x4C: the local kingdom 0x43A1C0 last saw (constructor 0).
    kingdom_4c: Option<u64>,
    /// +0x1D: the group-rebuild request the simulation's view notifier sets
    /// (0x50C2E7 / 0x50C9D3); 0x43A1C0 clears it (constructor 0).
    groups_changed_1d: bool,
}

/// One Kingdom+0x34C group record as 0x439BF0 reads it: +0x00 name (the
/// CString 0x439010 copies), +0x0C/+0x10 member handles (B364C0), +0x18.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KingdomGroup {
    pub name: Vec<u16>,
    pub members: Vec<u32>,
    pub byte_18: bool,
}

/// 0x4385E0's name of the +0x44 wildcard group (0x438E10, literal 0x9FD6D0).
pub const WILDCARD_NAME: &str = "static_group_wildcard_name";

/// The data leaves and effects of the selection manager, named after the
/// native routine, field or virtual slot each stands for. None has a
/// successful default.
pub trait Leaves {
    type View: Copy + Eq + std::fmt::Debug;
    /// The x87 precision control of the UI thread running these routines.
    fn precision(&self) -> Precision;

    // ---- view fields ----
    /// `VThing+0x54`.
    fn flags_54(&mut self, view: Self::View) -> Result<u32>;
    fn set_flags_54(&mut self, view: Self::View, flags: u32) -> Result<()>;
    /// `VThing+0x38..+0x40` world position (binary32 words).
    fn position_38(&mut self, view: Self::View) -> Result<[u32; 3]>;
    /// 0x58CB30 = `VThing+0x10` as an opaque identity (0x640BF0, 0x640E10).
    fn field_10_58cb30(&mut self, view: Self::View) -> Result<u64>;

    // ---- view virtuals ----
    /// vt+0x9C: pickable (0x5C19E0, 0x4367A0, 0x640BF0, 0x43D240).
    fn pickable_9c(&mut self, view: Self::View) -> Result<bool>;
    /// vt+0x78 / +0x84 / +0x7C / +0x80: the 0x800/0x400/0x200/0x100 classes
    /// of 0x6406C0; vt+0x78 also gates the 0x640CF0 double click.
    fn class_78(&mut self, view: Self::View) -> Result<bool>;
    fn class_84(&mut self, view: Self::View) -> Result<bool>;
    fn class_7c(&mut self, view: Self::View) -> Result<bool>;
    fn class_80(&mut self, view: Self::View) -> Result<bool>;
    /// vt+0x50(1) / vt+0x54(1), read by 0x640660.
    fn virtual_50(&mut self, view: Self::View) -> Result<bool>;
    fn virtual_54(&mut self, view: Self::View) -> Result<bool>;

    // ---- the actor at VThing+0x14 ----
    /// `VThing+0x14 != NULL`.
    fn actor_present_14(&mut self, view: Self::View) -> Result<bool>;
    /// 0x45B7A0 != 0: +0x14 present and its virtual +0xB4 true.
    fn actor_45b7a0(&mut self, view: Self::View) -> Result<bool>;
    /// `actor+8 & 1` (0x45B7D0; only asked when +0x14 is present).
    fn actor_flag8_bit0(&mut self, view: Self::View) -> Result<bool>;
    /// actor vt+0x58(0x5C0E50(1)) (0x5C19E0; +0x14 present).
    fn actor_virtual_58(&mut self, view: Self::View) -> Result<bool>;
    /// actor vt+0x84(0x5C0E50(1)) (0x45B470; +0x14 present).
    fn actor_virtual_84(&mut self, view: Self::View) -> Result<bool>;
    /// 0x4D12C0, actor vt+0x20(1), actor+0x70 != 0, actor+0x90 != 0
    /// (0x6406C0; asked only when 0x45B7A0 holds, in that order).
    fn actor_4d12c0(&mut self, view: Self::View) -> Result<bool>;
    fn actor_virtual_20(&mut self, view: Self::View) -> Result<bool>;
    fn actor_70(&mut self, view: Self::View) -> Result<bool>;
    fn actor_90(&mut self, view: Self::View) -> Result<bool>;
    /// 0x437650: the 0x45B7A0 actor's +0x16C handle resolved through
    /// B364C0 and that object's +0x0C view, when all are nonzero.
    fn link_16c_view(&mut self, view: Self::View) -> Result<Option<Self::View>>;
    /// 0x640B40: the actor's +0x94 -> +0x34 handle resolved and its +0x0C.
    fn link_94_view(&mut self, view: Self::View) -> Result<Option<Self::View>>;
    /// 0x5E05D0's test: the 0x45B7A0 actor's +0x94 is nonzero and its +0x34
    /// handle resolves to a nonzero B364C0 entry.
    fn link_94_live(&mut self, view: Self::View) -> Result<bool>;
    /// 0x640B40: `s = 0x4549C0(actor)` (actor+0x80 -> +0x10) is nonzero and
    /// `s` vt+0x58(0x5C0E50(1)) is false.
    fn hidden_4549c0(&mut self, view: Self::View) -> Result<bool>;
    /// `actor+0x0C`: the actor's own view.
    fn actor_view_0c(&mut self, view: Self::View) -> Result<Option<Self::View>>;
    /// `0x4CE6C0(actor)+0x0C`: the view of the actor's organization when it
    /// is a member (+0x7C NULL, +0x80 -> +0x10 nonzero), else its own.
    fn root_view_4ce6c0(&mut self, view: Self::View) -> Result<Option<Self::View>>;
    /// 0x436600's loop body after its 0x45B7A0 holds: `actor+0x7C -> +0x40`,
    /// `+0xA4 != 0` breaks the loop with `Some(record+0x0C)`; `None` continues.
    fn voiced_436600(&mut self, view: Self::View) -> Result<Option<Option<Self::View>>>;
    /// 0x436950's loop body after its 0x45B7A0 holds: the actor's
    /// vt+0x150(0x5C0E30() local kingdom, &cmd, 1) (GActor/GGroup/GKingdom
    /// 0x4CF430). The actor may complete the command (0x4DA570's type-4
    /// branch), so the wire is mutable.
    fn actor_virtual_150(&mut self, view: Self::View, wire: &mut CommandWire) -> Result<bool>;

    // ---- UI / world ----
    /// 0x436850: the selection acknowledgement (application state, sound
    /// option, 0x58CB30 voice and 0x42A590).
    fn selection_sound_436850(&mut self, view: Self::View) -> Result<()>;
    /// 0x437EB0 ([`super::command_masks`]): recompute +0x20..+0x2C in place
    /// (native partial stores when a leaf fails) from the selection +8 and the
    /// +0x40 group's +0x18 byte (None: +0x40 NULL). Its +0x4C kingdom is the
    /// 0x43A1C0 per-frame cache ([`State::kingdom_4c`]).
    fn command_masks_437eb0(&mut self, selected: &[Self::View], kingdom_4c: Option<u64>, group_40_byte_18: Option<bool>, masks: &mut [u32; 4]) -> Result<()>;
    /// UIWorldState+0x30 brush 0x42D4D0 (brush+8 = 1).
    fn brush_changed_42d4d0(&mut self) -> Result<()>;
    /// `VWorld+0x140 != NULL`, then 0x4325C0(3) on it.
    fn vworld_list_changed_4325c0(&mut self) -> Result<()>;
    /// `VWorld+0xAD` (0x437DE0).
    fn vworld_ad(&mut self) -> Result<bool>;
    /// The group floater of a view: 0x42EB10 on `view+0x9C` when nonzero
    /// (0x451860 / 0x452A90), and the 0x453470 refresh after a link change.
    fn floater_remove_42eb10(&mut self, view: Self::View) -> Result<()>;
    fn floater_update_453470(&mut self, view: Self::View) -> Result<()>;
    /// The selection-request emitters.
    fn requests(&mut self) -> &mut dyn Requests<Self::View>;

    // ---- per-frame 0x43A1C0 and the group rebuild 0x439BF0 ----
    /// 0x765E00(0x5C0E10()): the local player's kingdom (player+0x28; NULL
    /// while GWorld B36534 is NULL), as an identity.
    fn local_kingdom_765e00(&mut self) -> Result<Option<u64>>;
    /// `[kingdom+0x34C][0..+0x350]`: the kingdom's group records in order.
    fn kingdom_groups_34c(&mut self, kingdom: u64) -> Result<Vec<KingdomGroup>>;
    /// B364C0 of a member handle (generation word table+4+2*slot, object
    /// table+0x20004+4*slot): None when it does not resolve, else the
    /// object's +0x0C view.
    fn view_of_handle_b364c0(&mut self, handle: u32) -> Result<Option<Option<Self::View>>>;
    /// `[[0x5C0E10()+0x50]+0x24]+0x18`: the byte 0x439E58 copies into the
    /// +0x44 wildcard group.
    fn player_wildcard_byte_50(&mut self) -> Result<bool>;
}

/// The UI-manager and VWorld hooks: implemented by the state machine
/// ([`super::states`]) over the scene ([`super::pick`]).
pub trait Host: Leaves {
    /// KKC_VWorld::FindObjectAt 0x43D240(origin, direction, AL of vt+0x24,
    /// hits). The game UIWorldState's vt+0x24 is 0x6A71F0 (AL = 0).
    fn find_object_at_43d240(
        &mut self,
        origin: [u32; 3],
        direction: [u32; 3],
        hits: Option<&mut Vec<Self::View>>,
    ) -> Result<Option<Self::View>>;
    /// 0x436DD0: build the four planes of the box between the two events'
    /// world points (kept by the host for [`Host::box_inside_4367a0`]).
    fn box_frustum_436dd0(&mut self, start: &Event, end: &Event) -> Result<()>;
    /// 0x4371A0's planes for the dragged screen rectangle (press, cursor).
    fn box_frustum_4371a0(&mut self, rect: [i32; 4]) -> Result<()>;
    /// 0x4367A0's per-object prefix: `+0x18 == 2`, vt+0x9C, then 0x454B50
    /// (model bound inside the planes).
    fn box_inside_4367a0(&mut self, view: Self::View) -> Result<bool>;
    /// VWorld+0x94 -> +0x100: the view-thing list 0x43D240, 0x4367A0 and
    /// 0x640BF0 walk (next at +8, 0x42EEC0), in list order.
    fn vworld_list_94(&mut self) -> Result<Vec<Self::View>>;
    /// UI+0x60 vt+0x64(x, y) (VWorld: 0x43DC70); None when UI+0x60 is NULL.
    fn pick_under_cursor_43dc70(&mut self, x: i32, y: i32) -> Result<Option<Self::View>>;
    /// 0x5C1E70: the current UI state's vt+0x20 (false without a state).
    fn hover_enabled_5c1e70(&mut self) -> Result<bool>;
    /// 0x5C1970: the current UI state's vt+0x24 chooser, or the plain
    /// `a_distance < b_distance ? a : b` without a state.
    fn choose_5c1970(&mut self, a: Option<Self::View>, a_distance: u32, b: Option<Self::View>, b_distance: u32) -> Result<Option<Self::View>>;
    /// The tail of 0x5C19E0: the current UI state's vt+0x18(view); None
    /// when no UI state is installed (0x5C19E0 then returns false).
    fn state_pick_filter_18(&mut self, view: Self::View) -> Result<Option<bool>>;
}

impl<V: Copy + Eq + std::fmt::Debug> Default for State<V> {
    fn default() -> Self {
        Self::from_constructor_640640()
    }
}

impl<V: Copy + Eq + std::fmt::Debug> State<V> {
    /// The 0x438E10/0x640640 constructor values of the fields owned here.
    pub fn from_constructor_640640() -> Self {
        Self {
            selected: Vec::new(),
            hovered: None,
            candidates: Vec::new(),
            changed_1c: false,
            command_masks_20: [0; 4],
            groups: Vec::new(),
            static_40: StaticRef::None,
            wildcard_44: StaticGroup::new(0, WILDCARD_NAME.encode_utf16().collect(), Vec::new()),
            rebuilding_48: None,
            links_150: Vec::new(),
            next_group_id: 1,
            candidate_class_50: 0,
            selection_class_54: 0,
            kingdom_4c: None,
            groups_changed_1d: false,
        }
    }
    /// A manager holding `selected` (head first) and `hovered` (+0x10), for
    /// the state-machine oracle tests.
    #[cfg(test)]
    pub(crate) fn for_state_tests(selected: Vec<V>, hovered: Option<V>) -> Self {
        let mut s = Self::from_constructor_640640();
        s.selected = selected;
        s.hovered = hovered;
        s
    }
    /// The same manager with +0x40 = the wildcard group +0x44 whose +0x18 is
    /// `byte_18` (None leaves +0x40 NULL), for the 0x436950 oracle test.
    #[cfg(test)]
    pub(crate) fn with_static_for_state_tests(mut self, byte_18: Option<bool>) -> Self {
        if let Some(b) = byte_18 {
            self.wildcard_44.byte_18 = b;
            self.static_40 = StaticRef::Wildcard;
        }
        self
    }
    /// The view's `+0x150` group, if any.
    pub fn group_of_150(&self, view: V) -> Option<u64> {
        self.links_150.iter().find(|(v, _)| *v == view).map(|(_, g)| *g)
    }
    /// The selection list, head first.
    pub fn selected(&self) -> &[V] {
        &self.selected
    }
    pub fn hovered_10(&self) -> Option<V> {
        self.hovered
    }
    pub fn candidates_14(&self) -> &[V] {
        &self.candidates
    }
    pub fn changed_1c(&self) -> bool {
        self.changed_1c
    }
    /// The HUD consumer of +0x1C clears it after redrawing.
    pub fn take_changed_1c(&mut self) -> bool {
        std::mem::replace(&mut self.changed_1c, false)
    }
    pub fn command_masks_20(&self) -> [u32; 4] {
        self.command_masks_20
    }
    pub fn static_40(&self) -> StaticRef {
        self.static_40
    }
    pub fn groups(&self) -> &[StaticGroup<V>] {
        &self.groups
    }
    pub fn groups_mut(&mut self) -> &mut Vec<StaticGroup<V>> {
        &mut self.groups
    }
    pub fn candidate_class_50(&self) -> u32 {
        self.candidate_class_50
    }
    pub fn selection_class_54(&self) -> u32 {
        self.selection_class_54
    }
    /// 0x436510: the head and tail nodes hold different objects.
    pub fn multiple_436510(&self) -> bool {
        match (self.selected.first(), self.selected.last()) {
            (Some(h), Some(t)) => h != t,
            _ => false,
        }
    }
    /// `head obj == tail obj` of a nonempty list (0x437B20, 0x436D70).
    fn single(&self) -> Option<V> {
        match (self.selected.first(), self.selected.last()) {
            (Some(h), Some(t)) if h == t => Some(*h),
            _ => None,
        }
    }

    // ---- predicates ----

    /// 0x6406C0(view, current): the selection class bits, 0 = refused.
    /// Leaves are read lazily in native order.
    pub fn class_6406c0<H: Host<View = V>>(host: &mut H, view: V, current: u32) -> Result<u32> {
        let mut kind: u32 = if host.class_78(view)? {
            0x800
        } else if host.class_84(view)? {
            0x400
        } else if host.class_7c(view)? {
            0x200
        } else if host.class_80(view)? {
            0x100
        } else {
            0
        };
        let mut current = current;
        if current != 0 {
            if (kind as i32) < (current & 0xf00) as i32 {
                return Ok(0);
            }
            if (kind as i32) > current as i32 {
                current = 0;
            }
        }
        // `xor eax,eax; cmp ebx,edi; setg al; dec eax; and eax,edi`
        let pick = |kind: u32, current: u32| if (current as i32) > (kind as i32) { 0 } else { kind };
        let plain = |kind: u32, current: u32| {
            pick(if current != 0 && current & 0xdd == 0 { kind | 2 } else { kind | 0x20 }, current)
        };
        if !host.actor_45b7a0(view)? {
            return Ok(plain(kind, current));
        }
        let member = host.actor_4d12c0(view)?;
        if !host.actor_virtual_20(view)? {
            kind |= if current != 0 && current & 0x18 != 0 { 0x10 } else { 8 };
            return Ok(pick(kind, current));
        }
        if host.actor_70(view)? && !member {
            kind |= if current != 0 && current & 0xc0 != 0 { 0x80 } else { 0x40 };
            return Ok(pick(kind, current));
        }
        if !host.actor_90(view)? {
            return Ok(plain(kind, current));
        }
        kind |= if current != 0 && current & 5 != 0 { 1 } else { 4 };
        Ok(pick(kind, current))
    }
    /// 0x45B470 (also the game UI states' vt+0x18 0x60CA00).
    pub fn selectable_45b470<H: Host<View = V>>(host: &mut H, view: V) -> Result<bool> {
        if host.actor_present_14(view)? && host.actor_virtual_84(view)? {
            return Ok(true);
        }
        Ok(host.flags_54(view)? & view_flag::THING_4000 != 0)
    }
    /// vt+0x10 = 0x640660.
    pub fn selectable_640660<H: Host<View = V>>(host: &mut H, view: Option<V>) -> Result<bool> {
        let Some(view) = view else { return Ok(false) };
        if !Self::selectable_45b470(host, view)? {
            return Ok(false);
        }
        Ok(host.virtual_50(view)? || host.virtual_54(view)?)
    }
    /// 0x437650.
    fn pick_owner_437650<H: Leaves<View = V>>(host: &mut H, view: Option<V>) -> Result<Option<V>> {
        let Some(view) = view else { return Ok(None) };
        if !host.actor_45b7a0(view)? {
            return Ok(Some(view));
        }
        Ok(Some(host.link_16c_view(view)?.unwrap_or(view)))
    }
    /// vt+0x54 = 0x6409A0: the target a pointer object stands for.
    pub fn pick_target_6409a0<H: Leaves<View = V>>(host: &mut H, view: Option<V>) -> Result<Option<V>> {
        let Some(owner) = Self::pick_owner_437650(host, view)? else { return Ok(None) };
        if !host.actor_45b7a0(owner)? {
            return Ok(Some(owner));
        }
        let root = host
            .root_view_4ce6c0(owner)?
            .ok_or("0x6409C6 reads +0x54 of a NULL 0x4CE6C0 view (native NULL dereference)")?;
        if bit21(host.flags_54(root)?) != bit21(host.flags_54(owner)?) {
            Ok(Some(owner))
        } else {
            Ok(Some(root))
        }
    }
    /// vt+0x58 = 0x640B40: the object a click on `view` selects.
    pub fn select_owner_640b40<H: Host<View = V>>(host: &mut H, view: Option<V>) -> Result<Option<V>> {
        let Some(owner) = Self::pick_owner_437650(host, view)? else { return Ok(None) };
        if !host.actor_45b7a0(owner)? {
            return Ok(Some(owner));
        }
        if let Some(container) = host.link_94_view(owner)? {
            return Ok(Some(container));
        }
        if host.hidden_4549c0(owner)? {
            return host.actor_view_0c(owner);
        }
        host.root_view_4ce6c0(owner)
    }
    /// The UI manager's pick filter 0x5C19E0.
    pub fn pick_filter_5c19e0<H: Host<View = V>>(host: &mut H, view: Option<V>) -> Result<bool> {
        let Some(mut target) = view else {
            return Err("0x5C19E5 calls vt+0x9C of a NULL object (native NULL dereference)".into());
        };
        if host.pickable_9c(target)? {
            let mapped = Self::pick_target_6409a0(host, Some(target))?
                .ok_or("0x5C1A08 reads +0x54 of a NULL vt+0x54 result (native NULL dereference)")?;
            if bit21(host.flags_54(mapped)?) == bit21(host.flags_54(target)?) {
                target = mapped;
            }
        }
        let flags = host.flags_54(target)?;
        if flags & view_flag::UNSELECTABLE != 0 || flags & view_flag::BIT21 != 0 {
            return Ok(false);
        }
        if host.actor_present_14(target)? && !host.actor_virtual_58(target)? {
            return Ok(false);
        }
        Ok(host.state_pick_filter_18(target)?.unwrap_or(false))
    }

    // ---- list operations ----

    /// Game vt+4 = 0x6409E0: at most six selected objects, class gating via
    /// 0x6406C0 and +0x54, then the base 0x4365A0.
    pub fn select<H: Host<View = V>>(&mut self, host: &mut H, view: V, sound: bool) -> Result<()> {
        if self.selected.len() >= 6 {
            return Ok(());
        }
        let class = Self::class_6406c0(host, view, self.selection_class_54)?;
        if class == 0 {
            return Ok(());
        }
        if (class as i32) > (self.selection_class_54 as i32) && class & 0x93 == 0 {
            self.clear(host, false)?;
        }
        self.selection_class_54 = class;
        // 0x4365A0
        if host.flags_54(view)? & view_flag::UNSELECTABLE != 0 {
            return Ok(());
        }
        if let Some(owner) = Self::select_owner_640b40(host, Some(view))? {
            self.add_438370(host, owner, sound)?;
        }
        Ok(())
    }
    /// vt+0x14 = 0x438370.
    pub fn add_438370<H: Host<View = V>>(&mut self, host: &mut H, view: V, sound: bool) -> Result<()> {
        if host.flags_54(view)? & view_flag::SELECTED != 0 {
            return Ok(());
        }
        host.requests().add_436ad0(view, None)?;
        if sound {
            host.selection_sound_436850(view)?;
        }
        self.selected.insert(0, view);
        let flags = host.flags_54(view)?;
        host.set_flags_54(view, flags | view_flag::SELECTED)?;
        self.refresh_static_4374d0(host)?;
        self.changed_1c = true;
        self.refresh_masks_437eb0(host)?;
        host.brush_changed_42d4d0()?;
        host.vworld_list_changed_4325c0()
    }
    /// Game vt+8 = 0x6406A0: +0x54 = 0, then 0x437A40(`silent`).
    pub fn clear<H: Host<View = V>>(&mut self, host: &mut H, silent: bool) -> Result<()> {
        self.selection_class_54 = 0;
        if !silent {
            host.requests().clear_436c50(None)?;
        }
        while !self.selected.is_empty() {
            let view = self.selected.remove(0);
            let flags = host.flags_54(view)?;
            host.set_flags_54(view, flags & !view_flag::SELECTED)?;
        }
        self.changed_1c = true;
        self.refresh_static_4374d0(host)?;
        self.command_masks_20 = [0; 4];
        host.vworld_list_changed_4325c0()
    }
    /// vt+0xC = 0x438DF0: vt+0x58, then 0x438AD0.
    pub fn deselect_438df0<H: Host<View = V>>(&mut self, host: &mut H, view: V) -> Result<()> {
        let owner = Self::select_owner_640b40(host, Some(view))?
            .ok_or("0x438AD0 reached with a NULL owner (native NULL dereference at 0x59E240)")?;
        self.remove_438ad0(host, owner)
    }
    /// 0x438AD0(view): SelectionRemove request (0x59E240 -> 0x436B90),
    /// 0x438430, 0x4374D0, 0x437EB0, the brush 0x42D4D0, VWorld+0x140 0x4325C0(3).
    fn remove_438ad0<H: Host<View = V>>(&mut self, host: &mut H, owner: V) -> Result<()> {
        host.requests().remove_436b90(owner, None)?;
        // 0x438430
        if let Some(index) = self.selected.iter().position(|v| *v == owner) {
            self.selected.remove(index);
        }
        let flags = host.flags_54(owner)?;
        host.set_flags_54(owner, flags & !view_flag::SELECTED)?;
        self.changed_1c = true;
        self.refresh_static_4374d0(host)?;
        self.refresh_masks_437eb0(host)?;
        // back in 0x438AD0
        self.refresh_static_4374d0(host)?;
        self.refresh_masks_437eb0(host)?;
        host.brush_changed_42d4d0()?;
        host.vworld_list_changed_4325c0()
    }
    fn refresh_masks_437eb0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        let group = match self.static_40 {
            StaticRef::None => None,
            StaticRef::Wildcard => Some(self.wildcard_44.byte_18),
            StaticRef::Group(i) => Some(self.groups.get(i).ok_or("UIWorldState+0x40 names no +0x34 group")?.byte_18),
        };
        host.command_masks_437eb0(&self.selected, self.kingdom_4c, group, &mut self.command_masks_20)
    }
    /// 0x4374D0.
    fn refresh_static_4374d0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        if self.selected.is_empty() {
            self.static_40 = StaticRef::None;
            return Ok(());
        }
        self.static_40 = if self.multiple_436510() { StaticRef::Wildcard } else { StaticRef::None };
        'groups: for index in 0..self.groups.len() {
            let mut remaining: i32 = 0;
            for member in self.groups[index].members.clone() {
                // 0x45B7D0 is true for a NULL +0x14 or actor+8 bit0.
                let absent = !host.actor_present_14(member)? || host.actor_flag8_bit0(member)?;
                if !absent {
                    remaining += 1;
                }
            }
            for view in self.selected.clone() {
                if !host.actor_45b7a0(view)? {
                    continue;
                }
                if !self.groups[index].members.contains(&view) {
                    continue 'groups;
                }
                remaining -= 1;
            }
            if remaining == 0 {
                self.static_40 = StaticRef::Group(index);
                return Ok(());
            }
        }
        Ok(())
    }
    /// vt+0 = 0x436600.
    pub fn first_voiced_436600<H: Host<View = V>>(&self, host: &mut H) -> Result<Option<V>> {
        for view in &self.selected {
            if host.actor_45b7a0(*view)? {
                if let Some(voiced) = host.voiced_436600(*view)? {
                    return Ok(voiced);
                }
            }
        }
        Ok(self.selected.first().copied())
    }

    // ---- groups ----

    fn group_index(&self, id: u64) -> Option<usize> {
        self.groups.iter().position(|g| g.id == id)
    }
    /// 0x4384A0: remove the group from +0x34 (0x438240 destroys it).
    fn delete_group_4384a0(&mut self, id: u64) {
        if let Some(index) = self.group_index(id) {
            self.groups.remove(index);
        }
    }
    /// 0x4385B0 on a view's +0x150: delete that group when it is empty and
    /// not the one being rebuilt.
    fn release_link_4385b0(&mut self, group: u64) {
        if self.rebuilding_48 == Some(group) {
            return;
        }
        if let Some(index) = self.group_index(group) {
            if self.groups[index].members.is_empty() {
                self.delete_group_4384a0(group);
            }
        }
    }
    /// 0x451860 / the 0x452A90 tail: floater, 0x4385B0, `+0x150 = 0`.
    fn unlink_view_451860<H: Leaves<View = V>>(&mut self, host: &mut H, view: V) -> Result<()> {
        host.floater_remove_42eb10(view)?;
        if let Some(group) = self.group_of_150(view) {
            self.release_link_4385b0(group);
        }
        self.links_150.retain(|(v, _)| *v != view);
        Ok(())
    }
    /// The byte +0x18 of the group +0x40 names (0 for NULL).
    fn current_byte_18(&self) -> bool {
        match self.static_40 {
            StaticRef::None => false,
            StaticRef::Wildcard => self.wildcard_44.byte_18,
            StaticRef::Group(i) => self.groups.get(i).map(|g| g.byte_18).unwrap_or(false),
        }
    }
    /// 0x43A1C0, the per-frame update VWorld 0x440159 calls on VWorld+0x13C:
    /// every selected object whose vt+0x10 (0x640660) fails is removed by
    /// 0x438AD0 (the next node is read before the call); then when +0x1D is
    /// set or the local kingdom 0x765E00(0x5C0E10()) differs from +0x4C:
    /// +0x4C = it, +0x1D = 0, 0x439BF0, 0x4374D0; then the +0x30 brush's
    /// 0x4B2830 (`ret`) and the tail jump to 0x437EB0.
    pub fn update_43a1c0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()>
    where
        V: Ord,
    {
        for view in self.selected.clone() {
            if !Self::selectable_640660(host, Some(view))? {
                self.remove_438ad0(host, view)?;
            }
        }
        let kingdom = host.local_kingdom_765e00()?;
        if self.groups_changed_1d || self.kingdom_4c != kingdom {
            self.kingdom_4c = kingdom;
            self.groups_changed_1d = false;
            self.rebuild_groups_439bf0(host)?;
            self.refresh_static_4374d0(host)?;
        }
        self.refresh_masks_437eb0(host)
    }
    /// +0x4C (0x43A1C0's cache; the 0x437EB0 kingdom).
    pub fn kingdom_4c(&self) -> Option<u64> {
        self.kingdom_4c
    }
    /// +0x1D = 1: the simulation's view notifier (0x50C2B0 after 0x43B200,
    /// 0x50C970) asks 0x43A1C0 to rebuild the groups.
    pub fn request_group_rebuild_1d(&mut self) {
        self.groups_changed_1d = true;
    }
    /// 0x438510: destroy every +0x34 group (0x438240: 0x436D00 unlinks its
    /// members through 0x451860), count +0x38 = 0. +0x40 is not touched.
    fn clear_groups_438510<H: Leaves<View = V>>(&mut self, host: &mut H) -> Result<()> {
        for index in 0..self.groups.len() {
            for member in self.groups[index].members.clone() {
                // 0x451860 while the group still counts its members: the
                // 0x4385B0 release finds it nonempty.
                host.floater_remove_42eb10(member)?;
                self.links_150.retain(|(v, _)| *v != member);
            }
        }
        self.groups.clear();
        Ok(())
    }
    /// 0x439BF0: 0x438510, then with a local kingdom a UI group per
    /// Kingdom+0x34C record (0x439010 copies the name, +0x18 copied), each
    /// resolved member's view linked by 0x453520 and lower-bound inserted
    /// (0x869D30 / 0x4E1AB0); a group with members is appended to +0x34,
    /// an empty one destroyed; then +0x44's +0x18 from the player and 0x4374D0.
    fn rebuild_groups_439bf0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()>
    where
        V: Ord,
    {
        self.clear_groups_438510(host)?;
        let Some(kingdom) = host.local_kingdom_765e00()? else { return Ok(()) };
        for record in host.kingdom_groups_34c(kingdom)? {
            let id = self.next_group_id;
            self.next_group_id += 1;
            let mut building = StaticGroup::new(id, record.name.clone(), Vec::new());
            building.byte_18 = record.byte_18;
            for handle in record.members {
                let Some(object) = host.view_of_handle_b364c0(handle)? else {
                    return Err(format!(
                        "0x439D09 assertion (line 0x344): Kingdom+0x34C group member handle {handle:#x} does not resolve"
                    ));
                };
                let Some(view) = object else { continue };
                // 0x453520: 0x452A90(old group, view, 1) when +0x150 is set.
                if let Some(old) = self.group_of_150(view) {
                    let removed = if old == id {
                        building.members.iter().position(|m| *m == view).map(|at| {
                            building.members.remove(at);
                        })
                    } else {
                        self.group_index(old).and_then(|index| {
                            let members = &mut self.groups[index].members;
                            members.iter().position(|m| *m == view).map(|at| {
                                members.remove(at);
                            })
                        })
                    };
                    if removed.is_some() {
                        self.unlink_view_451860(host, view)?;
                    }
                }
                self.links_150.retain(|(v, _)| *v != view);
                self.links_150.push((view, id));
                host.floater_update_453470(view)?;
                let at = building.members.partition_point(|m| *m < view);
                building.members.insert(at, view);
            }
            if !building.members.is_empty() {
                self.groups.push(building);
            }
        }
        self.wildcard_44.byte_18 = host.player_wildcard_byte_50()?;
        self.refresh_static_4374d0(host)
    }
    /// 0x438560(name, byte) (the simulation's 0x50C410 after its 0x4F5290
    /// local-kingdom, [B36534]+D8 != 6 and 0x43AAF0 gates): the current
    /// group +0x40 when its name equals `name` (wcscmp) gets +0x18 = `byte`
    /// and 0x437EB0; otherwise the +0x34 group 0x4375F0 finds by name gets
    /// +0x18 = `byte` alone.
    pub fn set_group_byte_438560<H: Host<View = V>>(&mut self, host: &mut H, name: &[u16], byte: bool) -> Result<()> {
        let current = match self.static_40 {
            StaticRef::None => None,
            StaticRef::Wildcard => Some(&mut self.wildcard_44),
            StaticRef::Group(i) => Some(self.groups.get_mut(i).ok_or("UIWorldState+0x40 names no +0x34 group")?),
        };
        if let Some(group) = current {
            if group.name == name {
                group.byte_18 = byte;
                return self.refresh_masks_437eb0(host);
            }
        }
        if let Some(group) = self.groups.iter_mut().find(|g| g.name == name) {
            group.byte_18 = byte;
        }
        Ok(())
    }
    /// UIWorldState 0x439E90 SaveGroup(name), reached from the console
    /// command KKC_GameInterface::CommandSaveGroup 0x60D8F0 (after its
    /// 0x5C0E30 local-kingdom gate and the empty-name diagnostic).
    pub fn save_group_439e90<H: Host<View = V>>(&mut self, host: &mut H, name: &[u16]) -> Result<()>
    where
        V: Ord,
    {
        let flag = self.current_byte_18();
        let mut group: Option<u64> = None;
        if let Some(index) = self.groups.iter().position(|g| g.name == name) {
            let id = self.groups[index].id;
            group = Some(id);
            self.rebuilding_48 = Some(id);
            for member in self.groups[index].members.clone() {
                self.unlink_view_451860(host, member)?;
            }
            let index = self.group_index(id).ok_or("rebuilt group vanished")?;
            let g = &mut self.groups[index];
            g.members.clear();
            g.byte_18 = true;
            g.word_1c = 0;
        }
        for view in self.selected.clone() {
            if !host.actor_45b7a0(view)? || !host.class_78(view)? {
                continue;
            }
            let id = match group {
                Some(id) => id,
                None => {
                    let id = self.next_group_id;
                    self.next_group_id += 1;
                    self.groups.push(StaticGroup::new(id, name.to_vec(), Vec::new()));
                    group = Some(id);
                    id
                }
            };
            // 0x453520: leave the old group (0x452A90 with release), link.
            if let Some(old) = self.group_of_150(view) {
                if let Some(index) = self.group_index(old) {
                    if let Some(at) = self.groups[index].members.iter().position(|m| *m == view) {
                        self.groups[index].members.remove(at);
                        self.unlink_view_451860(host, view)?;
                    }
                }
            }
            self.links_150.retain(|(v, _)| *v != view);
            self.links_150.push((view, id));
            host.floater_update_453470(view)?;
            // 0x869D30 lower bound, then the shifted insert.
            let index = self.group_index(id).ok_or("SaveGroup target group vanished")?;
            let members = &mut self.groups[index].members;
            let at = members.partition_point(|m| *m < view);
            members.insert(at, view);
        }
        if let Some(id) = group {
            if self.group_index(id).map(|i| self.groups[i].members.is_empty()).unwrap_or(true) {
                self.delete_group_4384a0(id);
                group = None;
            }
        }
        if let Some(id) = group {
            let index = self.group_index(id).ok_or("SaveGroup group vanished")?;
            self.groups[index].byte_18 = flag;
        }
        host.requests().save_group_438180(name, None)?;
        self.static_40 = match group.and_then(|id| self.group_index(id)) {
            Some(index) => StaticRef::Group(index),
            None => StaticRef::None,
        };
        self.rebuilding_48 = None;
        self.refresh_masks_437eb0(host)?;
        self.refresh_static_4374d0(host)
    }
    /// UIWorldState 0x4376C0 SelectGroup(name): clear (with its request),
    /// select every member in array order, then the 0x436850 acknowledgement
    /// of vt+0 (0x436600).
    pub fn select_group_4376c0<H: Host<View = V>>(&mut self, host: &mut H, name: &[u16]) -> Result<()> {
        self.clear(host, false)?;
        let Some(index) = self.groups.iter().position(|g| g.name == name) else { return Ok(()) };
        for member in self.groups[index].members.clone() {
            self.select(host, member, false)?;
        }
        if let Some(voiced) = self.first_voiced_436600(host)? {
            host.selection_sound_436850(voiced)?;
        }
        Ok(())
    }
    /// 0x437E40: the current group +0x40 is a +0x34 group named `name`
    /// (the double-tap test of CommandSelectGroup 0x610490).
    pub fn current_group_is_437e40(&self, name: &[u16]) -> bool {
        match self.static_40 {
            StaticRef::Group(i) => self.groups.get(i).is_some_and(|g| g.name == name),
            StaticRef::Wildcard => self.wildcard_44.name == name,
            StaticRef::None => false,
        }
    }

    // ---- hover ----

    /// vt+0x34 = 0x4363C0.
    pub fn clear_hover_4363c0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        if let Some(view) = self.hovered {
            self.highlight_4365d0(host, view, false)?;
            self.hovered = None;
        }
        Ok(())
    }
    /// vt+0x38 = 0x436650(x, y), called each frame by the base UI state
    /// update 0x5CD6B0 with the cursor tracker UI+0x58 +4/+8.
    pub fn update_hover_436650<H: Host<View = V>>(&mut self, host: &mut H, x: i32, y: i32) -> Result<()> {
        let mut target = None;
        if host.hover_enabled_5c1e70()? {
            if let Some(view) = host.pick_under_cursor_43dc70(x, y)? {
                let mapped = Self::pick_target_6409a0(host, Some(view))?;
                if Self::pick_filter_5c19e0(host, mapped)? {
                    target = Some(view);
                }
            }
        }
        if self.hovered != target {
            if let Some(old) = self.hovered {
                self.highlight_4365d0(host, old, false)?;
            }
            if let Some(new) = target {
                self.highlight_4365d0(host, new, true)?;
            }
            self.hovered = target;
        }
        Ok(())
    }
    /// 0x4363B0: vt+0x54 of the hovered object.
    pub fn hovered_target_4363b0<H: Leaves<View = V>>(&self, host: &mut H) -> Result<Option<V>> {
        Self::pick_target_6409a0(host, self.hovered)
    }
    /// vt+0x18 = 0x436950(&cmd), CanSend of a selected-actors command:
    /// with a static group +0x40 and an explicit command (flag bit 6) whose
    /// definition +0x0C is 0x25 / 0x26, the answer is `group+0x18 == 0` /
    /// `group+0x18`; otherwise the first selected 0x45B7A0 actor whose
    /// vt+0x150 holds makes it true. Definition +0x0C is the ActorCommand
    /// registration index (0x4B4E60), which the wire carries as `command`.
    pub fn can_send_436950<H: Leaves<View = V>>(&self, host: &mut H, wire: &mut CommandWire) -> Result<bool> {
        let group = match self.static_40 {
            StaticRef::None => None,
            StaticRef::Wildcard => Some(&self.wildcard_44),
            StaticRef::Group(i) => Some(self.groups.get(i).ok_or("UIWorldState+0x40 names a released group")?),
        };
        if let Some(group) = group {
            if wire.flags & 0x40 != 0 {
                match wire.command {
                    Some(0x25) => return Ok(!group.byte_18),
                    Some(0x26) => return Ok(group.byte_18),
                    _ => {}
                }
            }
        }
        for view in self.selected.clone() {
            if !host.actor_45b7a0(view)? {
                continue;
            }
            if host.actor_virtual_150(view, wire)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// vt+0x68 = 0x4365D0.
    pub fn highlight_4365d0<H: Host<View = V>>(&mut self, host: &mut H, view: V, on: bool) -> Result<()> {
        let flags = host.flags_54(view)?;
        host.set_flags_54(view, if on { flags | view_flag::HIGHLIGHT } else { flags & !view_flag::HIGHLIGHT })
    }
    /// vt+0x4C = 0x436D70: keep a single selected object that is among the
    /// screen-point hits, else `fallback`.
    pub fn hover_choice_436d70(&self, hits: &[V], fallback: Option<V>) -> Option<V> {
        match self.single() {
            Some(single) if hits.contains(&single) => Some(single),
            _ => fallback,
        }
    }

    // ---- clicks ----

    /// vt+0x3C = 0x640CF0(origin, direction, double): plain left click.
    pub fn click_640cf0<H: Host<View = V>>(&mut self, host: &mut H, origin: [u32; 3], direction: [u32; 3], double: bool) -> Result<()> {
        if double {
            if let Some(head) = self.selected.first().copied() {
                if host.class_78(head)? {
                    let list = Self::same_definition_640bf0(host, Some(head))?;
                    if !list.is_empty() {
                        self.select_list_640fa0(host, &list)?;
                    }
                    return Ok(());
                }
            }
        }
        let mut hits = Vec::new();
        let hit = host.find_object_at_43d240(origin, direction, Some(&mut hits))?;
        self.click_hits_437b20(host, origin, &hits, hit)
    }
    /// vt+0x40 = 0x640E10(origin, direction, double): Shift/Ctrl click.
    pub fn click_add_640e10<H: Host<View = V>>(&mut self, host: &mut H, origin: [u32; 3], direction: [u32; 3], double: bool) -> Result<()> {
        if !double {
            // 0x436730
            let Some(hit) = host.find_object_at_43d240(origin, direction, None)? else { return Ok(()) };
            let owner = Self::select_owner_640b40(host, Some(hit))?
                .ok_or("0x436775 reads +0x54 of a NULL vt+0x58 result (native NULL dereference)")?;
            if host.flags_54(owner)? & view_flag::SELECTED != 0 {
                return self.deselect_438df0(host, owner);
            }
            if self.compatible_640b20(host, owner)? {
                self.select(host, owner, true)?;
            }
            return Ok(());
        }
        let Some(hit) = host.find_object_at_43d240(origin, direction, None)? else { return Ok(()) };
        if self.selected.contains(&hit) {
            let list = Self::same_definition_640bf0(host, Some(hit))?;
            if !list.is_empty() {
                self.select_list_640fa0(host, &list)?;
            }
            return Ok(());
        }
        let field = host.field_10_58cb30(hit)?;
        for view in self.selected.clone() {
            if host.field_10_58cb30(view)? == field {
                self.deselect_438df0(host, view)?;
            }
        }
        Ok(())
    }
    /// vt+0x50 = 0x437B20(origin, hits, hit): click selection that cycles to
    /// the nearest hit behind a single selected object hit again.
    pub fn click_hits_437b20<H: Host<View = V>>(&mut self, host: &mut H, origin: [u32; 3], hits: &[V], hit: Option<V>) -> Result<()> {
        let Some(selected) = self.single().filter(|s| hits.contains(s)) else {
            self.clear(host, false)?;
            if let Some(hit) = hit {
                self.select(host, hit, true)?;
            }
            return Ok(());
        };
        let p = host.precision();
        let selected_distance = st(&x87::distance_436480(origin, host.position_38(selected)?, p)?)?;
        let mut behind: Vec<V> = Vec::new();
        for other in hits.iter().copied() {
            if other == selected {
                continue;
            }
            let distance = st(&x87::distance_436480(origin, host.position_38(other)?, p)?)?;
            if host.choose_5c1970(Some(selected), selected_distance, Some(other), distance)? == Some(selected) {
                behind.insert(0, other);
            }
        }
        if behind.is_empty() {
            self.clear(host, false)?;
            if let Some(hit) = hit {
                self.select(host, hit, true)?;
            }
            return Ok(());
        }
        let mut best = None;
        let mut best_distance = 0x7f7f_ffff;
        for other in behind {
            let distance = st(&x87::distance_436480(origin, host.position_38(other)?, p)?)?;
            if host.choose_5c1970(best, best_distance, Some(other), distance)? == Some(other) {
                best = Some(other);
                best_distance = distance;
            }
        }
        self.clear(host, false)?;
        if let Some(best) = best {
            self.select(host, best, true)?;
        }
        Ok(())
    }

    // ---- box ----

    /// 0x436460: clear, then vt+0x48 (the unmodified box release 0x5CCB70).
    pub fn box_replace_436460<H: Host<View = V>>(&mut self, host: &mut H, start: &Event, end: &Event) -> Result<()> {
        self.clear(host, false)?;
        self.box_select_4377b0(host, start, end)
    }
    /// vt+0x48 = 0x4377B0(start, end).
    pub fn box_select_4377b0<H: Host<View = V>>(&mut self, host: &mut H, start: &Event, end: &Event) -> Result<()> {
        // 0x436DD0 -> 0x4367A0
        host.box_frustum_436dd0(start, end)?;
        self.gather_4367a0(host)?;
        if let Some(hovered) = Self::pick_target_6409a0(host, self.hovered)? {
            if host.flags_54(hovered)? & view_flag::CANDIDATE == 0 {
                self.candidate_640a40(host, hovered)?;
            }
        }
        let list = self.candidates.clone();
        self.select_list_640fa0(host, &list)?;
        if let Some(voiced) = self.first_voiced_436600(host)? {
            host.selection_sound_436850(voiced)?;
        }
        self.clear_candidates_6406b0(host)
    }
    /// 0x4367A0: walk the VGrid list adding each object inside the planes the
    /// host built (0x436DD0 or 0x4371A0) through vt+0x54 / 0x5C19E0 / vt+0x60.
    fn gather_4367a0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        for view in host.vworld_list_94()? {
            if !host.box_inside_4367a0(view)? {
                continue;
            }
            let mapped = Self::pick_target_6409a0(host, Some(view))?;
            if mapped.is_some_and(|m| self.candidates.contains(&m)) {
                continue;
            }
            if !Self::pick_filter_5c19e0(host, mapped)? {
                continue;
            }
            let mapped = mapped.ok_or("0x436825 vt+0x60 of a NULL object")?;
            self.candidate_640a40(host, mapped)?;
        }
        Ok(())
    }
    /// 0x437DC0 -> 0x4371A0(press x, press y, cursor x, cursor y): the drag
    /// box per-frame candidate preview (box state vt+8 0x5CC770).
    pub fn box_preview_4371a0<H: Host<View = V>>(&mut self, host: &mut H, press: [i32; 2], cursor: [i32; 2]) -> Result<()> {
        self.clear_candidates_6406b0(host)?;
        host.box_frustum_4371a0([press[0], press[1], cursor[0], cursor[1]])?;
        self.gather_4367a0(host)
    }
    /// vt+0x60 = 0x640A40.
    pub fn candidate_640a40<H: Host<View = V>>(&mut self, host: &mut H, view: V) -> Result<()> {
        if host.flags_54(view)? & view_flag::CANDIDATE != 0 {
            return Ok(());
        }
        let class = Self::class_6406c0(host, view, self.candidate_class_50)?;
        if class == 0 {
            return Ok(());
        }
        if (class as i32) > (self.candidate_class_50 as i32) && class & 0x93 == 0 {
            self.clear_candidates_6406b0(host)?;
        }
        self.candidate_class_50 = class;
        // 0x437490
        self.candidates.insert(0, view);
        let flags = host.flags_54(view)?;
        host.set_flags_54(view, flags | view_flag::CANDIDATE)
    }
    /// vt+0x44 = 0x6406B0: +0x50 = 0, then 0x437DE0.
    pub fn clear_candidates_6406b0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        self.candidate_class_50 = 0;
        if host.vworld_ad()? {
            for view in self.candidates.clone() {
                let flags = host.flags_54(view)?;
                host.set_flags_54(view, flags & !view_flag::CANDIDATE)?;
            }
        }
        self.candidates.clear();
        Ok(())
    }
    /// vt+0x64 = 0x640B20.
    pub fn compatible_640b20<H: Host<View = V>>(&self, host: &mut H, view: V) -> Result<bool> {
        if self.selection_class_54 == 0 {
            return Ok(true);
        }
        Ok(Self::class_6406c0(host, view, self.selection_class_54)? != 0)
    }
    /// Base vt+0x6C = 0x4368C0.
    pub fn select_list_4368c0<H: Host<View = V>>(&mut self, host: &mut H, list: &[V]) -> Result<()> {
        for view in list.iter().copied() {
            if host.flags_54(view)? & view_flag::SELECTED != 0 {
                continue;
            }
            if self.compatible_640b20(host, view)? {
                self.select(host, view, false)?;
            }
        }
        Ok(())
    }
    /// Game vt+0x6C = 0x640FA0: the six-object cap. When the list and the
    /// selection together exceed six, the `6 - selected` compatible objects
    /// nearest (XY) to the centroid 0x640A90 of the selection (or of the list
    /// when nothing is selected) are selected.
    pub fn select_list_640fa0<H: Host<View = V>>(&mut self, host: &mut H, list: &[V]) -> Result<()> {
        let count = self.selected.len();
        if count >= 6 {
            return Ok(());
        }
        if list.len() + count <= 6 {
            return self.select_list_4368c0(host, list);
        }
        let p = host.precision();
        let source = if count > 0 { self.selected.clone() } else { list.to_vec() };
        let centroid = centroid_640a90(host, &source, p)?;
        let slots = 6 - count;
        // 0x640840 array of {view, d2 binary32}, capacity `slots`.
        let mut best: Vec<(V, u32)> = Vec::with_capacity(slots);
        for view in list.iter().copied() {
            if host.flags_54(view)? & view_flag::SELECTED != 0 {
                continue;
            }
            if !self.compatible_640b20(host, view)? {
                continue;
            }
            let position = host.position_38(view)?;
            let dx = st(&x87::sub(&b(position[0])?, &b(centroid[0])?, p)?)?;
            let dy = st(&x87::sub(&b(position[1])?, &b(centroid[1])?, p)?)?;
            let d2 = x87::add(&x87::mul(&b(dy)?, &b(dy)?, p)?, &x87::mul(&b(dx)?, &b(dx)?, p)?, p)?;
            if best.len() < slots {
                best.push((view, st(&d2)?));
                continue;
            }
            // The first strictly largest entry (0x641110..0x64112C).
            let mut worst = 0;
            for index in 1..slots {
                if !x87::le(&b(best[index].1)?, &b(best[worst].1)?) {
                    worst = index;
                }
            }
            if x87::lt(&d2, &b(best[worst].1)?) {
                best[worst] = (view, st(&d2)?);
            }
        }
        // 0x641181: head insertion of entries 0..n-1, so the last entry
        // becomes the head of the list 0x4368C0 walks.
        let chosen: Vec<V> = best.iter().rev().map(|(v, _)| *v).collect();
        self.select_list_4368c0(host, &chosen)
    }
    /// 0x640BF0(view): every pickable object of the VWorld+0x94 list whose
    /// +0x10 equals `view`'s, head-inserted (reverse list order).
    pub fn same_definition_640bf0<H: Host<View = V>>(host: &mut H, view: Option<V>) -> Result<Vec<V>> {
        let Some(view) = view else { return Ok(Vec::new()) };
        let field = host.field_10_58cb30(view)?;
        let mut list = Vec::new();
        for other in host.vworld_list_94()? {
            if !host.pickable_9c(other)? {
                continue;
            }
            if host.field_10_58cb30(other)? == field {
                list.insert(0, other);
            }
        }
        Ok(list)
    }
}

/// 0x640A90: XY centroid. Both sums stay on the FPU stack; each is stored
/// binary32, then multiplied by `1.0 / count` (0x9F32E0 / fild) and stored.
pub fn centroid_640a90<H: Leaves>(host: &mut H, list: &[H::View], p: Precision) -> Result<[u32; 2]> {
    let mut sum_x = b(0)?;
    let mut sum_y = b(0)?;
    for view in list {
        let position = host.position_38(*view)?;
        sum_x = x87::add(&sum_x, &b(position[0])?, p)?;
        sum_y = x87::add(&sum_y, &b(position[1])?, p)?;
    }
    let (sum_x, sum_y) = (st(&sum_x)?, st(&sum_y)?);
    let count = i32::try_from(list.len()).map_err(|_| "0x640A90 count exceeds the native int")?;
    if count == 0 {
        return Err("0x640AE8 divides 1.0 by a zero count (reached only with an empty source)".into());
    }
    let inverse = x87::div(&b(0x3f80_0000)?, &aa_numeric::Binary::from_i32(count), p)?;
    let x = st(&x87::mul(&b(sum_x)?, &inverse, p)?)?;
    let y = st(&x87::mul(&inverse, &b(sum_y)?, p)?)?;
    Ok([x, y])
}

#[cfg(test)]
#[path = "ui_world_tests.rs"]
mod tests;
