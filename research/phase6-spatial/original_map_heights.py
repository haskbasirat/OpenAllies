"""Pin every actual map input and execute native samplers on selected raw maps."""
from pathlib import Path
import json,struct,sys
from original_height_binding import ROOT,OUT,BindingMachine,bits,sha,original,modified
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from inspect_header import decode
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools'))
from map_extract import read_chunks
SOURCE=ROOT.parent/'reconstruction/assets/original/Data'

def main():
    index=json.loads((ROOT.parent/'reconstruction/game/data/maps/index.json').read_text())
    svars=SOURCE/'Game/SVars.tgi';assert b'int HeightmapResolutionBits = 1' in svars.read_bytes()
    maps={};samples=[];visited={}
    for row in index:
        path=SOURCE/row['source'];chunks=read_chunks(path);header=decode(chunks['MAP_HEADER'][0])
        extent=header['creator_size_bits']
        maps[row['source']]={'map_sha256':sha(path.read_bytes()),'extent_bits':extent,
            'chunks':{name:sha(chunks[name][0])for name in ['MAP_HEADER','HEIGHT_MAP','MOUNTAIN_HEIGHT_MAP']}}
        if row['id'] not in ['Campaign/axis01_crete','Campaign/tutorial01']:continue
        for layer in ['HEIGHT_MAP','MOUNTAIN_HEIGHT_MAP']:
            machine=BindingMachine();state=machine.initialize(extent,1,bits(-10),bits(14),bits(2)if layer=='HEIGHT_MAP' else 0)
            loaded=machine.load(chunks[layer][0]);w,h=state['width'],state['height'];values=loaded['state']['height_bits']
            max_index=max(range(len(values)),key=lambda i:struct.unpack('<f',struct.pack('<I',values[i]))[0])
            x,y=max_index%w,max_index//w
            positions=[[0,0],[1.25,6.75],[(w//2)*2+0.5,(h//2)*2+1.25],[(w-1)*2,(h-1)*2],[x*2,y*2],[x*2+0.75,y*2+0.5]]
            for precision in ['Single24','Double53','Extended64']:
                for method in ['Nearest','Interpolated']:
                    for position in positions:
                        xy=list(map(bits,position))
                        samples.append({'source':row['source'],'layer':layer,'position_bits':xy,'precision':precision,'method':method,'output':machine.sample(method,xy,precision)})
            visited.update(machine.visited)
    pins=[]
    for pc,size in sorted(visited.items()):
        raw=original.range(pc,size);assert raw==modified.range(pc,size)
        pins.append({'address':hex(pc),'bytes':size,'sha256':sha(raw)})
    report={'schema':1,'passed':True,'resolution_bits':1,'resolution_source':{'path':'Game/SVars.tgi','sha256':sha(svars.read_bytes())},'maps':maps,'samples':samples,'executed_native_instructions':pins,'limits':['MAP_HEADER extents are parsed independently from the native-offset decoder; its own execution evidence is separate.','No dynamic height edits, tile ownership, collision or automatic precision default is inferred.']}
    path=OUT/'native-real-map-heights.json';path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'maps':len(maps),'native_samples':len(samples),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
