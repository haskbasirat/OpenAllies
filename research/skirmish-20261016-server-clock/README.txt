Native ServerClock arithmetic and exact time-source boundary

Existing aa-netcode heartbeat only emitted AdminClockRestart; existing simulation
Driver required real_admin_clock and host_rate. No matching implementation was
found in aa-orders, aa-netcode, aa-runtime or aa-client before this addition.

server_clock::ServerClock owns native75FBB0 fields+4 period,+8 limit,+C offset.
It has no Clone/Copy. restart75FBF0 stores period before asking actual59ABA0
for milliseconds. That callback must be the Node's retained QPC Timer with its
actual anchor/frequency; GetTickCount, game time and a generated zero are not
substitutes. Cached getters and host_rate read the actual Node+58 through a lazy
TimeSource. The global/bootstrap author owns that concrete timer/Node binding.

The native signed FILD/u32 correction is spilled to binary32 before division.
75FBF0 keeps division extended through subtraction;75FC40 spills division before
subtracting offset.75FCB0 spills its raw result and returns the lesser of it and
limit, choosing the stored limit on equality.75FCF0/75FD70 subtract limit and
spill.75FD80 spills elapsed and limit+0.001, optionally updates offset, rereads
cached Node+58, then raises limit only on strict greater. It never calls QPC.
Native0.1,0.001,1000,2^32 constants and the vtable/ctor are pinned by the producer.
All arithmetic uses explicit caller PC and aa_numeric, without host float math.
Finite arithmetic is the stated domain; NaN/invalid raw source words are errors,
not a claim of native SEH/nonfinite compatibility.

native_server_clock.py runs210 fresh machines,1932 ordered operations over both
unchanged binaries and all three PCs. Actual ctor/restart/getters/rate bodies
execute;149 genuinely executed instruction addresses are pinned.59ABA0 is an
explicit u32 input transport seam and separately listed nonexecuted boundary.
It does not prove the QPC implementation. Two cached host-rate reads can receive
different values through a labelled external memory intervention, not an inferred
application callback. Millisecond edge values include sign/u32 rollover and
binary32 rounding boundaries. Native ST0 is captured as exact80-bit data. Missing
timer inputs stop at the boundary after the period store; they never succeed.

Default command is immutable replay:
  python research/skirmish-20261016-server-clock/native_server_clock.py

The Rust oracle test replays every operation/result/field and lazy input count.
A second portable test confirms a required second cached read retains the prior
offset correction and does not invoke QPC. No native exception-equivalence claim
is made for that deliberately unavailable portable source.

This component does not launch the game, construct a duplicate Node, bind an
unproven admin time or claim an operational multiplayer/battle frame.
