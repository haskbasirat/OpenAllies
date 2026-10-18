"""Keep raw emulator results, add independent hardware-backed expected values."""
from pathlib import Path
import hashlib,json,struct,subprocess,tempfile
HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=json.loads((HERE/'remainder-vectors.json').read_text())
source=HERE/'host_remainder.rs';exe=HERE/'host_remainder.exe'
subprocess.run(['rustc','--edition=2024','-O',str(source),'-o',str(exe)],check=True)
# New named capture files stay in the research directory as evidence; no cleanup.
inputs=HERE/'host-remainder-input.bin';output=HERE/'host-remainder-output.bin'
inputs.write_bytes(b''.join(struct.pack('<IB3xQQ',{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[r['precision']],r['kind']=='f64',*r['inputs']) for r in report['cases']))
subprocess.run([str(exe),str(inputs),str(output)],check=True)
captured=output.read_bytes();assert len(captured)==len(report['cases'])*14
differences=[]
for i,row in enumerate(report['cases']):
 raw=captured[i*14:i*14+14]
 row['unicorn_result80']=row['result80'];row['unicorn_result32']=row['result32']
 row['result80']=raw[:10].hex();row['result32']=struct.unpack('<I',raw[10:])[0]
 if row['result80']!=row['unicorn_result80'] or row['result32']!=row['unicorn_result32']:
  differences.append({'index':i,**row})
report.update(scope='Actual host x87 FPREM loop expected values, with original Unicorn observations retained. Synthetic arithmetic only; no whole game claim.',
              host='Windows x86_64 Intel Core i7-11700KF',raw_unicorn_fixture_sha256=sha(HERE/'remainder-vectors.json'),
              capture_producer_sha256=sha(Path(__file__)),host_source_sha256=sha(source),host_executable_sha256=sha(exe),
              input_sha256=sha(inputs),output_sha256=sha(output),unicorn_mismatches=len(differences))
p=HERE/'remainder-hardware-vectors.json';p.write_text(json.dumps(report,indent=2),encoding='utf-8')
(HERE/'remainder-unicorn-discrepancies.json').write_text(json.dumps({'cases':differences,'count':len(differences),
 'conclusion':'Unicorn FPREM is not a reliable oracle for these wide-exponent ratios. Exact integer remainder and actual hardware are the acceptance references.'},indent=2),encoding='utf-8')
print(json.dumps({'cases':len(report['cases']),'unicorn_mismatches':len(differences),'hardware_fixture_sha256':sha(p)}))
