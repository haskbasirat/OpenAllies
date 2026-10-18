"""Additional native valid-boundary queue cases for the independent review."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-runtime'))
import native_saved_queues as saved
def words(values):return struct.pack('<'+'I'*len(values),*[v&0xffffffff for v in values])
def main():
 data=words([2,0,0,2,4,-1,10,0,11,1,12,0,13,-7,0,1,2,0,0])
 m=saved.Machine(data);m.write(m.read(m.table+4)+4,0);before=m.read(m.world+0xd4);m.load(21)
 assert m.snapshot()[3]['buckets']==[[[0,11],[0,13]],[]]
 assert m.snapshot()[4]['current']==1
 assert m.read(m.world+0xd4)==before and 0x85d840 not in saved.q.VISITED
 report={'passed':True,'payload':data.hex(),'world_version':21,'null_slots':[1],'snapshot':m.snapshot(),'boundaries':m.bound,'rng_before':before,'rng_after':m.read(m.world+0xd4),'scope':'Original signed negative list count skips records; null ID and unbound slot are discarded while retained entries keep source order.'}
 p=HERE/'native-edges.json';p.write_text(json.dumps(report,indent=2)+'\n');print('PASS',hashlib.sha256(p.read_bytes()).hexdigest())
if __name__=='__main__':main()
