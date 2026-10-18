"""Actual 554DD0/554330/5541F0 geometry; no list/zone seams or SDK source."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image,pc):
    h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
    map_=m.alloc(0x100);shape=m.alloc(24);placement=m.alloc(24);it=m.alloc(44)
    m.w32(map_+4,2,h.f2b(4),h.f2b(32),h.f2b(32));m.w32(map_+0x1c,8,8)
    m.w32(map_+0x30,*c['cell_shape']);m.w32(0xb3a8c4,*c['global_point_shape'])
    m.w32(shape,*c['shape']);m.w32(placement,shape,*c['facing'],*c['position'],0)
    m.call(0x554dd0,it,[map_,placement,c['mode'],0]);assert m.fpu_top()==0
    rectangle=m.r32(it+4,4);cells=[]
    while m.r32(it+0x28)<=m.r32(it+0x10):
        cells.append(m.r32(it+0x24,2));assert len(cells)<=64
        m.call(0x554330,it,[0]);assert m.fpu_top()==0
    return {'cells':cells,'rectangle':rectangle}
def cases():
    # Full-cell square matches native556689..696, radius sqrt(8).
    cell=[2,h.f2b(8**.5),h.f2b(-2),h.f2b(-2),h.f2b(2),h.f2b(2)]
    shapes=[[0,0,0,0,0,0],[1,h.f2b(.1),0,0,0,0],[1,h.f2b(3.2),0,0,0,0],
            [3,h.f2b(5),h.f2b(-4),h.f2b(-3),h.f2b(4),h.f2b(3)]]
    for shape in shapes:
      for facing in [[0,0xbf800000],[0x3f800000,0],[h.f2b(.6),h.f2b(.8)]]:
       for position in [[0,0],[h.f2b(12),h.f2b(12)],[h.f2b(10.3),h.f2b(12.8)],[h.f2b(31.99),h.f2b(31.99)],
                        [h.f2b(-1),h.f2b(10)],[h.f2b(33),h.f2b(12)]]:
        for mode in [0,1]:
            yield dict(shape=shape,facing=facing,position=position,cell_shape=cell,mode=mode,global_point_shape=[0]*6)
def main():
    rows=[]
    for pc in h.PRECISION:
      for c in cases():
        a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b
        rows.append(dict(precision=pc,input=c,output=a))
    result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),
                cases=rows,native_executions=len(rows)*2,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-placement-tiles.json'
    if '--write'in sys.argv:path.write_text(json.dumps(result,indent=1)+'\n')
    else:assert json.loads(path.read_text())==result
    print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()
