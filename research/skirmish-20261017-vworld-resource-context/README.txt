VWorld ResourceManager texture context (bounded)
================================================

Current source: vworld_resource_context.rs, plus narrow private Manager+80
storage/methods in resource_depot.rs and Creator retained-Biome accessor.
Original files/executables/profiles are untouched. No full VWorld/renderer,
managed mission activation, positive Ni unload, or playable-battle claim.

Native source and timing
------------------------
4F28FE reads the PARSED Creator+44 pointer;556570 stores it in GMap+24.
43F0EE reads that retained pointer, adds98, and calls5AE0A0. Biome loader
4B8EAC..4B8EC4 gets texture_subfolder_name at5B54D0 and stores its CString
at+98. Actual source values include inherited NULL(empty), Arctic, Desert,
and Tropical. Do not re-Find a stored IDS or use a later selected Creator.

New opaque Biome captures only canonical registration/source identities and
source PC at WorldInit, BEFORE select_creator_before_world_load may replace
MapResource+88. It stores no eager+98 value or mutable resource manager. Each
later input bind validates the actual moved Bank/source universe/PC and exact
canonical entry, then evaluates the actual typed string at that source PC.
Root owns placing this source owner beside creator_extents in World.

5AE0A0 compares UTF16 C-string prefixes case-sensitively using96CB83. Equal
prefixes skip assignment AND invalidation, even if counted suffixes differ
after an embedded NUL. A different prefix copies the complete counted string
at405B80 BEFORE the fallible5ACA50 operation. Later errors retain the write.
The sole same-Application Manager field starts with the actual5B04EA empty
CString construction value. There is no Default or public raw-state setter.

Closed current cache scope
--------------------------
5ACA50 really DOES traverse Manager+60; the exported decompile incorrectly
omits its main loop. Actual class chain target is B3D990, KKC_NiAVObjResource
(CRT9D06E0/name A46588). It only reads flags+8 and signed count+4 after matching
that ancestry, selecting bit5 set and signed count<=0 for5ABFF0.

Current private runtime insertion APIs admit MapResource and FilmResource
owners only. Every map entry, including Pending/Failed prefixes, is counted;
all retained Film allocations are counted. This is not an empty-cache claim.
Actual Map B3D7DC->Resource B38E24 and Film B3D7F0->Map parent chains exclude
the unrelated NiAVObj B3D990. Therefore each admitted entry takes the native
skip before count/flags or unload. Native hash bucket order and Rust BTree
order have no observable consequence within this all-skipped closed scope.

A future NiAVObj or modern GPU resource cache must be explicitly integrated
with this SAME namespace and a concrete unload owner before its admission.
Positive/unbound resource loading, foreign caches and Ni unload are not
represented as success. Native5ABFF0 clears loaded bit0 BEFORE vf40/vf54,
then can perform59BFA0; these nonconstant effects remain outside this port.

Validation
----------
Native producer default mode is immutable. It runs original and v14
separately at Single24/Double53/Extended64:66 context rows +12 source getter/
store slices +3 constructor-empty slices =162 machines.164 distinct executed
address-byte pins;405B80 CString assignment,5B54D0 TGI getter and856370
StringManager are named transports.5ABFF0 is a REQUIRED stopped entry, never
successful unload.5ACA50 is executed in ordinary rows and separately stopped
in the required-prefix probe. Descriptor+8 stores40977F..4097A8 execute with
pinned static CRT parent inputs; full process class registration is not claimed.
Native Map/Film/Resource probes poison count/flags and guard those fields from
reads; matching Ni probes cover bit5 and signed count, stopping before unload.

Fresh immutable162-machine replay passed. Current production cargo check
--lib --release --locked --offline -j4 passed26.39s. Test-only build overrides:
LTO=false/codegen-units16; shipping defaults unchanged. Full --tests checks
were stopped by unrelated in-progress StoredActions then order_gate compile
errors, retained in logs; they are not passing unit-test results.

Three Rust tests are saved for the root shared runtime cohort: every native
context prefix/ctor, explicit counted-string required-prefix regression, and
an ignored actual-source3Map+1Film same-Application test plus all-Biome/all-PC
source census. The actual fixture uses real field constructors, not5A26E0
managed film capture or a fresh whole-application startup. At this checkpoint
these current Rust tests have NOT run; root owns the next shared link/replay.
