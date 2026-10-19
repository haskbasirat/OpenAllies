//! In-battle selection and ordering input, ported from AA.exe.
//!
//! Native pipeline (all addresses are AA.exe, image base 0x400000):
//!
//! * `WndProc` 0x6FD4A0 turns WM_xBUTTONDOWN/UP/DBLCLK/MOUSEWHEEL into
//!   `KKC_Input::AddMouseEvent` 0x59A6E0 (codes 0x100 left, 0x101 right,
//!   0x102 middle, +8 double click, 0x110 wheel; flags 1 pressed, 2 released).
//!   The event carries the camera ray (0x5C2770) and the terrain hit point
//!   (0x71D840) of its pixel ([`event`]).
//! * The UI manager pump 0x5C3BC0 hit-tests widgets on presses and calls
//!   0x5C1B80, which offers the event to the modal UI-state stack (UI+0xB4),
//!   then the single slots UI+0xBC (brush/targeting), +0xC0 and +0xC4 (the
//!   game default state installed by KKC_GameInterface 0x60BAE0) ([`states`]).
//! * The game UIWorldState (ctor 0x640640 over base 0x438E10) is the UI
//!   selection manager ([`ui_world`]); each change emits Selection*Order
//!   requests 12/13/14 ([`requests`]), executed by the simulation as decrees.
//! * Right click builds a KKC_GActorCommand (0x4D9C80) and sends
//!   KKC_TellSelectedCommandOrderRequest 84 ([`command`]).
pub mod command;
pub mod command_mask_world;
pub mod command_masks;
pub mod event;
pub mod order_world;
pub mod pick;
pub mod requests;
pub mod states;
pub mod terrain_ray;
pub mod terrain_map;
pub mod ui_world;
pub mod x87;
pub mod x87_masked;
