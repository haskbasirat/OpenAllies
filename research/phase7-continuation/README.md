# Phase7 movement continuation

The single original5300A0 kernel now has `aa_navigation::try_action_go` with required fallible services. Existing `action_go` uses an infallible compatibility adapter. Activation/path event APIs are unchanged.

`aa_mind::element_move_continuation` implements5379C0 continuation,52FD90 turn offset,537150 forward check,52FD50/409230 collision normalization, and536F40 outer decisions. It shares `element_move::Payload.plan` and the authoritative Mind Arena. `get_back_to_path` implements532B50's common prefix; `recover_turn` implements532C64..533128. The split supports native prefix fixtures; it is not an approximate route fallback. Live World adapters call these concrete functions.

Recovery preserves the non-normalized requested direction, asymmetric stores, square-root then square comparison,4096-entry rotations,52DDB0 intersection (parallel output retains initialized angle), placed-shape/sweep requests, and52CA90/52D180 node ownership. Leading kind0 removal strips the whole leading chain. The unusual final kind1-before-kind0 order follows native prepends.

`aa_trig::turning` supplies exact bounded FPTAN offsets, native FCOS/acos recovery-angle selection, extended FSIN/FCOS values, and51D510 near/precision-flag observation. It preserves the caller x87 stack/status/control and MXCSR. Existing angle and lookup arithmetic is unchanged.

Validation:

-348 whole original52FD90 runs with real51D510,445400/CRT and FPTAN, acrossPC24/53/64 and both incoming PE states. No decision hooks; Rust comparison uses host x87 instructions.
-219 original continuation cases:102 outer/forward/collision/normalization branches,27 bounded532B50 prefixes,90 full532B50 recoveries. World virtuals, movement/scheduling, descriptor construction and query results are explicit fixture callbacks. Full recovery retains original trig/CRT, table lookup, line intersection, node construction/allocation wiring and offset calculation. Table bytes, period/count and publication fields are source-bound.
-255 existing Phase5 path/activation/ActionGo cases are independently reexecuted without replacing their gold fixture. All ActionGo cases also run through the fallible entry; every reached callback is failed after effects to check immediate stop, retained mutations and no later RNG/timer call.
-Continuation tests fail every reached callback and compare exact event prefixes and unchanged post-error payload. Seven trig tests include original angle/table cases, caller-state preservation and unsupported-domain errors. Four navigation fixture tests and two continuation tests pass; strict focused Clippy passes.
-Full function ranges, visited-address bytes (including intercepted seam entries) and constants are pinned against original and v14 binaries. A visited hook entry is provenance of the source boundary, not evidence that its intercepted body executed. `record_evidence.py` records source, fixture and test hashes. Independent peer review is a separate gate.

Required live integration:

-ActionGo must bind actual formation, organization, character/mover, RNG and scheduler behavior.
-Recovery descriptor554350(actor,false), generation checks and4CE6E0 ignore-target selection remain required source bindings.553BA0 is a placed-shape first hit; it cannot substitute for553CE0 sweep.531170/5312C0 adapters remain required, although the spatial owner now provides the shared sweep kernel.
-536F40 still requires536840 avoidance,536AD0 stop distance,530F40 collision-path result,52E130 wait,534A40 descriptor path plan,4D67F0 visibility and actual shape/overlap virtuals. No result is invented. Target-following533C30 is separate.
-Finite values, round-to-nearest andPC24/53/64 are supported. Incoming sticky precision flags are explicit contracts; full live-engine FPU lifecycle is not claimed. New trig helpers require x86_64 and bounded small angles. Unsupported/nonfinite results fail. No extreme FPREM or approximate host-libm fallback is introduced.
-Allocation/callback/numeric errors retain earlier effects. The runtime must latch failure and never automatically retry. Synthetic world fixtures are not evidence of an entire live match or playable multiplayer.

Original game, asset, save and release files remain unchanged.
