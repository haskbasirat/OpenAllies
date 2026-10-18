"""Native insertion-order evidence, deliberately not a full TGI parsing oracle."""
from pathlib import Path
import importlib.util, json, struct, re

HERE = Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('economy_oracle',HERE/'economy-phase2-oracle.py')
oracle=importlib.util.module_from_spec(spec);spec.loader.exec_module(oracle)
RANGES={'assign_index_and_append':(0x479fe9,0x47a008),'append_pointer':(0x93d4e0,0x93d50d),
        'increment_count_get_slot':(0x483a20,0x483a3a),'ensure_capacity':(0x47a040,0x47a076),
        'resource_node_loader':(0x479ea0,0x479f60),'manager_vtable':(0xa0c534,0xa0c558),
        'ordinary_parser_append_call':(0x5b74ac,0x5b74e2),
        'header_zero_list_flag':(0x5bc304,0x5bc33e),
        'header_list_keyword_only_setter':(0x5bc4b9,0x5bc4f0),
        'header_constructor_arguments':(0x5bc9d7,0x5bca31),
        'node_flag_constructor':(0x5b7330,0x5b73c6),
        'subnode_add':(0x5b5c80,0x5b5fbd),
        'file_root_visitor':(0x48d660,0x48d840),
        'list_keyword':(0xa4aa00,0xa4aa0a)}


def main():
    uc=oracle.Uc(oracle.UC_ARCH_X86,oracle.UC_MODE_32)
    oracle.ORIGINAL.load(uc)
    uc.mem_map(0x2000000,0x10000);uc.mem_map(0x3000000,0x10000)
    def put(a,v):uc.mem_write(a,struct.pack('<I',v))
    def get(a):return struct.unpack('<I',uc.mem_read(a,4))[0]
    manager=0x2000000;table=0x2001000;stack=0x3008000
    names=['money','ammo','oil','unit_limit_provided','unit_limit_consumed','building_limit_provided','building_limit_consumed']
    rows=[]
    for sequence in [names,list(reversed(names))]:
        put(manager+0x24,table);put(manager+0x28,0);put(manager+0x2c,8)
        objects=[]
        for index,name in enumerate(sequence):
            resource=0x2002000+index*0x100
            put(resource+0xc,0xffffffff)
            uc.reg_write(oracle.UC_X86_REG_EAX,resource)
            uc.reg_write(oracle.UC_X86_REG_ESI,manager)
            uc.reg_write(oracle.UC_X86_REG_ESP,stack)
            uc.emu_start(0x479fe9,0x47a008,count=1000)
            assert uc.reg_read(oracle.UC_X86_REG_EIP)==0x47a008
            assert uc.reg_read(oracle.UC_X86_REG_ESP)==stack
            objects.append(resource)
            assert get(resource+0xc)==index and get(manager+0x28)==index+1
            assert [get(table+i*4) for i in range(index+1)]==objects
            rows.append({'supplied_creation_sequence':sequence,'resource':name,'assigned_index':index,'count_after':index+1})
    ranges={}
    for name,(a,z) in RANGES.items():
        raw=oracle.ORIGINAL.at(a,z-a)
        assert raw==oracle.ACCEPTED.at(a,z-a)
        ranges[name]={'start_va':hex(a),'end_va_exclusive':hex(z),'sha256':oracle.sha(raw),'unchanged_in_v14':True}
    resource_path=oracle.GAME/'reconstruction/assets/original/Data/Game/resources.tgi'
    resource_bytes=resource_path.read_bytes()
    plain=re.sub(r'/\*.*?\*/','',resource_bytes.decode('cp1252'),flags=re.S)
    headers=re.findall(r'^\s*\[Resource\s+([^\]]+)\]',plain,re.M)
    ids=re.findall(r'^\s*IDS\s*=\s*(\w+)',plain,re.M)
    assert ids==names and len(headers)==7
    assert all(h.strip() in ('template=PurchaseResource','template=UpkeepResource','template=LimitedConsumedResource') for h in headers)
    result={'schema':1,'passed':True,'generator_sha256':oracle.sha(Path(__file__).read_bytes()),
        'original_sha256':oracle.ORIGINAL_SHA,'accepted_v14_sha256':oracle.ACCEPTED_SHA,
        'ranges':ranges,'cases':rows,'native_case_count':len(rows),
        'proven':'A successfully created Resource is assigned current manager count then appended; existing-capacity native helpers execute without decision/call hooks.',
        'source_chain':['ResourceManager vtable A0C534+1C dispatches 479EA0','479EA0 calls479F60 for new resource;479FE9 reads manager+28 and479FF4 writes Resource+C','479FF8 selects manager+24 vector;93D4E0->483A20->47A040 appends and increments count'],
        'ordinary_file_order_static_evidence':{
            'resource_source':str(resource_path.relative_to(oracle.GAME)), 'source_sha256':oracle.sha(resource_bytes),
            'active_ids_in_file_order':ids,'active_headers':headers,
            'links':[
                '5BC324 initializes local list flag to0;5BC4CC compares token against List;5BC4E9 is its setter1, guarded by template-context byte.',
                '5BCA16/1D passes that local as constructor argument4;5B73A7..C1 maps it to node flag0x4. The seven active Resource headers have no List token.',
                '5B74C3 passes explicit append-mode0 to AddSubNode5B5C80 at5B74C8.',
                'Same-name plain-node path leaves append-mode0:5B5DD7->5B5DE5->5B5DEC. Existing node hash entries are renamed;5B35A0 removes only hash buckets, not child-array slots.',
                'Subsequent normalized-list lookup increments first-child ordinal count and reaches the same5B5DEC append. Front insertion5D3C60 is only the nonzero-mode path.',
                '48D660 visits root+34 child array from index0 through root+38count; it skips template-flag nodes and dispatches by original unnormalized type name.',
                '48DB60 at48EB32 registers Resource type to GameData+10 ResourceManager; its vtable routes to479EA0 and the tested append chain.'
            ],
            'scope':'Static native control-flow argument for these seven ordinary source headers, plus executed native resource append. Not a complete parser execution or arbitrary-template/mod order guarantee.'
        },
        'limits':['The fixture starts after allocation/constructor/duplicate checks and uses sufficient preallocated capacity.','The full TGI parser is not executed by this fixture; parser branch premises are source/byte evidence.','Insertion order can differ for explicit List/template/front-insertion overrides; no universal authored-order guarantee for arbitrary mods.']}
    out=HERE/'economy-phase2-registry.json';out.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'path':str(out),'sha256':oracle.sha(out.read_bytes()),'cases':len(rows)}))


if __name__=='__main__':main()
