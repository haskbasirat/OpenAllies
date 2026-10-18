"""Independently unpack three original RMPs and execute their SUN/MISC bytes.
Ctor inputs are explicit diagnostic values here; startup seed authority is the
separate sealed Windows capture. These maps make no named registry calls.
"""
from pathlib import Path
import sys,struct,json,zlib,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE));import native_saved_sun as sun
sys.path.insert(0,str(HERE.parent/'saved-misc'));import native_saved_misc as misc
h=sun.h
H=lambda b:hashlib.sha256(b).hexdigest()
def chunks(raw):
    assert raw[:8]==b'TGCK\x02\0\0\0';n=struct.unpack_from('<I',raw,8)[0];out={}
    for i in range(n):
        e=raw[len(raw)-n*96+i*96:len(raw)-n*96+(i+1)*96]
        name=e[:64].decode('utf-16-le').split('\0')[0];off,size,_,length=struct.unpack_from('<QQQQ',e,64)
        packed=raw[off:off+size]
        try:payload=zlib.decompress(packed)
        except zlib.error:payload=packed
        assert len(payload)==length
        out[name]=payload
    return out
def main():
    reports=json.loads((HERE/'actual-map-codecs-v2.json').read_text())
    sun_rows=[];misc_rows=[]
    for report in reports:
        path=ROOT.parent/'reconstruction/assets/original/Data/Maps'/(report['map']+'.RMP')
        raw=path.read_bytes();ds=chunks(raw)
        assert ds['SUN'].hex()==report['sun_payload']
        version=struct.unpack_from('<I',ds['GWORLD'])[0]
        for build,image in [('original',sun.ORIGINAL),('v14',sun.V14)]:
            h.P=image
            for pc in h.CW:
                c=dict(name=report['map'],payload=ds['SUN'].hex(),colors=[0,0,0],seed=False,
                       table_word0=report['bound'],occupied=[])
                result=sun.SunMachine(c,pc).run()
                assert result['outcome'] is None and result['consumed']==len(ds['SUN'])
                for field in ['state','registered','events']:assert result[field]==report[field],(report['map'],field)
                sun_rows.append(dict(build=build,precision=pc,map=report['map'],map_sha256=H(raw),
                                     payload_sha256=H(ds['SUN']),input=c,native=result))
                c=dict(name=report['map'],payload=ds['MISC_DATA'].hex(),version=version,speed=0x3f800001,seed=False)
                result=misc.MiscMachine(c,pc).run()
                assert result['outcome'] is None and result['consumed']==len(ds['MISC_DATA'])
                assert result['events']==[]
                misc_rows.append(dict(build=build,precision=pc,map=report['map'],map_sha256=H(raw),
                                      payload_sha256=H(ds['MISC_DATA']),input=c,native=result))
    h.P=sun.ORIGINAL
    for directory,filename,rows in [(HERE,'native-actual-sun.json',sun_rows),
                                  (HERE.parent/'saved-misc','native-actual-misc.json',misc_rows)]:
        out=dict(schema=1,scope='original three-map codec inputs; no whole-world or startup constructor claim',
                 producer_sha256=H(Path(__file__).read_bytes()),sun_producer_sha256=H(Path(sun.__file__).read_bytes()),
                 misc_producer_sha256=H(Path(misc.__file__).read_bytes()),rows=rows)
        path=directory/filename;path.write_text(json.dumps(out,indent=2)+'\n')
        print(json.dumps(dict(path=str(path),rows=len(rows),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()
