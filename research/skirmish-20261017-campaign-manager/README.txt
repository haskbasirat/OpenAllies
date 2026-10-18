Fresh application Campaign table

The native table belongs outer Application+13C/+140/+144 (B397CC/D0/D4).
Both images place these words in PE-loader zero-filled virtual section tail.
Real CRT initializer9C79C0/40CAB0 leaves them untouched. Rust lifetime storage
is retained privately alongside the fresh GameAppState application component;
the private Arc/Weak witness is identity metadata, not a native mode field.
Construct that application component once at fresh app bootstrap, retain it
through mode changes/maps, and never replace it to reset Campaign storage.

application.campaigns() loans the SAME move-only manager. No public Campaign
constructor/reset/raw list setter/Clone exists. current_4f1310 reads actual
GameAppState+C; normal nonzero mode returnsNULL. lookup_411790(false) reads
actual empty count before any requested-name dereference, including poisoned
name pointers in the native proof. Mode0/create=true requires actual allocator
6F7810/470970 before append/resource loading. Positive name/index remain
required, never empty/default success. No global-forever-empty claim is made.

Native42paired/84original-v14/allPC machine runs/74executed pins; sole success
seam CRT atexit registration. Actual651810/40CA60 and query bodies execute.
Application+FC link and mode edits are explicit inspected/source fixture inputs,
not execution of full40CB40/Application/console startup. Native immutable replay
passes; Rust Campaign2/2 and existing Application2/2 (270-row native baseline)
pass. See findings.json and production-pins.json; independent review separate.

Read-only replay:
python research/skirmish-20261017-campaign-manager/native_campaign_manager.py
frozen-campaign-tests.exe campaign_manager::tests --nocapture
frozen-campaign-tests.exe application_mode::tests --nocapture

No Campaign gameplay, full film capture/writer, playable battle, multiplayer or
completion percentage follows from this slice. Original game, assets, saves,
releases, dependency manifests and lock were not written.
