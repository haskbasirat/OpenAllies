Saved Sun owner, 2026-10-16

The new aa-runtime::sun_state module ports the fields of GWorld+4C's single
Sun owner: constructor4EE0A0 (Updatable base50AB90, vtableA2A578) and saved
Load4EDBD0. It is not a second renderer, scheduler, or world-ready flag.
The constructor takes the three actual B2814C/B28150/B28154 words explicitly.
Its Updatable+4 cancellation head starts null. Load receives that same head
by mutable reference at the actual50B250 callback; a nonempty live head must
be cancelled by its real queue owner. This port does not silently discard it.

The outer Sun version0..2 is read before the Updatable version0..1. Raw
binary32 words, including exceptional patterns, are copied without arithmetic.
Each of seven triples is one12-byte read, in native order (direction, three
current colors, three target colors), then five separate words. Version1+
reads the two bytes and four transition words. Only after those reads do
versions0/1 zero the four transition words. Errors preserve prior stores and
registration/cancellation effects. The caller retains this same State across
Load. Padding, native pointers, allocation capacities, and C++ vtables are not
invented state fields.

native_saved_sun.py executes BOTH images at PC24/53/64:2376 rows,372 executed
nonleaf instruction pins,396 cases per build/PC. It executes the real ctor,
unit-vector normalization, Updatable Load and table checks. Stream/string/error
leaves come from the pinned shared native_machine.py; cancellation50B250 is an
explicit host seam. Byte-by-byte truncation is proved under the harness's
all-or-nothing read seam, not a claim about native filesystem EOF behavior.
The positive id==table-word0 synthetic case has an explicitly overallocated
lab table; a real owner must still refuse access outside its actual allocation
(the existing runtime Table already does so). It is not valid shipped input.

static_audit.py independently decodes all66 exact-address references to the
three globals in the complete image; all are MOV/FLD reads, none address-taking
or direct writes. The globals are initialized zero bytes in writable .data,
not a BSS tail. This census does not rule out arbitrary indirect aliases or
assert perpetual native immutability. It also pins ctor/Load/setting windows
used by this port and the accompanying MISC port.

Another author obtained an actual isolated Windows startup observation through
source completion and Session construction,62 watched threads, no native
writes and three calibrated post-cutoff watch hits. The separately owned
load_sun_reference::ConstructorInput verifies that sealed reference and current
source/numeric owner before consuming it to construct this State. The Sun ctor
itself was not reached by the capture. The independent-sun-reference-review
report here audits that OTHER author's binder/recorded receipt; it expressly
does not independently review this author's own Sun implementation.

The Rust saved_sun tests replay all2376 rows and load original tutorial02,
allies01_el_alamein and(2v2)Berlin chunks. native_actual_chunks.py independently
unpacks those original RMPs and runs each payload under both native images at
allPCs, comparing all stored words and callback traces to the Rust codec
report:18 additional actual-payload runs. Each real saved Sun has id-1, so
the actual table registration branch is not reached in these three inputs.
These isolated codec tests use explicit image-zero constructor inputs; the
separate sealed reference is required by the actual world constructor path.

Validation: research/development10/saved-misc/tests-v3.log records Sun2/2 and
MISC3/3 passing with --include-ignored, release --locked --offline -j4 and
AA_GAME_ROOT set. All command sessions were reaped. tests-v1 passed the initial
Sun parity test; actual-map-tests-v1 failed only on a relative report output
path after all three map loads, then tests-v2 passed with an absolute path.
The old failure remains preserved. No native game, asset, profile, save,
manifest/dependency, aa-rules/parser, or release file was changed.

Reproduce from rust-rebuild:
  python research/development10/saved-sun/native_saved_sun.py
  python research/development10/saved-sun/static_audit.py
  python research/development10/saved-sun/native_actual_chunks.py
The latter reads the preserved actual-map-codecs-v2.json Rust report and
independently checks it against the original RMP bytes before native execution.
  CARGO_TARGET_DIR=target/s-settlement-completion
  AA_GAME_ROOT=E:/Games/Axis and Alies
  cargo test -p aa-runtime --release --locked --offline -j4 --test saved_sun -- --include-ignored

World retention, chunk selection before queue Load, live cancellation/table
target binding and end-of-Load sequencing are root-owned integration. Sun
Update4EDDC0 and trigger-to-Sun effects are not certified by this codec proof.
No playable-battle or world-publication success is claimed here.
