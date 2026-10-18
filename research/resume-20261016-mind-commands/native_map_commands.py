"""Run original 536340/52C580/4DACE0 over exact exported real-map Mind payloads.

Registry lookup remains an explicit named-definition seam. This proves saved
field parsing, byte boundaries and command ownership, not TGI registry validity.
The companion runtime integration resolves actual authored definitions.
"""
from pathlib import Path
import json, hashlib
from native_commands import Machine, H, P, V, VISITED, HERE, ROOT
from native_header import read_chunks

def main():
    path=HERE/'map-command-corpus.jsonl'
    cases=[]
    names={};bytes_seen=set();maps=set();commands=0;chunks={};source_hashes={}
    for line in path.read_text().splitlines():
        source=json.loads(line);payload=bytes.fromhex(source['mind_payload_hex'])
        if source['map'] not in chunks:
            map_path=ROOT.parent/'reconstruction/assets/original/Data/Maps'/source['map']
            chunks[source['map']]=read_chunks(map_path)['DYNAMIC_THINGS'][0]
            source_hashes[source['map']]=H(map_path.read_bytes())
        at=source['source_offset'];length=source['source_length']
        assert length==len(payload)
        # Embedded source_span currently carries a local origin despite its
        # absolute-span documentation. Verify the entire already-decoded
        # payload occurs exactly once in the original chunk; do not discover
        # field boundaries by scanning for names or byte prefixes.
        verified=chunks[source['map']].find(payload)
        assert verified>=0 and chunks[source['map']].find(payload,verified+1)<0, (source['map'],source['saved_id'],'unique source bytes')
        m=Machine(payload+b'AFTER');native=m.mind()
        assert native['consumed']==len(payload),(source['map'],source['saved_id'])
        assert native['distinct_allocations']
        maps.add(source['map']);bytes_seen.add(payload)
        for command in native['pending']+([native['current']] if native['current'] else []):
            assert command['reference_count']==1
            commands+=1
            key=None if command['definition'] is None else ''.join(map(chr,command['definition']))
            names[key]=names.get(key,0)+1
        cases.append({'source':{**{key:source[key] for key in ['map','saved_id','definition']},
                                 'reported_offset':at,'verified_offset':verified},
                      'payload':payload.hex(),'native':native})
    assert cases, 'The embedded command corpus must not be empty'
    doc={'schema':1,'scope':__doc__,'source_sha256':H(path.read_bytes()),
         'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),
         'maps':len(maps),'unique_payloads':len(bytes_seen),'command_count':commands,
         'command_names':names,'map_sha256':source_hashes,'cases':cases,
         'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
    out=HERE/'native-map-commands.json';out.write_text(json.dumps(doc,indent=1)+'\n')
    print(json.dumps({'cases':len(cases),'maps':len(maps),'commands':commands,
                      'unique_payloads':len(bytes_seen),'names':names,'sha256':H(out.read_bytes())}))

if __name__=='__main__':main()
