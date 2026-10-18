Saved MISC_DATA owner, 2026-10-16

aa-runtime::saved_misc ports GWorld::Load4FC600's4FD017..4FD3B3 body. It owns
the actual ordered NameList counter vector(+268/+26C), pre-cutscene speed+278,
WeatherData reference+290, head-to-tail UTF16 string list+294/+298, and optional
two-word allocation+27C. The input version originally read by4FC600 governs
all gates, not a later raised GWorld+2C. The caller selects the MISC_DATA chunk
at version18+; older streams continue their current position. No fake chunk
selection or world-ready success is returned by this module.

Constructor4F7D00 initializes the vectors/list/pair empty, but does NOT write
+290 Weather. Reference::Uninitialized retains that fact until a reached Load
store. +278 reads **[B39AB0+114], the actual fixed GameSpeed variable. The native
507DB6..507DE4 registration creates GameSpeed with default0/index0x45 through
401710/4B3470. The pure constructor requires the current input word explicitly.
The native ctor-field slices execute actual stores/allocation and the speed
read; they do not certify the entire GWorld constructor or its other managers.

from_source_constructor(Catalog,WorldInventory) is for the controlled fresh
source-settings window only; callers must have no intervening SVar mutation.
It validates source ownership and the completed source-PC arena, the unique
Fixed GameSpeed declaration against the actual registered SVars value/type/span
and matching source hash, and refuses registered or saved GameSpeed WorldRule
application. It checks MAP_HEADER rules, creator rules and the separately
retained RULES bytes/identity. A later/UI-owned variable must provide its actual
current word to State::new instead. No startup/UI setting mutation is inferred
from the absence of a Catalog field. This is not a generic live SVar service.

Native order is preserved: version12+ NameList count then each UTF16 IDS/value,
lookup against GameData+59C (NameList manager484D90), warning-and-continue for
unknown IDs, ordered append for known IDs including duplicates; version13+
raw speed; version14+ Weather via4F9280; version24+ head-insert string list;
version28+ presence byte then allocation replacement before its two reads.
A false presence byte retains an existing allocation. Newly allocated but not
yet read words are represented as uninitialized Option values, never heap-fill
or zero defaults. Weather unknown lookup stores null before throwing. Serialized
string length governs the empty Weather test, while lookup/diagnostic text uses
the NUL prefix. Used strings retain all UTF16 units, including unpaired units.
Unknown-Weather diagnostics containing an unpaired lookup prefix are an explicit
unsupported boundary because the existing native Message model uses String.

native_saved_misc.py executes both original and v14 images at allthreePCs:
774 rows/468 executed nonleaf pins,129 cases per build/PC. It enters the actual
body with the surrounding frame/register inputs supplied explicitly and stops
at4FD3B3. It also executes the relevant ctor field slices. Registry lookups are
declared dictionary seams, not fabricated actual source-manager receipts.
The cases cover every byte-truncated prefix of the complete version28 payload,
all version gates, existing state/duplicates/aliases, warning continuation,
unknown Weather store-before-throw, nonempty leading-NUL Weather, NUL tails,
negative counts, head-insertion order and allocation partial-initialization.
The shared all-or-nothing stream-read/error/allocation-success seams are explicit.
This proof does not execute the whole surrounding Load, real allocation failure,
native EOF implementation, chunk seek, PostLoad, or end-of-load publication.

native_actual_chunks.py independently unpacks original tutorial02,
allies01_el_alamein and(2v2)Berlin and runs each MISC payload under both native
images/allPCs:18 additional cases. The first two are version27/14zero bytes;
Berlin is version28/15zero bytes. Their named/Weather lookup branches are not
reached. The Rust actual-source test first completes/verifies the captured full
source CV owner, binds real GameSpeed0 from its source precision, then loads
those three payloads and checks unchanged empty collections and loaded null
Weather. It also refuses a forged Settings.raw value and a disabled saved
GameSpeed rule (native application ignores that locked/enabled byte).

Final validation: tests-v3.log, MISC3/3 plus Sun2/2 passed, release --locked
--offline -j4 --include-ignored. Actual-source-v1.json records the three original
RMP/chunk identities and resulting states. All14 production/test/evidence pins
are in freeze.json. Shared lib.rs has only the two unique exports.
No aa-rules/parser, shared Catalog, root manifests/dependencies, native assets,
executables, saves/profiles or release files were changed.

Superseded attempts are retained. native-saved-misc-v1/v2.json are early
producers, excluded from final acceptance. tests-v1 compiled the earlier raw
NUL-lookup source while the fixture was expanded: its new NUL-tail regression
failed, and the final source now explicitly uses the native NUL prefix.
tests-v2 failed on another author's in-progress live_scheduler_dispatch type
comparison. tests-v3 is the final accepted replay; no guards were weakened.

Reproduce from rust-rebuild:
  python research/development10/saved-misc/native_saved_misc.py
  python research/development10/saved-sun/native_actual_chunks.py
  CARGO_TARGET_DIR=target/s-settlement-completion
  AA_GAME_ROOT=E:/Games/Axis and Alies
  cargo test -p aa-runtime --release --locked --offline -j4 --test saved_misc --test saved_sun -- --include-ignored
The actual-source report is read-only unless AA_MISC_REPORT names a new file;
its writer uses create_new and refuses to overwrite an existing report.

Root owns retention of this same State, canonical source-token callbacks,
actual warning consumption and ordering after Sun/queues and before PostLoad.
The source wrapper and codec do not certify a playable battle.
