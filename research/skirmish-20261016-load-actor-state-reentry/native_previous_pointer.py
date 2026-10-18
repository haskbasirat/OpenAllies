"""4D4A90 reuse: previous descriptor is unread on the supported Org-absent path.

Reuses all111 immutable entry inputs and runs original/v14 with a deliberately
unmapped nonnull old-descriptor argument. No instruction or body is replaced.
The current supported adapter rejects Organization57C400 before this argument
would be used. This does not certify that positive Organization transition.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import native_state_entry as entry
H=lambda b:hashlib.sha256(b).hexdigest()
PREVIOUS=0xdead0001

def main():
 reference=HERE/'native-state-entry.json';f=json.loads(reference.read_text())
 original_call=entry.h.Machine.call
 def with_previous(self,address,ecx=0,args=(),count=2_000_000):
  if address==0x4d4a90:
   assert len(args)==2 and args[1]==0
   args=(args[0],PREVIOUS)
  return original_call(self,address,ecx,args,count)
 entry.h.Machine.call=with_previous
 rows=[]
 try:
  for row in f['cases']+f['actual_rows']:
   result=[]
   for image in (entry.h.P,entry.h.V):
    value=entry.run(row['input'],row['precision'],image)
    assert value==row['output']
    result.append(value)
   assert result[0]==result[1]
   rows.append(dict(precision=row['precision'],input=row['input'],previous_argument=hex(PREVIOUS),output=result[0]))
 finally:entry.h.Machine.call=original_call
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(entry.h.VISITED.items())]
 assert not any(int(p['va'],16)in entry.h.ALLOCATORS for p in pins)
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),input_fixture_sha256=H(reference.read_bytes()),entry_producer_sha256=H((HERE/'native_state_entry.py').read_bytes()),original_sha256=entry.h.ORIGINAL_SHA,v14_sha256=entry.h.V14_SHA,cases=rows,machines=2*len(rows),executed_pins=pins,seams={})
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-previous-pointer.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
