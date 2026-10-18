"""Untouched source virtual+28=49BD10 expansion, ObjectData vtable, all PCs."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image,pc):
    h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
    d=m.alloc(0x200);out=m.alloc(24);data=m.alloc(0x100);clear=m.alloc(4)
    m.w32(d,0xa13318);m.w32(d+0x1c,0x20000400);m.w32(d+0x38,*c['shape'])
    m.w32(0xb39ab0,data);m.w32(data+0x44,clear);m.w32(clear,c['clearance'])
    result=m.call(0x49bd10,d,[out]);assert result==out;assert m.fpu_top()==0
    return m.r32(out,6)
def cases():
    shapes=[[0,0,0,0,0,0],[1,h.f2b(1.3),0,0,0,0],
            [2,h.f2b(2**.5),h.f2b(-1),h.f2b(-1),h.f2b(1),h.f2b(1)],
            [3,h.f2b(5),h.f2b(-4),h.f2b(-3),h.f2b(4),h.f2b(3)]]
    for shape in shapes:
        for clearance in [0,.1,.125,2.7,1e-5,100000]:
            yield dict(shape=shape,clearance=h.f2b(clearance))
def main():
    rows=[]
    for pc in h.PRECISION:
        for c in cases():
            a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b
            rows.append(dict(precision=pc,input=c,output=a))
    result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=rows,
                native_executions=len(rows)*2,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-creation-shape.json'
    if '--write'in sys.argv:path.write_text(json.dumps(result,indent=1)+'\n')
    else:assert json.loads(path.read_text())==result
    print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()
