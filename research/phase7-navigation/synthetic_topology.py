"""Small allocator/stream-only native topology cases for portable Rust tests."""
from pathlib import Path
import copy,json,struct
from topology_oracle import Machine,H,OUT,original,modified
W=lambda *v:struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v))
def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def encode(regions,rectangles,enabled=True):
 out=W(0)+bytes([enabled])
 if not enabled:return out
 out+=W(len(regions))
 for center,pairs in regions:out+=W(0,*map(bits,center),len(pairs))+b''.join(W(*p)for p in pairs)
 out+=W(len(rectangles))
 for region,island,bounds,sides in rectangles:
  out+=W(0,region,island)
  for bound,links in zip(bounds,sides):
   out+=struct.pack('<h',bound)+W(len(links))
   for j,neighbor in enumerate(links):out+=W(0)+struct.pack('<4H',j,j+1,j+2,j+3)+W(neighbor)
 return out
def main():
 layout={'width':8,'height':8,'coordinate_shift':1,'cell_size_bits':bits(2),'extent_i32':[16,16]}
 regions=[[(4,4),[]],[(12,4),[]],[(4,12),[]],[(12,12),[]]]
 rects=[[0,0,[0,3,3,0],[[1,1],[2],[],[0]]],[1,0,[0,7,3,4],[[0],[],[3],[]]],[2,0,[4,3,7,0],[[],[0],[],[3]]],[3,0,[4,7,7,4],[[1],[],[2],[]]]]
 cases=[('four_regions',regions,rects,True)]
 changed=copy.deepcopy(rects);changed[0][3]=[[2,1,2],[],[0],[]];cases.append(('ordered_duplicate_sides',regions,changed,True))
 changed=copy.deepcopy(rects);changed.append([0,0,[1,2,2,1],[[3],[],[],[]]]);cases.append(('overlap_last_publication',regions,changed,True))
 changed=copy.deepcopy(rects);changed[2][1]=1;changed[3][1]=1;cases.append(('two_islands',regions,changed,True))
 changed=copy.deepcopy(regions);changed[0][1]=[[i,bits(i*.125)]for i in range(30)]+[[3,bits(9.75)],[3,bits(-.25)]];cases.append(('distance_hash_grow_duplicates',changed,rects,True))
 changed=copy.deepcopy(rects)
 for row in changed:row[3]=[[],[],[],[]]
 cases.append(('disconnected',regions,changed,True));cases.append(('disabled',[],[],False))
 fixtures=[];visited={}
 for p in ['Single24','Double53','Extended64']:
  for name,rs,rectangles,enabled in cases:
   data=encode(rs,rectangles,enabled);flags=[0x200]*64;m=Machine(data,layout,flags,p);native=m.run();visited.update(m.executed)
   fixtures.append({'name':name+'_'+p,'precision':p,'input_hex':data.hex(),'layout':layout,'world_version':28,'flags20':flags,'native':native})
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'oracle_sha256':H((OUT/'topology_oracle.py').read_bytes()),'fixtures':fixtures,'native_basic_block_pins':pins}
 (OUT/'synthetic-topology.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'cases':len(fixtures)})
if __name__=='__main__':main()
