"""Independent replay and adapter checks; does not rewrite the owner's fixture."""
from pathlib import Path
import hashlib,json,os,subprocess,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
producer=HERE/'native_kingdom_flow_init.py';namespace={'__file__':str(producer)}
exec(producer.read_text().split('rows = []')[0],namespace)
Machine,P,V=namespace['Machine'],namespace['P'],namespace['V']
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EDI,UC_X86_REG_ESI,UC_X86_REG_EIP
fixture=HERE/'native-kingdom-flow-init.json';raw=fixture.read_bytes()
assert hashlib.sha256(raw).hexdigest()=='666b0ab074bfd640d737d6119c572856c728f6fe5f8f2610632ea4c60ecadcd3'
data=json.loads(raw)
for instruction in data['instructions']:
 a=int(instruction['va'],16);code=bytes.fromhex(instruction['bytes']);assert P.range(a,len(code))==code==V.range(a,len(code))
for case in reversed(data['cases']):
 m=Machine(0x27f);m.w(0xb36530,0x2250000);m.w(0x2250038,case['resource_count']&0xffffffff)
 for off,before in zip([0x1a0,0x1ac,0x1b8],case['before']):
  pointer=m.alloc(before['capacity']*4)if before['capacity']else 0
  for i,value in enumerate(before['values']):m.w(pointer+i*4,value)
  for i,value in enumerate([pointer,len(before['values']),before['capacity']]):m.w(m.obj+off+i*4,value)
 m.w(m.stack+8,m.obj);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EDI,m.obj);m.u.reg_write(UC_X86_REG_ESI,0);m.u.emu_start(0x4e5eab,0x4e5fb5,count=100000);assert m.u.reg_read(UC_X86_REG_EIP)==0x4e5fb5
 for off,after in zip([0x1a0,0x1ac,0x1b8],case['after']):
  ptr,count,capacity=[m.r(m.obj+off+i*4)for i in range(3)];assert {'values':[m.r(ptr+i*4)for i in range(count)],'capacity':capacity}==after
command=['cargo','test','-p','aa-runtime','--test','kingdom_economy','--test','scheduler_owners','--','--include-ignored','--nocapture']
env=os.environ.copy();env['AA_GAME_ROOT']=str(ROOT.parent);started=time.monotonic();result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True)
log=HERE/'kingdom-economy-independent-tests.txt';log.write_text(result.stdout+result.stderr,encoding='utf8')
report={'passed':result.returncode==0,'native_cases_replayed_in_reverse_order':len(data['cases']),'instructions_verified':len(data['instructions']),'command':command,'cwd':str(ROOT),'environment':{'AA_GAME_ROOT':str(ROOT.parent)},'seconds':round(time.monotonic()-started,3),'exit_code':result.returncode,'log':str(log.relative_to(ROOT)),'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
(HERE/'kingdom-economy-independent-checks.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report));raise SystemExit(result.returncode)
