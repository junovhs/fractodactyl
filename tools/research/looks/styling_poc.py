#!/usr/bin/env python3
"""Bounded local real-sample styling demo. No packages, services or downloaded assets."""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import time
import zlib

root = Path(__file__).resolve().parents[3]
out = Path(os.environ.get('FD_STYLE_OUTPUT', str(Path.home() / '.fdv/fractodactyl/styling-poc')))
fd = os.environ['FD_STYLE_BIN']
out.mkdir(parents=True, exist_ok=True)
commands = []
previous = json.loads((out/'commands.json').read_text()) if (out/'commands.json').exists() else []

def run(args, label, timeout=60):
    start = time.perf_counter()
    result = subprocess.run([str(x) for x in args], cwd=root, capture_output=True, text=True, timeout=timeout)
    seconds = time.perf_counter() - start
    (out / (label + '.stdout')).write_text(result.stdout)
    (out / (label + '.stderr')).write_text(result.stderr)
    commands.append({'label': label, 'argv': [str(x) for x in args], 'exit': result.returncode, 'seconds': seconds})
    (out / 'commands.json').write_text(json.dumps(commands, indent=2))
    if result.returncode:
        raise RuntimeError(f'{label}: {result.stderr[-6000:]}')
    return result.stdout

def png_data(path):
    data = path.read_bytes()
    i, width, height, packed = 8, 0, 0, bytearray()
    while i < len(data):
        n = int.from_bytes(data[i:i+4], 'big')
        kind, chunk = data[i+4:i+8], data[i+8:i+8+n]
        if kind == b'IHDR':
            width, height = struct.unpack('>II', chunk[:8])
            assert chunk[8:10] == b'\x08\x02', 'RGB8 expected'
        if kind == b'IDAT':
            packed.extend(chunk)
        i += n+12
    raw = zlib.decompress(packed)
    stride = width*3+1
    assert all(raw[y*stride] == 0 for y in range(height)), 'fd PNG filter 0 expected'
    rgb = b''.join(raw[y*stride+1:(y+1)*stride] for y in range(height))
    return width, height, rgb

def chunk(kind, body):
    return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind+body))

def png_save(path, width, height, rgb):
    rows = b''.join(b'\0'+rgb[y*width*3:(y+1)*width*3] for y in range(height))
    header = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', header)+chunk(b'IDAT', zlib.compress(rows, 9))+chunk(b'IEND', b''))

def compress_copy(source, destination):
    w,h,rgb = png_data(source)
    png_save(destination,w,h,rgb)
    return {'path': str(destination), 'size': [w,h], 'min': min(rgb), 'max': max(rgb), 'mean_channel': sum(rgb)/len(rgb)}

celestial = root/'looks/styles/modular-light.style'
swapped = root/'looks/styles/modular-light-swapped.style'
copper = root/'looks/styles/etched-copper.style'
# Both images are genuine renderer outputs. High-iteration math happens only here.
views = [
    ('valley','-0.7453','0.1127','0.0065','1024x640','2'),
    ('whole','-0.55','0','3.1','1024x768','2'),
]
inputs = {}
for name,re,im,width,size,ss in views:
    target = out/(name+'.fds')
    argv=[fd,'render','--re',re,'--im',im,'--width',width,'--size',size,'--ss',ss,'--iter','5000','--threads','2','--columns','nu,de,normal','-o',target]
    if target.exists() and any(c.get('argv')==[str(x) for x in argv] and c.get('exit')==0 for c in previous):
        commands.append({'label':'math-'+name,'argv':[str(x) for x in argv],'exit':0,'reused_input':True,'sha256':hashlib.sha256(target.read_bytes()).hexdigest()})
    else:
        run(argv, 'math-'+name)
    inputs[name] = target
hashes = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name,path in inputs.items()}

common = ['--unresolved','interior']
finish = ['--dither','on']
images = []
for name,source in inputs.items():
    for style,graph in [('celestial',celestial),('copper',copper)]:
        target = out/f'{name}-{style}.png'
        run([fd,'style','render',graph,source,'-o',target]+common+finish,'still-'+name+'-'+style)
        images.append(compress_copy(target,out/f'{name}-{style}-share.png'))

# A same-raster ablation: real time advances every frame, so static replay is not
# mistaken for the cost of animated shading. Each probe stays below 30 seconds.
rows = []
for name,target in [('base','albedo_image'),('warp','warped'),('relight','lit'),('halo','halo'),('all','final')]:
    graph=out/('probe-'+name+'.style')
    graph.write_text(celestial.read_text().replace('output final','output '+target))
    text = run([fd,'style','sequence',graph,inputs['valley'],'-o',out/('ablation-'+name),'--frames','8','--fps','24']+common+finish,'ablation-'+name,timeout=30)
    frame_rows = []
    adapter = ''
    summary = ''
    for line in text.splitlines():
        if line.startswith('style_frame '):
            tokens=line.split()
            keys=['seconds','dispatches','pipelines','allocations','sample_uploads','working_bytes_estimate','bloom_support_px','graph_support_px','evaluations_per_sample_max']
            frame_rows.append({key:float(tokens[tokens.index(key)+1]) for key in keys})
        elif line.startswith('style_output '):
            adapter=line.split(' adapter ',1)[1].split(' load_seconds ',1)[0]
        elif line.startswith('style_summary '):
            summary=line
    assert len(frame_rows)==8
    assert all(r['sample_uploads']==1 for r in frame_rows)
    assert len({(r['pipelines'],r['allocations'],r['working_bytes_estimate']) for r in frame_rows})==1
    warm=sorted(r['seconds'] for r in frame_rows[1:])
    rows.append({'variant':name,'adapter':adapter,'first_seconds':frame_rows[0]['seconds'],'warm_p50_seconds':warm[len(warm)//2],'warm_max_seconds':max(warm),'resources':frame_rows[-1],'summary':summary})

# Same primitives and parameters, different dependency order.
run([fd,'style','inspect',celestial],'connected-plan')
run([fd,'style','render',swapped,inputs['valley'],'-o',out/'valley-swapped.png']+common+finish,'swapped-order')
_,_,normal=png_data(out/'valley-celestial.png')
_,_,other=png_data(out/'valley-swapped.png')
order_difference={'mean_abs_channel_difference':sum(abs(a-b) for a,b in zip(normal,other))/len(normal),'changed_channels':sum(a!=b for a,b in zip(normal,other))}
assert order_difference['changed_channels']>0,'operation order had no effect'
images.append(compress_copy(out/'valley-swapped.png',out/'valley-swapped-share.png'))

# Animation uses the same stored raster: no camera zoom or rerendered math.
frames = out/'motion'
run([fd,'style','sequence',celestial,inputs['valley'],'-o',frames,'--frames','48','--fps','24']+common+finish,'motion')
replay = out/'replay-24.png'
run([fd,'style','render',celestial,inputs['valley'],'-o',replay,'--time','1','--exposure','24']+common+finish,'replay')
assert replay.read_bytes() == (frames/'frame-00024.png').read_bytes(), 'frame replay differs'
clip = out/'celestial-motion.mp4'
run(['ffmpeg','-y','-v','error','-threads','2','-filter_threads','1','-framerate','24','-i',frames/'frame-%05d.png','-frames:v','48','-c:v','libx264','-threads','2','-crf','16','-pix_fmt','yuv444p',clip],'encode')
decoded = out/'decoded.rgb'
run(['ffmpeg','-y','-v','error','-threads','2','-i',clip,'-vf',r'select=eq(n\,0)+eq(n\,24)+eq(n\,47)','-vsync','0','-pix_fmt','rgb24','-f','rawvideo',decoded],'decode')
raw=decoded.read_bytes()
w,h,_=png_data(frames/'frame-00000.png')
count=w*h*3
assert len(raw)==count*3
motion=[]
for k,frame in enumerate([0,24,47]):
    rgb=raw[k*count:(k+1)*count]
    _,_,original=png_data(frames/f'frame-{frame:05}.png')
    mae=sum(abs(a-b) for a,b in zip(rgb,original))/len(rgb)
    target=out/f'decoded-{frame:02}.png'
    png_save(target,w,h,rgb)
    motion.append({'frame':frame,'decoded_png':str(target),'mean_abs_channel_error':mae})
for name,path in inputs.items():
    assert hashes[name]==hashlib.sha256(path.read_bytes()).hexdigest(), 'styling changed input samples'
report={'images':images,'ablations':rows,'input_sha256':hashes,'order_difference':order_difference,'motion':{'path':str(clip),'frames':48,'fps':24,'replay_equal':True,'decoded':motion},'notes':['Warm wall times include readback; PNG encoding/writing and input math are separate.','Buffer bytes are explicit renderer estimates, not total driver VRAM.','Connected graph: rasterized fields, bounded image warp/blur/height relighting and fused composition; no global flow or fluid simulation.','Aesthetic reference matching remains a visual judgment; no exact-match claim.']}
(out/'report.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
