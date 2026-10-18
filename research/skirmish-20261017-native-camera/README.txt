Native camera work checkpoint, 2026-10-17

This checkpoint does NOT complete the real7016A0 constructor, NiCamera update,
camera publication, scene attachment, VWorld initialization, GPU or a battle.
native_camera::State is explicitly a partial field owner. Unknown fields stay
unbound, including constructor+204 (actual702380 later writes the ordinal).
The renderer's current camera must never be replaced with viewer Camera data.

Completed current-variable owner:
 native_camera_variables.rs retains17 disjoint current AVars7..23, same
 completed source Bank/PC/startup provenance, current Settings declarations
 span/type/raw including constructor-only absence. Profile overrides require
 their actual owner; this scope is explicit ControlledNone. Original401BD0
 registration executes6times (bothPE/allPC),1364pins. V14's402446 gameplay-max
 patch2.1 is retained; original is1.4. Camera source files are real native
 wildcard AVars*.tgi, including AVars_AA.tgi (public Settings omission was
 corrected separately by root). Fullparser+Settings4B3C10 source policy runs
24times/4614pins: flags2/34 source Float3 records hardcoded diagnostic, skips
 numeric getter/store, continues to the next writable field. Diagnostic log
 level/render output is not asserted by this current-cell projection. Fixed4
 hardcoded comparison/inheritance/duplicate-field/error traversal remain out
 of scope. Source success has distinct independently written cells; malformed
 source partial-error authored order is NOT claimed equal to native hash order.
 Independent real source factory probe passed in view-startup-inputs evidence.

Reset700E30 +projection6FE920/6FDF80:
 native_camera_reset.py:53cases*2PE*3PC=318executions,268executedpins,
480localhardware FSIN/FPTAN transports. Mode/dev/visualheight/pose callbacks
 are explicit required transports. Prefix errors are pre-dependency cutoffs,
 not native exceptions. Each read/store keeps native order. Binary32 local
 stores, extended FPTAN input and near max(input,far/10000,0.1) are preserved.
 Constructor field projection preserves byte-only known masks; it does not
 invent upper bytes or substitute uninitialized204 withzero.

Pose6FF220 +constraints6FEA20 +rotations6FE3B0 +zoom6FE470:
 native_camera_pose.py runs180full actual native reset/pose/NiCamera math paths,
1591executedpins,1260localhardware trig transports. Real9E3520 initializes
 processB40054 matrix. Rust math currently ports only through the required
6FE050 Ni update child. Tests compare all ported fields/arguments/ordered
 reads against native; the native's laterNi outputs are retained as reference
 for that child and are NOT currently asserted as an implemented Rust update.
 Camera+164's separate0x100-byte root allocation was watched in all180paths:
 zero writes; that excludes arbitraryparent/controller callbacks (laboratory
 parent24/B0NULL) and does not prove the root constructor or a GPUtransform.

Trig authority:
 Unicorn transcendental instructions lose low80bits. Producers substitute
 THESE CPU's isolated hardware FSIN/FCOS/FPTAN results, preserving raw80 and
 full host x87 environment. Transported opcodes are listed, not executed pins.
 aa_trig::turning::small_tangent_extended accepts finite |angle|<=f32pi and
 preserves full caller state; hardware exactoriginal FOV sequence allPC passes.
 All13aa-trig tests passed. This is not universalCPU transcendental parity or
 x87exception-flag parity. Supported finite/normalextended domain only.

Rust replay:
 replay-math.ps1 reuses same accepted non-LTO dependencies without a full
runtime link. Exact production-source includes:3/3tests pass0.07sec.
 Native immutable replays: python native_camera_reset.py and
python native_camera_pose.py (no --write); they compare existing JSON.
 No original game/save/profile/SDK code was modified or executed.

Next required integration:
 Real874B40/Ni base, full7016A0 constructor (+164sameNode), directory702380,
 actualNiCamera update/plane owners, current mode/dev/UVar gates, same source
 seed allocation, realvisual431550 andterrain550210; scene7010D0→7005F0 and
700500 must use that same completed camera. No ready bit may be minted here.
