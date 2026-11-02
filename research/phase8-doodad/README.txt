DynamicDoodad deferred updates of the original AA.exe.

Oracle: native_doodad.py runs the original 0x5902C0 (Disturbable, Updatable +18)
and 0x58FA30 (Breakable, Updatable +18) under Unicorn at PC53, through
research/phase8-ai/native_machine.py, and writes native-doodad.json:
240 Disturbable runs, 480 Breakable runs over every state 0..8.
Replayed by crates/aa-play/tests/doodad_native.rs on aa_play::doodad.

Seams, recorded in each run's "log":
  0x5901F0  the circle scan's answer (an input here)
  0x4F02B0  GetRNG (the getter)
  0x85D840  the draw (its value is an input)
  0x50ACC0  the re-arm, delay bits and data word
  0x50B250  the cancel
  0x42A590  the topple action list
  vtable +0xC of the Disturbable (0x590380, its action list)

Proven: the order of those calls, the binary32 next time and re-arm delays, and
the Breakable's words. What the original does and a reader might not expect:
  * state 4's "not yet" branch loads +0x1C (the topple duration), not +0x20;
  * the delay loop goes on only while the delay is exactly 0: a NaN leaves it;
  * left is on the x87 stack (binary64 at PC53), stored binary32 only around the
    0x42A590 call of state 2.

Not proven by this oracle:
  * 0x5901F0 and its circle test: aa_spatial::circle_actors has its own fixtures;
    aa_play::doodad::disturbed reproduces the test over aa-play's actors, and is
    measured against the films by the census (research/byte-exact/PARITY-CENSUS.md).
  * 0x58FF70, the Break that starts state 2, has no host in aa-play. No stock map
    saves a Breakable in a state other than 0 (the Load 0x58FD60 zeroes the words
    with GWorld+48 = 0), so its Updatable only ever cancels itself in the films.

Measured on the maps (crates/aa-play/tests/map_queues.rs, 2026-11-02): (2v2)Berlin's
queue 3 is 796 events, two for each of 398 Disturbable components of grass;
(2v2)Kursk has 256 of these and 1 Breakable; (2v2)Comrades none (its 537 trees
are re-created at load, 0x4FD47C, and their Breakables cancel themselves).
