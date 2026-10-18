"""Capture actual Windows/WOW64 source-load FP state in a fresh QA runtime.

Only a newly created debuggee is instrumented. Int3 bytes are in-memory only.
The SHGetFolderPathW result buffer is redirected to this QA folder before the
game builds any profile paths. No active profile is opened. Terminate this
owned debuggee after the source-manager return; never attach to another game.
"""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import argparse
import hashlib
import json
import os
import shutil
import struct
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/native-game/runtime-expanded-v14'
EXPECTED = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'


def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


class STARTUP(C.Structure):
    _fields_ = [('cb', W.DWORD), ('reserved', W.LPWSTR), ('desktop', W.LPWSTR), ('title', W.LPWSTR),
        *[(n, W.DWORD) for n in ['x','y','xs','ys','xc','yc','fill','flags']],
        ('show', W.WORD), ('reserved2n', W.WORD), ('reserved2', C.c_void_p),
        ('stdin', W.HANDLE), ('stdout', W.HANDLE), ('stderr', W.HANDLE)]


class PROCESS(C.Structure):
    _fields_ = [('process', W.HANDLE), ('thread', W.HANDLE), ('pid', W.DWORD), ('tid', W.DWORD)]


class FLOAT(C.Structure):
    _fields_ = [(n, W.DWORD) for n in ['ControlWord','StatusWord','TagWord','ErrorOffset','ErrorSelector','DataOffset','DataSelector']] + [('registers', C.c_ubyte*80),('Cr0NpxState',W.DWORD)]


class CONTEXT(C.Structure):
    _fields_ = [('flags',W.DWORD)] + [(n,W.DWORD) for n in ['Dr0','Dr1','Dr2','Dr3','Dr6','Dr7']] + [('floating',FLOAT)] + [(n,W.DWORD) for n in ['SegGs','SegFs','SegEs','SegDs','Edi','Esi','Ebx','Edx','Ecx','Eax','Ebp','Eip','SegCs','EFlags','Esp','SegSs']] + [('extended',C.c_ubyte*512)]


class EVENT(C.Structure):
    _fields_ = [('code',W.DWORD),('pid',W.DWORD),('tid',W.DWORD),('padding',W.DWORD),('payload',C.c_ubyte*160)]


def prepare(folder):
    assert not folder.exists(), 'Use a fresh capture directory'
    assert sha(NATIVE/'AA.exe') == EXPECTED
    folder.mkdir(parents=True)
    originals = {}
    for source in NATIVE.rglob('*'):
        if not source.is_file():
            continue
        relative = source.relative_to(NATIVE)
        # Logs, user profiles and save files are intentionally not imported.
        if any(part.lower() in ['logs','userdata','saves','saved games','profiles'] for part in relative.parts):
            continue
        dest = folder/'runtime'/relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        if source.suffix.lower() == '.rwd':
            originals[str(source)] = sha(source)
        shutil.copy2(source, dest)
        if source.suffix.lower() == '.rwd':
            assert not os.path.samefile(source, dest)
            dest.chmod(0o444)
    for name in ['personal','common','other']:
        (folder/name).mkdir()
    return originals


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--folder',type=Path,required=True)
    parser.add_argument('--seconds',type=int,default=90)
    args=parser.parse_args()
    folder=args.folder.resolve()
    assert ROOT.resolve() in folder.parents
    original_hashes=prepare(folder)
    kernel=C.WinDLL('kernel32',use_last_error=True)
    def api(name,restype,argtypes):
        f=getattr(kernel,name);f.restype=restype;f.argtypes=argtypes;return f
    create=api('CreateProcessW',W.BOOL,[W.LPCWSTR,W.LPWSTR,C.c_void_p,C.c_void_p,W.BOOL,W.DWORD,C.c_void_p,W.LPCWSTR,C.POINTER(STARTUP),C.POINTER(PROCESS)])
    wait=api('WaitForDebugEvent',W.BOOL,[C.POINTER(EVENT),W.DWORD])
    cont=api('ContinueDebugEvent',W.BOOL,[W.DWORD,W.DWORD,W.DWORD])
    get=api('Wow64GetThreadContext',W.BOOL,[W.HANDLE,C.POINTER(CONTEXT)])
    put=api('Wow64SetThreadContext',W.BOOL,[W.HANDLE,C.POINTER(CONTEXT)])
    read=api('ReadProcessMemory',W.BOOL,[W.HANDLE,C.c_void_p,C.c_void_p,C.c_size_t,C.POINTER(C.c_size_t)])
    write=api('WriteProcessMemory',W.BOOL,[W.HANDLE,C.c_void_p,C.c_void_p,C.c_size_t,C.POINTER(C.c_size_t)])
    flush=api('FlushInstructionCache',W.BOOL,[W.HANDLE,C.c_void_p,C.c_size_t])
    suspend=api('SuspendThread',W.DWORD,[W.HANDLE])
    resume=api('ResumeThread',W.DWORD,[W.HANDLE])
    terminate=api('TerminateProcess',W.BOOL,[W.HANDLE,W.UINT])
    close=api('CloseHandle',W.BOOL,[W.HANDLE])
    join=api('WaitForSingleObject',W.DWORD,[W.HANDLE,W.DWORD])
    def check(ok):
        if not ok: raise C.WinError(C.get_last_error())
    assert C.sizeof(CONTEXT)==716 and C.sizeof(EVENT)==176
    process=PROCESS();startup=STARTUP();startup.cb=C.sizeof(startup);startup.flags=1;startup.show=0
    executable=folder/'runtime/AA.exe'
    command=C.create_unicode_buffer('"'+str(executable)+'"')
    check(create(str(executable),command,None,None,False,2,None,str(executable.parent),C.byref(startup),C.byref(process)))
    threads={process.tid:process.thread};points={};stepping={};events=[];captures=[];manager=None;completed=False;extra_handles=set()
    def memory(address,length):
        b=C.create_string_buffer(length);done=C.c_size_t();check(read(process.process,address,b,length,C.byref(done)));assert done.value==length;return b.raw
    def word(address):return struct.unpack('<I',memory(address,4))[0]
    def change(address,bytes_):
        done=C.c_size_t();b=C.create_string_buffer(bytes_);check(write(process.process,address,b,len(bytes_),C.byref(done)));assert done.value==len(bytes_);check(flush(process.process,address,len(bytes_)))
    def breakpoint(address,label):
        if address not in points:
            points[address]=(memory(address,1),label);change(address,b'\xcc')
    def context(tid):
        c=CONTEXT();c.flags=0x1003f;check(get(threads[tid],C.byref(c)));return c
    def wide(address):
        raw=bytearray()
        for offset in range(0,1024,2):
            pair=memory(address+offset,2)
            if pair==b'\0\0':return raw.decode('utf-16-le')
            raw.extend(pair)
        raise ValueError('unterminated source IDS')
    def snapshot(c,label,tid):
        assert c.floating.ControlWord&0xffff==struct.unpack_from('<H',bytes(c.extended),0)[0]
        assert c.floating.StatusWord&0xffff==struct.unpack_from('<H',bytes(c.extended),2)[0]
        row={'label':label,'thread_id':tid,'eip':c.Eip,'esp':c.Esp,'x87_control':c.floating.ControlWord&0xffff,'x87_status':c.floating.StatusWord&0xffff,'mxcsr':struct.unpack_from('<I',bytes(c.extended),24)[0],'crt_sse2_global':word(0xb4b340)}
        row['globals']={'assertion_guard':memory(0xb3b4f8,1)[0], 'csv_guard':word(0xb3b4f4),'ranged_guard':word(0xb3a66c)}
        table=word(0xb39ab0)
        row['settings_table']=table
        row['settings']={f'{off:x}':{'pointer':word(table+off),'bits':word(word(table+off))} for off in [0xa4,0x144,0x148,0x14c,0x150,0x154,0x158,0x15c]}
        if row['globals']['csv_guard']&1:
            cached=word(0xb3b4f0);row['csv_cached_pointer']=cached
            row['csv_length']=word(word(cached+0x10)-0xc)
        if manager is not None:
            row['manager_means']={f'{offset:x}':word(manager+offset) for offset in range(0x74,0x94,4)}
            row['stream_object_bytes']=memory(manager+0x4c,0x30).hex()
        captures.append(row)
    start=time.monotonic()
    try:
        while time.monotonic()-start<args.seconds:
            e=EVENT()
            if not wait(C.byref(e),250):
                if C.get_last_error()==121:continue
                raise C.WinError(C.get_last_error())
            p=bytes(e.payload);status=0x10002;stop=False
            if e.code==3:
                _,hp,ht,base=struct.unpack_from('<QQQQ',p)
                assert base==0x400000,hex(base)
                extra_handles.add(hp)
                extra_handles.add(threads[e.tid])
                threads[e.tid]=ht
                for address,label in [(0x85aa1b,'folder_redirect'),(0x4bd800,'source_entry'),(0x4bbf90,'means_entry'),(0x4bb590,'values_entry')]:breakpoint(address,label)
                if (handle:=struct.unpack_from('<Q',p)[0]):close(handle)
            elif e.code==2:threads[e.tid]=struct.unpack_from('<Q',p)[0]
            elif e.code==4:
                handle=threads.pop(e.tid,None)
                if handle:close(handle)
            elif e.code==6:
                if (handle:=struct.unpack_from('<Q',p)[0]):close(handle)
            elif e.code==1:
                code=struct.unpack_from('<I',p)[0];address=struct.unpack_from('<Q',p,16)[0]
                if code in [0x80000003,0x4000001f] and address in points:
                    c=context(e.tid);original,label=points[address];change(address,original);c.Eip=address
                    if label=='folder_redirect':
                        assert c.Eax < 0x80000000, 'SHGetFolderPathW failed; isolation not established'
                        csidl=word(c.Esp+0x218)&0x7fff
                        dest=folder/('personal' if csidl==5 else 'common' if csidl==35 else 'other')
                        raw=(str(dest)+'\0').encode('utf-16-le');assert len(raw)<=520
                        change(c.Esp+8,raw)
                        events.append({'folder_redirection_csidl':csidl,'destination':str(dest),'tid':e.tid})
                    else:
                        if label=='source_entry':
                            assert any(event.get('folder_redirection_csidl')==5 for event in events), 'Personal folder was not redirected before source startup'
                            manager=c.Ecx;breakpoint(word(c.Esp),'source_return')
                        snapshot(c,label,e.tid)
                        if label=='source_return':
                            rows=[];node=word(word(manager+0x68)+8);seen=set()
                            while node:
                                assert node not in seen;seen.add(node);actor=word(node)
                                rows.append({'ids':wide(word(actor+8)),'flags_174':word(actor+0x174), 'fields':{f'{off:x}':word(actor+off) for off in [0x1e4,0x30c,0x36c]}})
                                node=word(node+8)
                            captures[-1]['actors']=rows;completed=True;stop=True
                    suspended=[]
                    for tid,handle in threads.items():
                        if tid!=e.tid:
                            previous=suspend(handle)
                            check(previous!=0xffffffff);suspended.append(handle)
                    c.EFlags|=0x100;check(put(threads[e.tid],C.byref(c)));stepping[e.tid]=(address,suspended)
                elif code in [0x80000004,0x4000001e] and e.tid in stepping:
                    address,suspended=stepping.pop(e.tid);change(address,b'\xcc');c=context(e.tid);c.EFlags&=~0x100;check(put(threads[e.tid],C.byref(c)))
                    for handle in suspended:check(resume(handle)!=0xffffffff)
                elif code in [0x80000003,0x4000001f]:events.append({'loader_breakpoint':hex(code),'address':hex(address)})
                else:
                    events.append({'exception':hex(code),'address':hex(address),'first_chance':struct.unpack_from('<I',p,152)[0]});status=0x80010001
            elif e.code==5:
                events.append({'exit':struct.unpack_from('<I',p)[0]});stop=True
            check(cont(e.pid,e.tid,status))
            if stop:break
    finally:
        terminate(process.process,0);assert join(process.process,5000)==0, 'Owned debuggee did not exit'
        close(process.process)
        for handle in set(threads.values())|extra_handles:
            if handle!=process.process:close(handle)
        preserved=all(sha(Path(path))==digest for path,digest in original_hashes.items())
        redirected=any('folder_redirection_csidl' in event for event in events)
        report={'schema':1,'source_exe_sha256':sha(executable),'completed_source_return':completed,'scope':'Actual Windows/v14/dgVoodoo startup diagnostic; fresh profile isolation is established ONLY if actual folder redirection events were captured. Not a battle test.', 'runtime_pins':{name:sha(executable.parent/name) for name in ['AA.exe','D3D9.dll','dgVoodoo.conf']}, 'temporary_process_only_breakpoints':True,'folder_redirection_observed':redirected,'events':events,'captures':captures,'original_archives_preserved':preserved,'original_archives_sha256':original_hashes,'script_sha256':sha(Path(__file__))}
        (folder/'capture.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps({'completed':completed,'capture_count':len(captures),'preserved':preserved,'output':str(folder/'capture.json')},indent=2))
    assert completed and preserved, 'Native source completion capture did not succeed'


if __name__=='__main__':main()
