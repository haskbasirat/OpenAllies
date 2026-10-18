Application window focus word (B396C4)

Original App=B39690;41781A sets+34 to0.40DAC0 promotes exactly mask2 to3, ORs current bits and stores before transition effects.40DA30 clears requested bits before transition effects. Native WndProc6FD4A0 maps WM_ACTIVATE active-lowword/non-minimized-highword to mask2 and WM_ACTIVATEAPP to mask1. WM_SETFOCUS only logs.

Proof:840 original/v14 executions,384 word cases +33 WndProc routing cases, all3 PCs;2 standalone Rust tests passed. Native observer cutoffs are not executed-pin claims. Existing v14 input-clear wrapper is independently pinned and bounded before post-return custom effects.

One non-Clone UI writer owns atomic flags; cloned Reader handles retain the same allocation and reread it at each lazy simulation gate. No per-frame flags/Gates snapshot. windows_focus samples real Win32 current foreground process/active HWND/IsIconic; it does not fabricate focus from GameAppState or unavailable egui values. This platform observation translation is not exact WM timing parity. Full timer/input/capture effects and GUI acceptance remain outside this bounded result.

Reproduce native: PYTHONPATH=<game>/reconstruction/tools/python-packages python research/skirmish-20261017-window-focus/native_window_focus.py
Reproduce Rust without runtime link: set CARGO_MANIFEST_DIR=<workspace>/crates/aa-runtime; rustc --edition=2024 --test crates/aa-runtime/src/application_window_state.rs -o <owned-target>/window-focus-tests.exe; run binary.
