$ErrorActionPreference = 'Stop'
$taskDeps = 'target/s-owner-recruitment/release/deps'
$taskSerde = (Get-ChildItem "$taskDeps/libserde_json-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
rustc --test --edition 2024 research/skirmish-20261017-regiment-orphan/exact-draft-tests.rs -L "dependency=$taskDeps" --extern "serde_json=$taskSerde" -o research/skirmish-20261017-regiment-orphan/exact-draft-tests.exe *> research/skirmish-20261017-regiment-orphan/exact-draft-build.log
if ($LASTEXITCODE -ne 0) { Get-Content research/skirmish-20261017-regiment-orphan/exact-draft-build.log; exit $LASTEXITCODE }
& research/skirmish-20261017-regiment-orphan/exact-draft-tests.exe --nocapture *> research/skirmish-20261017-regiment-orphan/exact-draft-tests.log
$taskExit = $LASTEXITCODE
Get-Content research/skirmish-20261017-regiment-orphan/exact-draft-tests.log
exit $taskExit
