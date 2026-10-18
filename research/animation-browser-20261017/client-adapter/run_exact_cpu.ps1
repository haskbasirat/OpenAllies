$ErrorActionPreference = 'Stop'
$taskFolder = 'research/animation-browser-20261017/client-adapter'
$taskPins = Get-Content "$taskFolder/exact-cpu-pins.json" -Raw | ConvertFrom-Json
foreach ($taskSource in $taskPins.sources.PSObject.Properties) {
    if ((Get-FileHash -LiteralPath $taskSource.Name -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskSource.Value) {
        throw "Exact CPU probe source changed: $($taskSource.Name)"
    }
}
$taskDeps = 'target/s-anim-browser/release/deps'
$taskAnyhowDeps = 'target/s-builder-current-mode/release/deps'
$taskAnimation = (Get-ChildItem "$taskDeps/libaa_animation-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskAssets = (Get-ChildItem "$taskDeps/libaa_assets-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskAnyhow = (Get-ChildItem "$taskAnyhowDeps/libanyhow-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$taskSerde = (Get-ChildItem "$taskDeps/libserde_json-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
rustc --test --edition 2024 "$taskFolder/exact-cpu-adapter.rs" -L "dependency=$taskDeps" -L "dependency=$taskAnyhowDeps" --extern "aa_animation=$taskAnimation" --extern "aa_assets=$taskAssets" --extern "anyhow=$taskAnyhow" --extern "serde_json=$taskSerde" -o "$taskFolder/exact-cpu-adapter-tests.exe" *> "$taskFolder/exact-cpu-build.log"
if ($LASTEXITCODE -ne 0) { Get-Content "$taskFolder/exact-cpu-build.log"; exit $LASTEXITCODE }
& "$taskFolder/exact-cpu-adapter-tests.exe" --nocapture *> "$taskFolder/exact-cpu-tests.log"
$taskExit = $LASTEXITCODE
Get-Content "$taskFolder/exact-cpu-tests.log"
exit $taskExit
