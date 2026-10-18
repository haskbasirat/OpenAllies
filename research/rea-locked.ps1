# Run the REA CLI with a machine-wide lock so parallel sessions never start
# two Ghidra analyses at once (each needs several GB of RAM and minutes of
# import). Usage, from any directory:
#   & 'E:\Games\Axis and Alies\rust-rebuild\research\rea-locked.ps1' decompile 'E:\Games\Axis and Alies\AA.exe' 0x005519a0 --provider ghidra --json
# Also wrap any direct Ghidra analyzeHeadless run in the same mutex.
$rea = 'E:\Games\Axis and Alies\reconstruction\tools\rea\rea.ps1'
$mutex = New-Object System.Threading.Mutex($false, 'Global\AxisAlliesREA')
try {
    try { [void]$mutex.WaitOne() } catch [System.Threading.AbandonedMutexException] { }
    & $rea @args
} finally {
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
