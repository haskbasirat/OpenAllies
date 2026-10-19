"""Sample a running client's memory every N seconds until it exits.

Records process working set / private bytes (PowerShell Get-Process) and the
GPU dedicated/shared usage of the process (Windows performance counters
"GPU Process Memory"), as JSON lines. Read-only observation of our own
child process; nothing is modified.
usage: sample_memory.py <pid> <out.jsonl> [--every 30]
"""
import json, subprocess, sys, time

pid = int(sys.argv[1]); out = sys.argv[2]
every = int(sys.argv[sys.argv.index('--every') + 1]) if '--every' in sys.argv else 30
PS = r"""
$p = Get-Process -Id {pid} -ErrorAction SilentlyContinue
if ($null -eq $p) {{ 'gone'; exit }}
$ded = 0; $sha = 0
try {{
  $c = Get-Counter -Counter '\GPU Process Memory(pid_{pid}*)\Dedicated Usage','\GPU Process Memory(pid_{pid}*)\Shared Usage' -ErrorAction Stop
  foreach ($s in $c.CounterSamples) {{ if ($s.Path -like '*dedicated*') {{ $ded += $s.CookedValue }} else {{ $sha += $s.CookedValue }} }}
}} catch {{ }}
'{{0}} {{1}} {{2}} {{3}} {{4}}' -f $p.WorkingSet64, $p.PrivateMemorySize64, $ded, $sha, $p.TotalProcessorTime.TotalSeconds
"""
start = time.monotonic()
with open(out, 'a', encoding='utf-8') as f:
    while True:
        r = subprocess.run(['powershell', '-NoProfile', '-Command', PS.format(pid=pid)], capture_output=True, text=True)
        line = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else 'gone'
        if line == 'gone':
            break
        ws, priv, ded, sha, cpu = line.split()
        f.write(json.dumps(dict(t=round(time.monotonic() - start), working_set=int(ws), private=int(priv),
                                gpu_dedicated=int(float(ded)), gpu_shared=int(float(sha)), cpu_seconds=float(cpu))) + '\n')
        f.flush()
        time.sleep(every)
