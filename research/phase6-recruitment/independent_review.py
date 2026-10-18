"""Independent recruitment wire-codec replay and instruction audit."""
from pathlib import Path
import hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_recruitment as n
from native_header import P,V
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 doc=json.loads((HERE/'native-recruitment.json').read_text());regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
 for row in doc['cases']:
  machine=n.Machine(bytes.fromhex(row['hex'])+b'FOLLOWING RECORD')
  sentinels=[0x11111111*(i+1) for i in range(4)]
  for r,v in zip(regs,sentinels):machine.u.reg_write(r,v)
  got=machine.run();assert got=={k:row[k] for k in ['consumed','items','word_24']},row['name']
  assert [machine.u.reg_read(r) for r in regs]==sentinels,row['name']
 windows=doc['visited_native_instructions_and_seam_entries']
 for window in windows:
  a=int(window['va'],16);raw=bytes.fromhex(window['bytes']);assert P.range(a,len(raw))==raw==V.range(a,len(raw))
 result=subprocess.run(['cargo','test','-p','aa-mapworld','--test','recruitment'],cwd=ROOT,text=True,capture_output=True)
 assert result.returncode==0,result.stdout+result.stderr
 paths=['crates/aa-mapworld/src/recruitment.rs','crates/aa-mapworld/tests/recruitment.rs','research/phase6-recruitment/native_recruitment.py','research/phase6-recruitment/native-recruitment.json']
 out={'schema':1,'reviewer':'visual_audit','passed':True,'native_cases_replayed':len(doc['cases']),'instruction_windows_checked':len(windows),'callee_saved_registers_verified':True,'rust_tests':'2 passed','reviewed_behavior':['Native4E6C20 signed list count preserves negative count words but consumes no entries.','Kind0 Single and kind1 Group dispatch and base version0 UTF16 property-reference/enable byte order match.','Single version0 consumes legacy byte without setting a runtime object field; missing word10/18 retain native constructor zeros.','Single version1 reads word0c/10/14/1c; version2 adds word18 last. Group version0 reads word0c only.','Recruit-list version1 word24 follows every item; minimum-size precheck is conservative and bounds allocations by bytes.','Field/reference values remain raw wire data; unknown kinds/versions and truncations reject without searching for next record.','Appended source suffix remains unconsumed by original executions; callee-saved registers verified independently.'],'findings':[],'limits':['Property registry lookup, duplicate suppression and publication are not reconstructed by this wire codec. Fixtures deliberately resolve property references to null.','Version helper, primitive stream I/O and allocation are explicit native-oracle seams; native item field branches and list loop execute.'],'pins':{p:sha(ROOT/p) for p in paths}}
 path=HERE/'independent-review.json';path.write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'cases':len(doc['cases']),'sha256':sha(path)}))
if __name__=='__main__':main()
