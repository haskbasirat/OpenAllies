//! In-battle selection and ordering input: the client side of
//! `aa_runtime::player_order_input` (ported from AA.exe's WndProc 0x6FD4A0,
//! KKC_Input 0x59A5C0/0x59A6E0, the UI manager dispatch 0x5C1B80, the game
//! UI states and the game UIWorldState).
//!
//! UI thread: [`observe`] turns this frame's egui pointer/modifier events into
//! the native window messages (WM_L/RBUTTONDOWN/UP/DBLCLK and the modifier
//! keys) in native client pixels of the presented battle image. Only that
//! Send packet crosses threads.
//!
//! Simulation thread: [`Orders::frame`] runs one native UI frame of world
//! input (0x5C92F0's tracker / vt+0x10, each event through 0x59A6E0 +
//! 0x5C3BC0 + 0x5C1B80, then vt+8) against [`Live`], which binds the actual
//! owners: the AdminNode request stream of the activated session (selection
//! requests 12/13/14/15 and TellSelectedCommand 84), the current native
//! camera and the view/actor/HUD leaves supplied through [`ViewLeaves`] and
//! [`HudLeaves`]. A leaf without a live owner is an explicit error naming the
//! native field it stands for; nothing is defaulted.
//!
//! Middle button and wheel are bound by hotkeys_core.txt on the `vworld`
//! widget (`+button3`, `mousewheel`), whose hotkey chain (0x5EFC70) runs before
//! the UI states, so they belong to the camera input (battle_camera_input.rs)
//! and are not produced here.
#![allow(dead_code)] // entry points: called from the game screen's input loop
use aa_orders::payload::{Registry, SchemaContext};
use aa_runtime::player_order_input::{
    command::Orders as NativeOrders,
    event::{self, code, flag, Event},
    pick::{Model, Scene},
    requests::{AdminRequests, Counts, Requests},
    states::{self, CameraSlot, CommandDefinition, Machine, Widget},
    ui_world::{Leaves, StaticRef},
};
use aa_orders::commands::CommandWire;
use aa_numeric::Precision;
use eframe::egui;

pub type Result<T> = std::result::Result<T, String>;
pub type View = crate::live_scene::view_directory::ViewHandle;

/// Windows' default double-click time and rectangle (GetDoubleClickTime,
/// SM_CXDOUBLECLK/SM_CYDOUBLECLK), the same rule tgi_frontend.rs applies:
/// the window class has CS_DBLCLKS (0x6FB2DD), so the second press of a
/// button inside both limits arrives as WM_xBUTTONDBLCLK.
const DOUBLE_CLICK_MS: f64 = 500.0;
const DOUBLE_CLICK_HALF: i32 = 2;

/// One observed window message, already mapped to the native client area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// WM_KEYDOWN/UP of VK_SHIFT/VK_CONTROL/VK_MENU (0x59A5C0's modifiers).
    Modifier { vk: u16, released: bool },
    /// A button message: `code` 0x100/0x101 (+8 double click), `flags` 1
    /// press / 2 release, LOWORD/HIWORD pixel (releases clamped by 0x40D280).
    Button { code: u16, flags: u32, x: i16, y: i16, widget: Widget },
}

/// One UI frame (Send).
#[derive(Clone, Debug, Default)]
pub struct Observed {
    pub messages: Vec<Message>,
    /// Input+0xC/+0x10: the cursor in native client pixels.
    pub cursor: Option<[i32; 2]>,
    pub client: [u32; 2],
    /// The caller's verdict for presses: false while a HUD widget is under
    /// the cursor (the HUD owner decides; UI+0x60 is then not VWorld).
    pub world_under_cursor: bool,
}

/// Double-click and modifier tracking of the window (UI thread).
#[derive(Clone, Debug, Default)]
pub struct Window {
    last_press: Option<(egui::PointerButton, f64, [i32; 2])>,
    modifiers: [bool; 3],
}

fn native_pixel(image: egui::Rect, pixels: [u32; 2], p: egui::Pos2) -> [i32; 2] {
    let x = (p.x - image.min.x) * pixels[0] as f32 / image.width();
    let y = (p.y - image.min.y) * pixels[1] as f32 / image.height();
    [x.floor() as i32, y.floor() as i32]
}

/// Collect this frame's world-input messages. `image` is where the native
/// battle target is drawn and `pixels` its native size (as camera input).
pub fn observe(ctx: &egui::Context, window: &mut Window, image: egui::Rect, pixels: [u32; 2], world_under_cursor: bool) -> Observed {
    ctx.input(|i| {
        let mut messages = Vec::new();
        let valid = image.width() > 0.0 && image.height() > 0.0;
        let sync = |m: &egui::Modifiers, window: &mut Window, messages: &mut Vec<Message>| {
            // VK_SHIFT, VK_CONTROL, VK_MENU in the order WM_KEYDOWNs arrive.
            for (k, (vk, down)) in [(0x10u16, m.shift), (0x11, m.ctrl), (0x12, m.alt)].into_iter().enumerate() {
                if window.modifiers[k] != down {
                    window.modifiers[k] = down;
                    messages.push(Message::Modifier { vk, released: !down });
                }
            }
        };
        for e in &i.events {
            match e {
                egui::Event::Key { modifiers, .. } => sync(modifiers, window, &mut messages),
                egui::Event::PointerButton { pos, button, pressed, modifiers } if valid => {
                    let base = match button {
                        egui::PointerButton::Primary => code::LEFT,
                        egui::PointerButton::Secondary => code::RIGHT,
                        _ => continue,
                    };
                    sync(modifiers, window, &mut messages);
                    let p = native_pixel(image, pixels, *pos);
                    let widget = if world_under_cursor { Widget::World } else { Widget::Other };
                    if *pressed {
                        let now = i.time * 1000.0;
                        let double = window.last_press.is_some_and(|(b, t, q)| {
                            b == *button && now - t <= DOUBLE_CLICK_MS
                                && (p[0] - q[0]).abs() <= DOUBLE_CLICK_HALF && (p[1] - q[1]).abs() <= DOUBLE_CLICK_HALF
                        });
                        let code = if double { base + 8 } else { base };
                        window.last_press = if double { None } else { Some((*button, now, p)) };
                        messages.push(Message::Button { code, flags: flag::PRESSED, x: p[0] as i16, y: p[1] as i16, widget });
                    } else {
                        // 0x6FD3D0: ReleaseCapture, 0x40D280 clamp, flags 2.
                        let c = event::clamp_release_40d280(p, [pixels[0] as i32, pixels[1] as i32]);
                        messages.push(Message::Button { code: base, flags: flag::RELEASED, x: c[0] as i16, y: c[1] as i16, widget });
                    }
                }
                _ => {}
            }
        }
        sync(&i.modifiers, window, &mut messages);
        let cursor = i.pointer.latest_pos().filter(|_| valid).map(|p| native_pixel(image, pixels, p));
        Observed { messages, cursor, client: pixels, world_under_cursor }
    })
}

/// The per-view and per-actor leaves the native selection manager and pick
/// path read. Their owner is the view directory (crates/aa-client/src/
/// view_directory.rs and the VGrid owner); [`UnboundViews`] is the explicit
/// "not bound" implementation until it provides them.
pub trait ViewLeaves {
    fn flags_54(&mut self, v: View) -> Result<u32>;
    fn set_flags_54(&mut self, v: View, f: u32) -> Result<()>;
    fn position_38(&mut self, v: View) -> Result<[u32; 3]>;
    fn field_10_58cb30(&mut self, v: View) -> Result<u64>;
    fn pickable_9c(&mut self, v: View) -> Result<bool>;
    fn class_78(&mut self, v: View) -> Result<bool>;
    fn class_84(&mut self, v: View) -> Result<bool>;
    fn class_7c(&mut self, v: View) -> Result<bool>;
    fn class_80(&mut self, v: View) -> Result<bool>;
    fn virtual_50(&mut self, v: View) -> Result<bool>;
    fn virtual_54(&mut self, v: View) -> Result<bool>;
    fn actor_present_14(&mut self, v: View) -> Result<bool>;
    fn actor_45b7a0(&mut self, v: View) -> Result<bool>;
    fn actor_flag8_bit0(&mut self, v: View) -> Result<bool>;
    fn actor_virtual_58(&mut self, v: View) -> Result<bool>;
    fn actor_virtual_84(&mut self, v: View) -> Result<bool>;
    fn actor_4d12c0(&mut self, v: View) -> Result<bool>;
    fn actor_virtual_20(&mut self, v: View) -> Result<bool>;
    fn actor_70(&mut self, v: View) -> Result<bool>;
    fn actor_90(&mut self, v: View) -> Result<bool>;
    fn link_16c_view(&mut self, v: View) -> Result<Option<View>>;
    fn link_94_view(&mut self, v: View) -> Result<Option<View>>;
    fn link_94_live(&mut self, v: View) -> Result<bool>;
    fn hidden_4549c0(&mut self, v: View) -> Result<bool>;
    fn actor_view_0c(&mut self, v: View) -> Result<Option<View>>;
    fn root_view_4ce6c0(&mut self, v: View) -> Result<Option<View>>;
    fn voiced_436600(&mut self, v: View) -> Result<Option<Option<View>>>;
    /// `actor+0x14` slot (u16) for the selection requests (0x76A500).
    fn actor_slot_14(&mut self, v: View) -> Result<u16>;
    /// `actor+0x14` handle for the command target word (0x4D9BF0).
    fn actor_handle_14(&mut self, v: View) -> Result<u32>;
    fn actor_can_34(&mut self, v: View, index: u32) -> Result<bool>;
    fn on_list_100(&mut self) -> Result<Vec<View>>;
    fn proximity_18(&mut self, v: View) -> Result<i32>;
    fn model_78(&mut self, v: View) -> Result<Option<Model>>;
    fn selection_radius_469ac0(&mut self, v: View) -> Result<u32>;
    /// 0x86E8D0(node, VWorld+0x8C, 1) before 0x715D40's model-space test.
    fn update_model_86e8d0(&mut self, v: View) -> Result<Model>;
    fn floater_remove_42eb10(&mut self, v: View) -> Result<()>;
    fn floater_update_453470(&mut self, v: View) -> Result<()>;
    fn flash_459600(&mut self, v: View) -> Result<()>;
}

/// The HUD/widget, presentation and audio leaves (HUD, camera and audio
/// owners). [`UnboundHud`] reports each one as unbound.
pub trait HudLeaves {
    fn widget_route_5e0690(&mut self, e: &Event) -> Result<bool>;
    fn pressed_widget_chain_5efff0(&mut self, e: &Event) -> Result<bool>;
    fn widget_pick_64(&mut self, x: i32, y: i32) -> Result<Option<View>>;
    fn brush_installed_bc(&mut self) -> Result<bool>;
    fn camera_slot_c0(&mut self) -> Result<CameraSlot>;
    fn camera_frame_10(&mut self) -> Result<bool>;
    fn camera_frame_8(&mut self) -> Result<()>;
    fn map_world_5d74b0(&mut self, cursor: [i32; 2]) -> Result<[u32; 2]>;
    fn map_auxiliary_b279c0(&mut self) -> Result<[u32; 2]>;
    fn cursor_feedback_620640(&mut self) -> Result<()>;
    fn selection_sound_436850(&mut self, v: View) -> Result<()>;
    fn command_masks_437eb0(&mut self, selected: &[View], group: StaticRef, masks: &mut [u32; 4]) -> Result<()>;
    fn brush_changed_42d4d0(&mut self) -> Result<()>;
    fn vworld_list_changed_4325c0(&mut self) -> Result<()>;
    fn vworld_ad(&mut self) -> Result<bool>;
    fn drag_threshold_b39990(&mut self) -> Result<i32>;
    fn box_rectangle_begin(&mut self, press: &Event) -> Result<()>;
    fn box_rectangle_frame(&mut self) -> Result<()>;
    fn box_rectangle_end(&mut self) -> Result<()>;
    fn facing_begin_689380(&mut self, selected: &[View]) -> Result<()>;
    fn facing_drag_start(&mut self) -> Result<()>;
    fn facing_frame(&mut self, facing: [u32; 3], rotated: bool) -> Result<()>;
    fn facing_end_6885f0(&mut self) -> Result<()>;
    fn marker_43db10(&mut self, xy: [u32; 2]) -> Result<()>;
    fn acknowledge_61eef0(&mut self, wire: &CommandWire) -> Result<()>;
    /// UIWorldState vt+0x18 0x436950 over the selected actors' vt+0x150.
    fn can_send_436950(&mut self, wire: &mut CommandWire) -> Result<bool>;
    fn command_definition_298(&mut self, index: u32) -> Result<Option<CommandDefinition>>;
}

macro_rules! unbound {
    ($owner:literal; $($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret {
            $(let _ = $arg;)*
            Err(format!(concat!(stringify!($name), ": no live accessor yet (owner: ", $owner, ")")))
        })*
    };
}
/// No view owner bound: every leaf is an explicit error.
pub struct UnboundViews;
impl ViewLeaves for UnboundViews {
    unbound! { "crates/aa-client/src/view_directory.rs / VGrid";
        flags_54(v: View) -> Result<u32>;
        set_flags_54(v: View, f: u32) -> Result<()>;
        position_38(v: View) -> Result<[u32; 3]>;
        field_10_58cb30(v: View) -> Result<u64>;
        pickable_9c(v: View) -> Result<bool>;
        class_78(v: View) -> Result<bool>;
        class_84(v: View) -> Result<bool>;
        class_7c(v: View) -> Result<bool>;
        class_80(v: View) -> Result<bool>;
        virtual_50(v: View) -> Result<bool>;
        virtual_54(v: View) -> Result<bool>;
        actor_present_14(v: View) -> Result<bool>;
        actor_45b7a0(v: View) -> Result<bool>;
        actor_flag8_bit0(v: View) -> Result<bool>;
        actor_virtual_58(v: View) -> Result<bool>;
        actor_virtual_84(v: View) -> Result<bool>;
        actor_4d12c0(v: View) -> Result<bool>;
        actor_virtual_20(v: View) -> Result<bool>;
        actor_70(v: View) -> Result<bool>;
        actor_90(v: View) -> Result<bool>;
        link_16c_view(v: View) -> Result<Option<View>>;
        link_94_view(v: View) -> Result<Option<View>>;
        link_94_live(v: View) -> Result<bool>;
        hidden_4549c0(v: View) -> Result<bool>;
        actor_view_0c(v: View) -> Result<Option<View>>;
        root_view_4ce6c0(v: View) -> Result<Option<View>>;
        voiced_436600(v: View) -> Result<Option<Option<View>>>;
        actor_slot_14(v: View) -> Result<u16>;
        actor_handle_14(v: View) -> Result<u32>;
        actor_can_34(v: View, index: u32) -> Result<bool>;
        on_list_100() -> Result<Vec<View>>;
        proximity_18(v: View) -> Result<i32>;
        model_78(v: View) -> Result<Option<Model>>;
        selection_radius_469ac0(v: View) -> Result<u32>;
        update_model_86e8d0(v: View) -> Result<Model>;
        floater_remove_42eb10(v: View) -> Result<()>;
        floater_update_453470(v: View) -> Result<()>;
        flash_459600(v: View) -> Result<()>;
    }
}
/// No HUD owner bound: every leaf is an explicit error.
pub struct UnboundHud;
impl HudLeaves for UnboundHud {
    unbound! { "HUD/camera/audio owners";
        widget_route_5e0690(e: &Event) -> Result<bool>;
        pressed_widget_chain_5efff0(e: &Event) -> Result<bool>;
        widget_pick_64(x: i32, y: i32) -> Result<Option<View>>;
        brush_installed_bc() -> Result<bool>;
        camera_slot_c0() -> Result<CameraSlot>;
        camera_frame_10() -> Result<bool>;
        camera_frame_8() -> Result<()>;
        map_world_5d74b0(cursor: [i32; 2]) -> Result<[u32; 2]>;
        map_auxiliary_b279c0() -> Result<[u32; 2]>;
        cursor_feedback_620640() -> Result<()>;
        selection_sound_436850(v: View) -> Result<()>;
        command_masks_437eb0(selected: &[View], group: StaticRef, masks: &mut [u32; 4]) -> Result<()>;
        brush_changed_42d4d0() -> Result<()>;
        vworld_list_changed_4325c0() -> Result<()>;
        vworld_ad() -> Result<bool>;
        drag_threshold_b39990() -> Result<i32>;
        box_rectangle_begin(press: &Event) -> Result<()>;
        box_rectangle_frame() -> Result<()>;
        box_rectangle_end() -> Result<()>;
        facing_begin_689380(selected: &[View]) -> Result<()>;
        facing_drag_start() -> Result<()>;
        facing_frame(facing: [u32; 3], rotated: bool) -> Result<()>;
        facing_end_6885f0() -> Result<()>;
        marker_43db10(xy: [u32; 2]) -> Result<()>;
        acknowledge_61eef0(wire: &CommandWire) -> Result<()>;
        can_send_436950(wire: &mut CommandWire) -> Result<bool>;
        command_definition_298(index: u32) -> Result<Option<CommandDefinition>>;
    }
}

/// The live order-stream context: the ActorCommand registry is the 39
/// predefined commands registered before UI source loading (as
/// aa-netcode's local_game_data.rs proves); object definitions are not yet
/// reached by the UI paths.
pub struct OrderSchema;
impl SchemaContext for OrderSchema {
    fn registry_count(&self, registry: Registry) -> aa_orders::Result<usize> {
        if registry == aa_runtime::player_order_input::requests::ACTOR_COMMANDS {
            return Ok(aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len());
        }
        Err(aa_orders::Error::SchemaContext("UI order stream reached an unbound definition registry"))
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("UI order stream has no TeamCommand source"))
    }
    fn definition_kind(&self, _: Registry, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("UI order stream has no definition kind source"))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}

/// The battle TerrainMap B36518 for 0x71D840
/// (`aa_runtime::player_order_input::terrain_ray::intersect_71d840` over its
/// `Terrain` binding).
pub trait TerrainQuery {
    fn query_71d840(&mut self, eye: [u32; 3], direction: [u32; 3], secondary: Option<&mut [u32; 3]>, p: Precision)
        -> Result<aa_runtime::player_order_input::terrain_ray::Outcome>;
}

/// [`TerrainQuery`] over any native-layout TerrainMap binding: 0x71D840 with
/// margin 0, as both callers (0x59A6E0 with a NULL secondary point, 0x43C4B0
/// with its normal output) pass it.
pub struct TerrainRay<'t, T: aa_runtime::player_order_input::terrain_ray::Terrain>(pub &'t T);
impl<T: aa_runtime::player_order_input::terrain_ray::Terrain> TerrainQuery for TerrainRay<'_, T> {
    fn query_71d840(&mut self, eye: [u32; 3], direction: [u32; 3], secondary: Option<&mut [u32; 3]>, p: Precision)
        -> Result<aa_runtime::player_order_input::terrain_ray::Outcome> {
        aa_runtime::player_order_input::terrain_ray::intersect_71d840(self.0, eye, direction, secondary, 0, p)
    }
}

/// B43CF8..B43D00, the vector 0x59A6E0 copies into the event before 0x71D840,
/// read from the SAME retained seed owner the view constructors read
/// (`view_startup_inputs::...::seeds()`).
pub fn seed_b43cf8(seeds: &aa_runtime::view_grid_startup::Seeds, catalog: &aa_runtime::catalog::Catalog<'_>) -> Result<[u32; 3]> {
    use aa_runtime::view_grid_state::Globals;
    let mut globals = seeds.bind(catalog)?;
    Ok([globals.word(0xb43cf8)?, globals.word(0xb43cfc)?, globals.word(0xb43d00)?])
}

/// The live owners for one frame (simulation thread).
pub struct Live<'a> {
    pub views: &'a mut dyn ViewLeaves,
    pub hud: &'a mut dyn HudLeaves,
    /// The SAME activated AdminNode, its Admin+0x2C0 running flag and Node+8.
    pub admin: &'a mut aa_orders::net::admin::AdminNode,
    pub running_2c0: bool,
    pub local_player: Option<u32>,
    /// `*B40048`: the current native camera (normalized directory state).
    pub camera: Option<&'a aa_runtime::native_camera::State>,
    /// B396D0/B396D4.
    pub viewport: [i32; 2],
    /// B396C0 (aa-netcode Notifications application kind; 3 drops input).
    pub application_kind: u32,
    /// `B36518 != 0` and `+0xBC`.
    pub terrain_ready: bool,
    /// 0x71D840 against the battle TerrainMap (margin 0); None = not bound.
    pub terrain: Option<&'a mut dyn TerrainQuery>,
    /// B43CF8: the process seed vector.
    pub seed_b43cf8: Option<[u32; 3]>,
    pub precision: Precision,
}
struct Emitter<'a, 'b> {
    live: &'b mut Live<'a>,
}
impl Requests<View> for Emitter<'_, '_> {
    fn add_436ad0(&mut self, view: View, player: Option<u32>) -> Result<()> {
        let slot = self.live.views.actor_slot_14(view)?;
        self.requests(|r| r.add_436ad0(slot, player))
    }
    fn remove_436b90(&mut self, view: View, player: Option<u32>) -> Result<()> {
        let slot = self.live.views.actor_slot_14(view)?;
        self.requests(|r| r.remove_436b90(slot, player))
    }
    fn clear_436c50(&mut self, player: Option<u32>) -> Result<()> {
        self.requests(|r| Requests::<u16>::clear_436c50(r, player))
    }
    fn save_group_438180(&mut self, name: &[u16], player: Option<u32>) -> Result<()> {
        self.requests(|r| Requests::<u16>::save_group_438180(r, name, player))
    }
}
impl Emitter<'_, '_> {
    fn requests<R>(&mut self, call: impl FnOnce(&mut AdminRequests<'_, fn(u16) -> Result<u16>>) -> Result<R>) -> Result<R> {
        let mut r = AdminRequests {
            admin: &mut *self.live.admin,
            running_2c0: self.live.running_2c0,
            local_player: self.live.local_player,
            context: &OrderSchema,
            actor_slot: Ok::<u16, String> as fn(u16) -> Result<u16>,
        };
        call(&mut r)
    }
}

/// The per-frame host the native machine runs against.
pub struct Host<'a, 'b> {
    live: &'b mut Live<'a>,
}
impl<'a, 'b> Host<'a, 'b> {
    pub fn new(live: &'b mut Live<'a>) -> Self {
        Self { live }
    }
}
macro_rules! views {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret { self.live.views.$name($($arg),*) })*
    };
}
macro_rules! hud {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret { self.live.hud.$name($($arg),*) })*
    };
}
impl Leaves for Host<'_, '_> {
    type View = View;
    fn precision(&self) -> Precision {
        self.live.precision
    }
    views! {
        flags_54(v: View) -> Result<u32>;
        set_flags_54(v: View, f: u32) -> Result<()>;
        position_38(v: View) -> Result<[u32; 3]>;
        field_10_58cb30(v: View) -> Result<u64>;
        pickable_9c(v: View) -> Result<bool>;
        class_78(v: View) -> Result<bool>;
        class_84(v: View) -> Result<bool>;
        class_7c(v: View) -> Result<bool>;
        class_80(v: View) -> Result<bool>;
        virtual_50(v: View) -> Result<bool>;
        virtual_54(v: View) -> Result<bool>;
        actor_present_14(v: View) -> Result<bool>;
        actor_45b7a0(v: View) -> Result<bool>;
        actor_flag8_bit0(v: View) -> Result<bool>;
        actor_virtual_58(v: View) -> Result<bool>;
        actor_virtual_84(v: View) -> Result<bool>;
        actor_4d12c0(v: View) -> Result<bool>;
        actor_virtual_20(v: View) -> Result<bool>;
        actor_70(v: View) -> Result<bool>;
        actor_90(v: View) -> Result<bool>;
        link_16c_view(v: View) -> Result<Option<View>>;
        link_94_view(v: View) -> Result<Option<View>>;
        link_94_live(v: View) -> Result<bool>;
        hidden_4549c0(v: View) -> Result<bool>;
        actor_view_0c(v: View) -> Result<Option<View>>;
        root_view_4ce6c0(v: View) -> Result<Option<View>>;
        voiced_436600(v: View) -> Result<Option<Option<View>>>;
        floater_remove_42eb10(v: View) -> Result<()>;
        floater_update_453470(v: View) -> Result<()>;
    }
    hud! {
        selection_sound_436850(v: View) -> Result<()>;
        brush_changed_42d4d0() -> Result<()>;
        vworld_list_changed_4325c0() -> Result<()>;
        vworld_ad() -> Result<bool>;
    }
    fn command_masks_437eb0(&mut self, s: &[View], g: StaticRef, m: &mut [u32; 4]) -> Result<()> {
        self.live.hud.command_masks_437eb0(s, g, m)
    }
    fn requests(&mut self) -> &mut dyn Requests<View> {
        self
    }
}
/// `Host` is its own request emitter (the borrow of `live` is shared).
impl Requests<View> for Host<'_, '_> {
    fn add_436ad0(&mut self, view: View, player: Option<u32>) -> Result<()> {
        Emitter { live: &mut *self.live }.add_436ad0(view, player)
    }
    fn remove_436b90(&mut self, view: View, player: Option<u32>) -> Result<()> {
        Emitter { live: &mut *self.live }.remove_436b90(view, player)
    }
    fn clear_436c50(&mut self, player: Option<u32>) -> Result<()> {
        Emitter { live: &mut *self.live }.clear_436c50(player)
    }
    fn save_group_438180(&mut self, name: &[u16], player: Option<u32>) -> Result<()> {
        Emitter { live: &mut *self.live }.save_group_438180(name, player)
    }
}
impl Scene for Host<'_, '_> {
    type Camera = aa_runtime::native_camera::State;
    views! {
        on_list_100() -> Result<Vec<View>>;
        proximity_18(v: View) -> Result<i32>;
        model_78(v: View) -> Result<Option<Model>>;
        selection_radius_469ac0(v: View) -> Result<u32>;
        update_model_86e8d0(v: View) -> Result<Model>;
    }
    fn camera(&mut self) -> Result<&Self::Camera> {
        self.live.camera.ok_or_else(|| "*B40048 current native camera is not bound".into())
    }
    fn viewport_b396d0(&mut self) -> Result<(i32, i32)> {
        Ok((self.live.viewport[0], self.live.viewport[1]))
    }
}
impl NativeOrders for Host<'_, '_> {
    fn can_send_436950(&mut self, wire: &mut CommandWire) -> Result<bool> {
        self.live.hud.can_send_436950(wire)
    }
    fn acknowledge_61eef0(&mut self, wire: &CommandWire) -> Result<()> {
        self.live.hud.acknowledge_61eef0(wire)
    }
    fn emit_61e960(&mut self, wire: &CommandWire) -> Result<()> {
        let counts = Counts { commands: aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len(), objects: 0 };
        Emitter { live: &mut *self.live }.requests(|r| r.tell_selected_61e960(wire, counts, None)).map(|_| ())
    }
    fn diagnostic_61eef0(&mut self, wire: &CommandWire, sent: bool) -> Result<()> {
        if sent {
            log::info!("TellSelectedCommandOrder sent: flags {:#05x} word {:#010x} position {:08x?} auxiliary {:08x?}",
                wire.flags, wire.word, wire.position, wire.auxiliary);
        } else {
            log::info!("KKC_TellSelectedCommandOrder::Send order.CanSend() failed (flags {:#05x})", wire.flags);
        }
        Ok(())
    }
    fn marker_43db10(&mut self, xy: [u32; 2]) -> Result<()> {
        self.live.hud.marker_43db10(xy)
    }
}
impl states::Host for Host<'_, '_> {
    fn map_present_b36518(&mut self) -> Result<bool> {
        Ok(self.live.terrain_ready)
    }
    fn project_43c4b0(&mut self, pixel: [i32; 2], secondary: &mut [u32; 3]) -> Result<[u32; 3]> {
        let (width, height) = (self.live.viewport[0], self.live.viewport[1]);
        let precision = self.live.precision;
        let camera = self.live.camera.ok_or("*B40048 current native camera is not bound")?;
        let terrain = self.live.terrain.as_mut().ok_or("0x71D840 battle TerrainMap query is not bound")?;
        // 0x43C4B0 passes an uninitialized local point: it must be written.
        aa_runtime::view_pointer::project_43c4b0(camera, pixel, width, height, secondary, precision, |eye, dir, secondary| {
            terrain.query_71d840(eye, dir, Some(secondary), precision)?.point_words()
        })
    }
    hud! {
        widget_route_5e0690(e: &Event) -> Result<bool>;
        pressed_widget_chain_5efff0(e: &Event) -> Result<bool>;
        widget_pick_64(x: i32, y: i32) -> Result<Option<View>>;
        drag_threshold_b39990() -> Result<i32>;
        brush_installed_bc() -> Result<bool>;
        camera_slot_c0() -> Result<CameraSlot>;
        camera_frame_10() -> Result<bool>;
        camera_frame_8() -> Result<()>;
        map_world_5d74b0(cursor: [i32; 2]) -> Result<[u32; 2]>;
        map_auxiliary_b279c0() -> Result<[u32; 2]>;
        command_definition_298(index: u32) -> Result<Option<CommandDefinition>>;
        cursor_feedback_620640() -> Result<()>;
        box_rectangle_begin(press: &Event) -> Result<()>;
        box_rectangle_frame() -> Result<()>;
        box_rectangle_end() -> Result<()>;
        facing_begin_689380(selected: &[View]) -> Result<()>;
        facing_drag_start() -> Result<()>;
        facing_frame(facing: [u32; 3], rotated: bool) -> Result<()>;
        facing_end_6885f0() -> Result<()>;
    }
    views! {
        actor_can_34(v: View, index: u32) -> Result<bool>;
        actor_handle_14(v: View) -> Result<u32>;
        flash_459600(v: View) -> Result<()>;
    }
}
impl event::Sources for Host<'_, '_> {
    type Camera = aa_runtime::native_camera::State;
    fn input_block_b396c0(&mut self) -> Result<u32> {
        Ok(self.live.application_kind)
    }
    fn code_out_of_range(&mut self, c: u16) -> Result<()> {
        log::error!("KKC_Input::AddMouseEvent Mouse code {c} out of range 256-272.");
        Ok(())
    }
    fn camera(&mut self) -> Result<&Self::Camera> {
        self.live.camera.ok_or_else(|| "*B40048 current native camera is not bound".into())
    }
    fn viewport_b396d0(&mut self) -> Result<(i32, i32)> {
        Ok((self.live.viewport[0], self.live.viewport[1]))
    }
    fn seed_b43cf8(&mut self) -> Result<[u32; 3]> {
        self.live.seed_b43cf8.ok_or_else(|| "B43CF8 process seed vector is not bound".into())
    }
    fn terrain_ready_b36518(&mut self) -> Result<bool> {
        Ok(self.live.terrain_ready)
    }
    fn terrain_71d840(&mut self, eye: [u32; 3], direction: [u32; 3], point: &mut [u32; 3]) -> Result<()> {
        let precision = self.live.precision;
        let terrain = self.live.terrain.as_mut().ok_or("0x71D840 battle TerrainMap query is not bound")?;
        terrain.query_71d840(eye, direction, None, precision)?.apply_to(point);
        Ok(())
    }
}

/// What one frame did (for logs and the UI script's census).
#[derive(Clone, Debug, Default)]
pub struct FrameReport {
    pub events: usize,
    pub handled: usize,
    pub selected: usize,
    pub error: Option<String>,
}

/// The per-battle world-input owner (simulation thread, Send).
pub struct Orders {
    machine: Machine<View>,
}
impl Default for Orders {
    fn default() -> Self {
        Self::new()
    }
}
impl Orders {
    /// After KKC_GameInterface 0x60BAE0 activated the game interface.
    pub fn new() -> Self {
        Self { machine: Machine::for_game_interface_60bae0() }
    }
    pub fn machine(&self) -> &Machine<View> {
        &self.machine
    }
    /// One native UI frame of world input. The first failing leaf stops the
    /// frame and is reported; the machine keeps its native prefix state.
    pub fn frame(&mut self, observed: &Observed, live: &mut Live<'_>) -> FrameReport {
        let mut report = FrameReport::default();
        if let Err(error) = self.frame_inner(observed, live, &mut report) {
            report.error = Some(error);
        }
        report.selected = self.machine.selection().selected().len();
        report
    }
    fn frame_inner(&mut self, observed: &Observed, live: &mut Live<'_>, report: &mut FrameReport) -> Result<()> {
        if let Some(cursor) = observed.cursor {
            self.machine.set_cursor(cursor);
        }
        let mut host = Host::new(live);
        self.machine.frame_begin(&mut host)?;
        for message in &observed.messages {
            match *message {
                Message::Modifier { vk, released } => {
                    let flags = if released { flag::RELEASED } else { flag::PRESSED };
                    self.machine.input_mut().modifier_key_59a5c0(vk, flags);
                }
                Message::Button { code, flags, x, y, widget } => {
                    let Some(e) = self.machine.mouse_message(&mut host, code, flags, x, y, 0)? else { continue };
                    report.events += 1;
                    self.machine.pump_press_5c3bc0(&mut host, &e, widget)?;
                    self.machine.track_5bf2f0(&e);
                    // The root widget chain (HUD) declined presses over the
                    // world; over HUD widgets the HUD owns the event.
                    if widget == Widget::Other {
                        continue;
                    }
                    if self.machine.event(&mut host, &e)? {
                        report.handled += 1;
                    }
                }
            }
        }
        self.machine.frame_end(&mut host)
    }
}
