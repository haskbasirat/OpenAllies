Native presentation queue kernel, active Development10 work.

fx_manager.rs ports50CE00 header stores and50C800 submission ordering.
72 submission rows +9 constructor rows execute unchanged original/v14
instructions atPC24/53/64 (162 separate machines,114 executed addresses).
Allocator, unused capacity-element lifecycle, view callback and command
destructor are explicitly named seams. Constructor capacity4 is allocation
policy; no4-command limit is imposed by the Rust VecDeque.

Fresh immutable native replay passed. Rust fixture replay passed1/1 using
release test-only overrides LTO=false,codegen-units16 in target/native-runtime.
See qa-output/development10/fx-manager-kernel.log. Neither run starts a game.

This kernel is NOT bound to created-FX map callbacks yet. A real compiled
immutable FXC graph and native command-constructor binding are still required.
The queue never treats authored FX metadata as completed commands. View
attachment/drain, complete SoundManager/render effects and real battle remain
unfinished. No Development10 EXE has been packaged.
