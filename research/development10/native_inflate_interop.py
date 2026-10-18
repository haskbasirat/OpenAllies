"""Compatibility of Rust TGCK zlib payloads with original zlib1.1.4 inflater.
Representation compatibility, not original deflater byte identity. Raw original
908210/908230 execute; only caller-provided allocation/free callbacks are seams.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
INPUT=HERE/'compressed-chunks-rust-01.json'
def run(raw,expected,precision,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(precision)
 stream=m.alloc(0x38);version=m.alloc(8);m.u.mem_write(version,b'1.1.4\0')
 alloc=h.STUBS+0x1400;free=h.STUBS+0x1410;allocations=[]
 def allocate(mm):
  count,size=mm.arg(1),mm.arg(2);n=count*size;allocations.append(n);mm.ret(0,mm.alloc(n))
 def release(mm):mm.ret(0)
 m.seams.update({alloc:allocate,free:release});m.w32(stream+0x20,alloc,free,0)
 assert m.call(0x908210,0,[stream,version,0x38])==0
 data=m.alloc(len(raw));m.u.mem_write(data,raw)
 target=m.alloc(len(expected)+32)
 m.w32(stream,data,len(raw),0,target,len(expected)+32,0)
 status=m.call(0x908230,0,[stream,0],count=50_000_000)
 assert status==1,(status,len(raw),len(expected))
 assert m.r32(stream+4)==0 and m.r32(stream+8)==len(raw)
 assert m.r32(stream+0x14)==len(expected)
 actual=bytes(m.u.mem_read(target,len(expected))) if expected else b''
 assert actual==expected
 return dict(status=status,total_in=m.r32(stream+8),total_out=m.r32(stream+0x14),
  output_sha256=hashlib.sha256(actual).hexdigest(),allocations=allocations)
def main():
 source=json.loads(INPUT.read_text());rows=[]
 for case in source['cases']:
  for chunk in case['chunks']:
   if not chunk['compressed']:continue
   raw=bytes.fromhex(chunk['stored']);expected=bytes.fromhex(chunk['payload'])
   for precision in h.PRECISION:
    a=run(raw,expected,precision,'original');b=run(raw,expected,precision,'v14');assert a==b
    rows.append(dict(case=case['case'],chunk=chunk['name'],precision=precision,output=a))
 out=dict(schema=1,scope=__doc__,input_sha256=hashlib.sha256(INPUT.read_bytes()).hexdigest(),rows=rows,
  native_executions=len(rows)*2,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-inflate-interop.json'
 if '--write' in sys.argv:path.write_text(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
