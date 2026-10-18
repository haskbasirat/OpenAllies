use eframe::egui;

/// Held camera directions: `[right - left, forward - backward]`.
///
/// Pointer-button events do not alter these keyboard axes. Alias keys for one
/// direction do not multiply its speed. The caller must still gate camera input
/// when a text widget or modal owns keyboard focus; `InputState::focused` only
/// describes native viewport focus. egui clears held keys on WindowFocused(false).
pub fn camera_axes(input: &egui::InputState) -> [f32; 2] {
    if !input.focused {
        return [0.0, 0.0];
    }
    let down = |key, alias| f32::from(input.key_down(key) || input.key_down(alias));
    [
        down(egui::Key::D, egui::Key::ArrowRight) - down(egui::Key::A, egui::Key::ArrowLeft),
        down(egui::Key::W, egui::Key::ArrowUp) - down(egui::Key::S, egui::Key::ArrowDown),
    ]
}

#[cfg(test)]
mod tests {
    use super::camera_axes;
    use eframe::egui::{self, Context, Event, Key, Modifiers, PointerButton, RawInput};

    struct Frames {
        context: Context,
        time: f64,
        focused: bool,
    }

    #[derive(Debug)]
    struct Snapshot {
        axes: [f32; 2],
        secondary_down: bool,
        held_keys: usize,
        focused: bool,
    }

    impl Frames {
        fn new() -> Self {
            Self {
                context: Context::default(),
                time: 0.0,
                focused: true,
            }
        }

        fn next(&mut self, events: Vec<Event>) -> Snapshot {
            self.time += 1.0 / 60.0;
            let input = RawInput {
                time: Some(self.time),
                focused: self.focused,
                events,
                ..Default::default()
            };
            let mut snapshot = None;
            // Real egui frame processing maintains keys/pointer state between
            // frames. No fabricated InputState or direct key-set edits are used.
            let _ = self.context.run(input, |context| {
                snapshot = Some(context.input(|input| Snapshot {
                    axes: camera_axes(input),
                    secondary_down: input.pointer.button_down(PointerButton::Secondary),
                    held_keys: input.keys_down.len(),
                    focused: input.focused,
                }));
            });
            snapshot.expect("egui ran the frame")
        }
    }

    fn key(key: Key, pressed: bool) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn secondary(pressed: bool) -> Event {
        Event::PointerButton {
            pos: egui::pos2(100.0, 100.0),
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn held_w_continues_before_during_and_after_secondary_click() {
        let mut frames = Frames::new();
        assert_eq!(frames.next(vec![]).axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![key(Key::W, true)]).axes, [0.0, 1.0]);
        assert_eq!(frames.next(vec![]).axes, [0.0, 1.0]);

        let pressed = frames.next(vec![secondary(true)]);
        assert!(pressed.secondary_down);
        assert_eq!(pressed.axes, [0.0, 1.0]);
        let held = frames.next(vec![]);
        assert!(held.secondary_down);
        assert_eq!(held.axes, [0.0, 1.0]);
        let released = frames.next(vec![secondary(false)]);
        assert!(!released.secondary_down);
        assert_eq!(released.axes, [0.0, 1.0]);
        assert_eq!(frames.next(vec![]).axes, [0.0, 1.0]);

        assert_eq!(frames.next(vec![key(Key::W, false)]).axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![]).axes, [0.0, 0.0]);
    }

    #[test]
    fn key_release_stops_even_in_a_frame_with_pointer_events() {
        for release_first in [false, true] {
            let mut frames = Frames::new();
            assert_eq!(
                frames.next(vec![key(Key::W, true), secondary(true)]).axes,
                [0.0, 1.0]
            );
            let events = if release_first {
                vec![key(Key::W, false), secondary(false)]
            } else {
                vec![secondary(false), key(Key::W, false)]
            };
            let released = frames.next(events);
            assert_eq!(released.axes, [0.0, 0.0]);
            assert_eq!(released.held_keys, 0);
            assert!(!released.secondary_down);
            assert_eq!(frames.next(vec![]).axes, [0.0, 0.0]);
        }
    }

    #[test]
    fn all_arrow_keys_alias_wasd_without_doubling_one_direction() {
        for (letter, arrow, expected) in [
            (Key::W, Key::ArrowUp, [0.0, 1.0]),
            (Key::A, Key::ArrowLeft, [-1.0, 0.0]),
            (Key::S, Key::ArrowDown, [0.0, -1.0]),
            (Key::D, Key::ArrowRight, [1.0, 0.0]),
        ] {
            let mut frames = Frames::new();
            assert_eq!(frames.next(vec![key(arrow, true)]).axes, expected);
            assert_eq!(frames.next(vec![secondary(true)]).axes, expected);
            assert_eq!(frames.next(vec![secondary(false)]).axes, expected);
            assert_eq!(frames.next(vec![key(letter, true)]).axes, expected);
            assert_eq!(frames.next(vec![key(arrow, false)]).axes, expected);
            assert_eq!(frames.next(vec![]).axes, expected);
            assert_eq!(frames.next(vec![key(letter, false)]).axes, [0.0, 0.0]);
        }
    }

    #[test]
    fn opposing_directions_cancel_and_releasing_one_resumes_the_other() {
        let mut frames = Frames::new();
        assert_eq!(
            frames.next(vec![key(Key::W, true), key(Key::D, true)]).axes,
            [1.0, 1.0]
        );
        assert_eq!(
            frames
                .next(vec![key(Key::ArrowDown, true), key(Key::ArrowLeft, true)])
                .axes,
            [0.0, 0.0]
        );
        assert_eq!(
            frames
                .next(vec![key(Key::ArrowDown, false), key(Key::ArrowLeft, false)])
                .axes,
            [1.0, 1.0]
        );
        assert_eq!(
            frames
                .next(vec![key(Key::W, false), key(Key::D, false)])
                .axes,
            [0.0, 0.0]
        );
    }

    #[test]
    fn actual_focus_loss_event_clears_held_keys_without_key_up() {
        let mut frames = Frames::new();
        assert_eq!(
            frames
                .next(vec![key(Key::W, true), key(Key::ArrowRight, true)])
                .axes,
            [1.0, 1.0]
        );
        frames.focused = false;
        let lost = frames.next(vec![Event::WindowFocused(false)]);
        assert!(!lost.focused);
        assert_eq!(lost.held_keys, 0, "egui itself must clear the held key set");
        assert_eq!(lost.axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![]).axes, [0.0, 0.0]);

        frames.focused = true;
        let regained = frames.next(vec![Event::WindowFocused(true)]);
        assert!(regained.focused);
        assert_eq!(regained.held_keys, 0);
        assert_eq!(regained.axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![]).axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![key(Key::W, true)]).axes, [0.0, 1.0]);
    }

    #[test]
    fn repeated_key_down_does_not_change_speed_or_prevent_release() {
        let mut frames = Frames::new();
        assert_eq!(frames.next(vec![key(Key::W, true)]).axes, [0.0, 1.0]);
        for _ in 0..3 {
            assert_eq!(frames.next(vec![key(Key::W, true)]).axes, [0.0, 1.0]);
        }
        assert_eq!(frames.next(vec![key(Key::W, false)]).axes, [0.0, 0.0]);
        assert_eq!(frames.next(vec![]).held_keys, 0);
    }
}
