Native MobileDeployment command availability, 2026-10-16

This is the pure action-availability contributor needed by restored actors.
It does not implement packing/unpacking execution, animations, spawning,
elapsed-time updates or a playable battle. Runtime binding is a separate owner.

Original functions
575FB0: discarded group13 lookup, then575C00(true), then actor+70 Mind test
only after that predicate succeeds; read/modify/write actor+F0/F4 bit34.
The other63 bits are preserved, including high bits outside active commands.
575C00: ActorData+548 must be non-null; +54C pack_health_min compared with
9F32E0=1.0; optional Properties+1C must not have bit0x800. With argumenttrue,
575B70 must succeed and4D0230(false) must be strictly greater than+54C.
575B70: group13 absent(-1) orNORMAL(27), then actor state bit21 must be clear.
575BB0: ActorData+534 must be non-null; argumenttrue applies the same state
gate as575B70, inline. It does not query health, Properties or Mind.
4D0230(false): Body+10 / Body+14 in x87 at the selected significand precision;
without Body it returns9F32DC=+0.0. It does not aggregate settlement/members
because this call's explicit argument isfalse.

The instruction branches matter:575C24..29 TEST AH,1 accepts threshold<1
OR unordered.575C65..6E FCOMP / TEST AH,41 / JP accepts ratio>threshold OR
unordered, but NOT equality. Quiet/signalling NaNs, infinities, signed zeros
and zero division have explicit classification paths in the Rust predicate.
Finite ratios use aa_numeric::Binary::div at PC24/53/64; no binary32 store is
inserted before comparison. FPU exception flags and NaN payload identity are
not returned by this predicate API; the oracle compares its masked branch
results and action bits, not those unexposed flags.

Source binding requirements for the runtime owner
ActorData+534 is the forward unpack_to_actor_ids reference.4605B0 resolves it,
validates Body/MobileDeployment/Element compatibility and packed-position
shape bounds, then writes the reverse source pointer to target ActorData+548.
Having a MobileDeployment component alone does not prove either reference.
464BD0 reads+54C pack_health_min only for an empty unpack_to_actor_ids.
Those definition-load facts are NOT synthesized by this contributor.
World::has_mind must reflect the authoritative logical Mind presence, including
an active Mind temporarily moved into an Arena by its owning runtime adapter.

Public API: aa_actors::mobile_deployment_actions
  contribute(world, actor, precision)            575FB0
  can_pack(world, actor, check, precision)       575C00
  can_unpack(world, actor, check)               575BB0
  state_permits(world, actor)                   575B70
Required World facts: fields and action setter, has_pack_reference(+548),
has_unpack_reference(+534), pack_health_min_bits(+54C), Properties flags,
optional BodyHealth{health_bits, maximum_bits}, and has_mind(actor+70).
Missing facts return errors. No default callbacks or successful no-ops exist.
Fact reads short-circuit in native order. A fact error occurs before the sole
action write; if the setter fails its caller owns that write's consequences.

Evidence
native_mobile_actions.py executes the original575FB0/575C00/575BB0/575B70,
4D0230,529C30,4CF110,4D14C0/4D0820 unchanged under Unicorn.477FC0 first
registers actual original state descriptors/masks. Its established allocation,
CString and registry-transport seams are used only for this startup. No
contributor, health result, state predicate or Properties lookup is mocked.
Page-zero reads are guarded. All1294 observed image instruction byte sequences
match both originalAA.exe and accepted v14. The552-case fixture covers state
groups/bit21, Mind, reference and Properties gates, absent Body, equal/near
health thresholds with PC24/53/64, and nonfinite arithmetic branch outcomes.

Tests
  CARGO_TARGET_DIR=target/s-actorcreate
  cargo test -p aa-actors --test native_mobile_deployment_actions --release --locked --offline -j4
Replays all552 native rows, preserves unrelated mask bits, and verifies that
missing required facts cannot write actions and unreachable facts stay unread.
Set PYTHONPATH=reconstruction/tools/python-packages, then:
  python research/skirmish-20261016-mobile-actions/native_mobile_actions.py
Default is read/compare. --write is an explicit fixture regeneration option.
Logs and findings.json record final checks and hashes.

Preservation
Backup: backups/source-before-mobile-deployment-actions-20261016.zip.
The contributor added a new aa-actors module/test and one lib.rs export.
The later approved source-projection extension is documented separately in
README-source-pairing.txt. No existing generic parser, runtime, original asset,
save or release08 file was edited by either portion of this work.
No fixed memory/bandwidth/disk/VRAM cap or RNG consumption was introduced.
