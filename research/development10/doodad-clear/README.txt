4F5560 and TriggerManager Thing-reference protection

Native producer: native_clear.py; immutable fixture native-clear.json.
Both original and v14 execute51 cases at3 x87 modes:153pairs/306executions.
285 observed pins comprise279 executed instruction pins and6 seam entries.

Actual54C200/540C40/543A90 and native wcscmp execute; CString name ownership,
overlap descriptor/query, logger, destructor and private-node release are
explicit seams. Each actor is queried independently, then the actual clear
coordinator runs over a collected GTID list. Destruction changes flags of a
later Thing to prove lazy current-field reads. Case-sensitive UTF16 comparison
includes NUL-terminated strings with differing data after the terminator.

Rust trigger membership test1/1 and clear coordinator tests2/2 passed. The
clear tests include every observed required-child error prefix; preceding
committed effects remain. Runtime integration uses the SAME existing loaded
TriggerManager; B36520 must never become a second empty construction manager.

This does not prove positive actual destructors, the query algorithm, native
logging policy, complete startup, or a playable battle. See findings.json.

Replay:
python research/development10/doodad-clear/native_clear.py --check
cargo test -p aa-triggers --test native_thing_references --release --locked --offline
cargo test -p aa-spatial --test native_doodad_clear --release --locked --offline
