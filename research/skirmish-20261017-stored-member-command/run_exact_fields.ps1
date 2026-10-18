$ErrorActionPreference = 'Stop'
$taskDeps = 'target/s-owner-recruitment/release/deps'
$taskMind = (Get-ChildItem "$taskDeps/libaa_mind-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskSpatial = (Get-ChildItem "$taskDeps/libaa_spatial-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskZones = (Get-ChildItem "$taskDeps/libaa_zones-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskSerde = (Get-ChildItem "$taskDeps/libserde_json-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
rustc --test --edition 2024 research/skirmish-20261017-stored-member-command/exact-fields-tests.rs -L "dependency=$taskDeps" --extern "aa_mind=$taskMind" --extern "aa_spatial=$taskSpatial" --extern "aa_zones=$taskZones" --extern "serde_json=$taskSerde" -o research/skirmish-20261017-stored-member-command/exact-fields-tests.exe *> research/skirmish-20261017-stored-member-command/exact-fields-build.log
if ($LASTEXITCODE -ne 0) { Get-Content research/skirmish-20261017-stored-member-command/exact-fields-build.log; exit $LASTEXITCODE }
& research/skirmish-20261017-stored-member-command/exact-fields-tests.exe --nocapture *> research/skirmish-20261017-stored-member-command/exact-fields-tests.log
$taskExit = $LASTEXITCODE
Get-Content research/skirmish-20261017-stored-member-command/exact-fields-tests.log
exit $taskExit
