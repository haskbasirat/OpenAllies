# Actual Session cleanup during pre-static Kingdom deletion

Native76C7C0 traverses the current Session BC list and calls765E00 for each
player. That getter first reads globalWorld B36534; when nonnull it returns
the raw retained Player+28. A pointer match reaches765E10(NULL). A complete
VWorld replacement is not required to read that raw slot: the constructor
sets it null, and765E10 stores it before later destructor/constructor work.

The additive SessionPhase method
clear_kingdom_references_in_created_world accepts the sealed SAME current
managed-load World revision, published at76AA90. The caller must validate its
current Catalog/session/load-world scope; arbitrary old constructed tokens or
scalar booleans cannot establish the contract. The actual SessionLoadBorrow
implementation reads the same retained BC roster in order. A nonempty null or
foreign-link roster completes without any write. A first matching player
returns an explicit765E10-required error before effects. No complete-view or
binding_complete guard was weakened, and those guards still protect readers
that require finished VWorld construction. No matched replacement or no-World
branch is implemented by this immutable worker loan.

SDK's native-session-scan producer/fixture under the separate configure-kingdom-
removal evidence area executes real76C7C0 and765E00:54 rows, original/v14,
all three x87 precisions,26 executed instruction pins. Positive765E10 is a
named laboratory mutating child/entry cutoff; it is not a real replacement
implementation. Six no-World cases poison Player28 to prove native laziness.
Independent read-only replay here passed with the exact frozen fixture hash
a4c700872a2019f6eb4185219061e5d5317e2a56d3cd761e2ba83f6aa76e238e.

The Rust adapter test admits24 positive-World no-match rows and checks the
first required child boundary in24 further rows. Six no-World rows remain
explicitly outside this adapter's scope. A separate test confirms nonempty
constructor-null raw slots while VWorld binding remains incomplete, and a
stored raw pointer remains observable after later view-construction failure.
The pre-existing24-row native fresh reset/player-constructor replay separately
proves constructor+28 is null. Focused current-tree Cargo validation status is
recorded in netcode-tests01.log and the final checkpoint, not inferred here.

All builds in this work use target/s-local-bootstrap, release with explicit
lto=false/codegen-units=16 overrides. They are distinct from the final thin-LTO
release. The earlier PointShape proof and sealed reference in the same globals
module remain unchanged; only the SessionPhase trait gained a default-required
method. Backups retain the exact pre-edit module and Bootstrap source.
