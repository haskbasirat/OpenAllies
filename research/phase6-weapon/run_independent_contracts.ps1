$ErrorActionPreference = 'Stop'
$reviewWorkspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location -LiteralPath $reviewWorkspace
try {
    cargo test -p aa-combat --no-run
    if ($LASTEXITCODE -ne 0) { throw 'Building current combat test libraries failed' }
    $combatLib = Get-ChildItem -LiteralPath target/debug/deps -Filter 'libaa_combat-*.rlib' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $bodyLib = Get-ChildItem -LiteralPath target/debug/deps -Filter 'libaa_body-*.rlib' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $numericLib = Get-ChildItem -LiteralPath target/debug/deps -Filter 'libaa_numeric-*.rlib' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    rustc --test --edition 2024 research/phase6-weapon/independent_contracts.rs --extern "aa_combat=$($combatLib.FullName)" --extern "aa_body=$($bodyLib.FullName)" --extern "aa_numeric=$($numericLib.FullName)" -L dependency=target/debug/deps -o research/phase6-weapon/independent-contracts.exe
    if ($LASTEXITCODE -ne 0) { throw 'Compiling reviewer contract tests failed' }
    & ./research/phase6-weapon/independent-contracts.exe --nocapture 2>&1 | Tee-Object -FilePath research/phase6-weapon/independent-contracts.log
    if ($LASTEXITCODE -ne 0) { throw 'Reviewer contract tests failed' }
} finally { Pop-Location }
