Native FX initialized unset-position constant, 2026-10-17
========================================================
B28114/B28118/B2811C are binary32(-100.0) each, encoded0xC2C80000 in BOTH
pinned original andv14 PE data. They are not zero and are not the unrelated
B3A8C4 PointShape reference. The engine uses them as an unset point through
428170/428430 comparisons and428620/4286E0 command target initialization.

Raw complete PE-section scans find34 direct literal references to these three
addresses:14 toB28114 and10 each to18/1C. Every direct instruction is an absolute
source read, or passes the base address to point comparison428170/428430 or
42A710.42A710->429080 copies the input point into locals before later effects;
no original pointer is retained in its command. Both images have the same
reference sites and bytes. This is a bounded direct-reference audit; it does
not make a whole-program computed-alias or external memory-patcher guarantee.

native_fx_static_position.py executes180 pairs/360 fresh native runs at all3PC
and both images: original constructors428620/4286E0, finite equality/complement
comparators428170/428430, and actual address escape42A710->429080->428480 with
null source and muted/nonempty/app-worker branch fixtures. The actual mapped
sentinel is never overwritten by the harness. A memory-write guard rejects any
write to its12-byte region during each run.456 executed pins and34 reference
records are retained; allocator/OS/final queue are declared services. Constructor
source positions remain the separate caller's positions. Target sentinel bits
come from the original initialized data on every native case.

NEW fx_static_position keeps one private immutable static with those proven
bits. unset_position() copies the same value when requested. It does not create
a mutable per-World snapshot, a fake zero vector or a new gameplay setting.
RequiredEffects::enter_fx now returns that native constant at the pre-existing
lazy global_position constructor callback. No match/mute/earlier effect error
reaches it, preserving the previous native ordering. Positive event cases can
now advance that specific dependency; actual app View clock/delivery/destruction
remain separate required services and no full presentation/battle claim follows.

Exact production module's focused Rust fixture test1/1 passes (compiled directly
with rustc --test), replaying constant reads from all fixture cases. Runtime
library check currently stops at unrelated film_resource_factory Creator versus
Box<Creator> migration errors91/100/109; its failed log is retained honestly.
Parent owns final shared integration validation. No full runtime pass claimed.

Contemporaneous load_world.rs before-copy is under before/. Only its exact
RequiredEffects global-position closure was edited; root owns all other World
work. lib.rs adds one module export. No SDK code/dependency/manifest, original
PE/save/profile or release changes.
