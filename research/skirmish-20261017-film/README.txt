FILM STARTUP / SAME-STREAM RECORDER -- ongoing native port, 2026-10-17

This folder is NOT evidence that a full battle or film startup has completed.
All native fixtures compare separate original AA.exe and v14 image executions
at x87 PC24/53/64. Native graphs are pinned; allocator/OS/logging seams remain
explicit. Service-stop rows prove prefixes, not native C++ exception unwinding.

Completed bounded pieces
------------------------
film_session.rs / native-film-session.json: 76C310 and real765E00, 123 paired
rows / 246 executions / 140 pins. Same BC roster order, ID-only first DWORD,
low16 counted UTF16 names, positive Kingdom getter repeated, exact diagnostics,
SessionPlayers -> Orders -> actual flush -> Admin attachment ordering. Header
and stream/attachment services are mandatory. Final chunk is Orders.

film_header.rs / native-film-header.json: 59E7E0 latest header7,5116F0 rules4,
50D730 parameters13,512A00 baseCreator3, class-tag41CF30. Nine paired rows /18
executions /1099 pins. Real nested writers and49AA30 run. Stream writes and
5A1120 text reference collection are explicit required seams. Unsupported
rule kinds leave already-written prefixes. This writer is NOT the current
GWorld-to-header producer. Base serializer is factored without changing this
fixture; the final read-only native replay still has identical bytes/hash.

film_creator.rs / native-film-creator.json: actual derived512C30,24 paired
rows /48 executions /1147 pins. Load version0 -> base3 payload -> SAME nullable
sourceResource+50 IDS, including empty and65537-word low16 truncation. The
live entry point borrows resource_creator::Creator and its actual Kind and
source resource; it never labels the Load subclass as a complete base DTO.

film_resource.rs: move-only Resource owns one Arc<Mutex<ChunkFileWriter>>;
RecorderHandle is Weak identity, not another stream or lifetime owner. Opens,
writes, flushes, continued Orders writes and final close use SAME writer.
The attachment becomes available only after actual Orders flush. Flushes do
not invalidate existing native-style handles. Close/drop refuses later writes,
including after another allocation at the same filename. Actual Session1C and
Admin268 attachment are owned by aa-netcode local_bootstrap/local_host. This
memory backend does not prove native work-depot filename creation/file locks.

Root's compressed TGCK backend is separately verified by native zlib1.1.4
inflater equivalence in research/development10/compressed-writer-findings.json.
Compressed representation compatibility is not identical deflater bytes.

film_text.rs / native-film-text.json: 36 paired rows /72 original-v14 parser
and selector sessions /1327 pinned basic blocks. Real TGI parser, TextData,
field-hash growth/rehash, prepend order and49AD70 resolve exact current text.
Includes exact tutorial02/Firefight/Berlin MAPSTRINGS at all PCs. MapTextOrder
retains only original map entry order/provenance; values stay in live_text and
are reread at serialization. New values on existing entries are observed.
The C-locale no-BOM representation is semantically compatible; it does NOT
claim byte-identical4738C0 formatting. It admits counted bytes32..255 excluding
quote, ampersand, backslash and DEL; doubles leading@; refuses unsupported
Unicode/font escapes/control bytes. One Default Text language only. New map
entry creation and language merge remain required producers, not fake empty
entries. Header5A1120 reference-list order does NOT define MAPSTRINGS output;
native59FF28/4738C0 uses GameData+5B4 TextData's actual head-first entry list.

film_variables.rs / native-film-rotation.json: full76FE60,27 paired rows /54
executions /1371 pinned basic blocks. Real CString/CRT integer/localization
and current LastFilmIndex raw store. Fresh State parses actual pinned wildcard
AVars*/UVars* sources once, explicitly ControlledNone profile/console overrides.
Registered LastFilmIndex constructor is0/flags10, not an inferred saved value.
Actual defaults queue5, Films/, LastFilm.RFM, MapSaveCompressed=true. Calls
use/mutate SAME current words; old signed index clamps, new index stores BEFORE
format/name assignment, filename uses OLD index. Rotate0 does no variable or
text reads. Positive unproved source replacements/errors refuse explicitly.

film_paths.rs / native-film-paths.json: 85AB30/85ABB0/85A1A0,120 paired rows /
240 executions /98 blocks. Entire real CString graph, counted rawUTF16 with
embedded NUL/unpaired surrogates. Nonempty extension ALWAYS appends '.'+words,
even if an extension exists. Right-leading slash removes exactly ONE trailing
left slash; otherwise nonempty left without separator gets '/'. No filesystem
normalization, absolute-path replacement, or repeated-slash collapse.

film_header_capture.rs / native-film-capture.json: 5A26E0 coordinator,51 paired
rows /102 executions /34 blocks. Current FILETIME high store beforelow, source
reads after mutable child callbacks, Campaign current/lookup laziness and all
required service/read prefixes. The target is a distinct native-constructed
header. 41D130,510EB0,Campaign manager and5A17D0 are mandatory services; no raw
saved-header shortcut. Campaign lookup is ALWAYS called (create=current!=null).
Campaign name copies to header+3C; header+38 remains untouched here and is
later overridden by5A3250's original-map-header38 argument. The branch is the
CampaignState manager411790, NOT a local SessionPlayer or VWorld lookup.

film_header_entries.rs / native-film-entries.json: full5A17D0,84 paired rows /
168 executions /98 blocks. Real765E00/4BEDF0/50DC00/79EA90/766380/5A1360 and
CString code; no successful getter/profile/append seams. Includes first profile
match, sparse slots, null World/profile and each service/read prefix. Header
count clears beforefirst Session access. AI profile mutates actual target
KingdomParameter+2C before appending the player's entry. Production LiveWorld
reads SAME ActorStore and actual SAI Arena/manager first-match facade; actual
FilmRoster supplies opaque handles/current Sim-aware765E00 and sameNode8 test.

Remaining complete-startup requirements
--------------------------------------
* Native fresh MapHeader constructor5A23F0/5A14C0 process metadata/current clock
  and complete510EB0 current Team/Kingdom/mission-zone parameter producer.
  Native localized parameter pairs include BOTH key and captured current text;
  the parsed map DTO stores key only. Missing5A1120 entry creation needs that
  actual pair value, never an invented empty/default global lookup.
* Actual CampaignState manager4F1310/411790 source path for reached capture.
* Complete5A3250 work-depot/resource publish and5A00E0 open0x102 lifetime.
  A memory TGCK stream is not proof of those resource-manager services.
* Actual selected CreatorLoad and targetResource+88 clone are separate from
  raw MAP_HEADER's appended baseCreator. The selected direct param2 is cloned
  at5A3009 and installed+88; it is NOT a nested object field+8.
*59FDA0 text formatting/materialization and optional separate-mapstrings file
  branch need the actual current EditorMapStringsSeparate UVar/depot owner.
*7757F0's actual Host launch/SessionE4/F8/Session150/currentCreator producer and
  its concrete factory/output adapter must invoke these pieces in native order.
  Recording was not disabled to bypass these requirements.
* No full battle, end-to-end RFM playback, current save compatibility, or
  multiplayer success is claimed by this bounded film work.

Commands and failed attempts
---------------------------
Each Python generator defaults to read-only byte-for-byte fixture verification;
--write explicitly regenerates. PYTHONPATH is reconstruction/tools/python-packages.
Cargo uses isolated target/s-borrowed-leader and --release --locked --offline -j4.
Earlier failed logs are retained: final Orders name corrected from a mistaken
SessionOrders assumption; header cdeclESP assertion corrected; filmText Encoding
import corrected to aa_mapworld::archive::Encoding; capture Loader.ret requires
explicit pop0. Peer WIP compile failures were reported to owners, never edited
or hidden. Native fixture bytes did not change for unrelated peer fixes.
