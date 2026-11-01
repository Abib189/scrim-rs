"""live.py: Fix AI stage 2a's live check on scratch apps (scratch HOME and SCRIM_SETTINGS_DIR each).
App A: control 7979, MCP 7690. App B (the second live-session client): control 7980, MCP 7691.
Never port 7680, never the person's settings, files or pointer: input goes through each window's
own control channel, and copy and paste are Scrim's own clipboard (engine commands), not the
system's.

Steps: a normal file made, edited, saved and reopened; a .scrim, a .figcraft and a .sketch opened;
copy, paste and duplicate inside a file, and in a file whose saved id counter is stale; a
document with an embedded PNG and a progressive JPEG saved and reopened (and a crafted JPEG
refused); a local live session hosted by A and joined by B, with edits both ways.
Run from the repository root after `cargo build -p figcraft`:
`python3 docs/design/upstream-sync/live.py`."""
import atexit, base64, hashlib, json, os, shutil, struct, subprocess, sys, tempfile, time, urllib.request, zipfile, zlib
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
HERE = tempfile.mkdtemp(prefix='scrim-fix-ai-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
OUT = os.path.join(HERE, 'out'); os.makedirs(OUT)
SAMPLE = os.path.join(ROOT, 'crates', 'format', 'tests', 'fixtures', 'sketch', 'sample.sketch')
results = []
T0 = time.time()
def step(m): print(f'[{time.time()-T0:6.1f}s] {m}', flush=True)

def check(what, ok, detail=''):
    results.append((what, bool(ok)))
    print(('PASS ' if ok else 'FAIL ') + what + (f' ({detail})' if detail else ''))

def scratch(tag, mcp):
    base = os.path.join(HERE, tag)
    home, settings = os.path.join(base, 'home'), os.path.join(base, 'settings')
    docs = os.path.join(home, 'Documents', 'Scrim')
    for d in (home, settings, docs): os.makedirs(d, exist_ok=True)
    json.dump({'prefs': {'theme': 'light', 'getting_started_dismissed': True,
                         'mcp': {'port': mcp, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
    return home, settings, docs, os.path.join(base, 'app.log')

def launch(tag, control, mcp):
    home, settings, docs, log = scratch(tag, mcp)
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
    p = subprocess.Popen([APP, '--control', str(control)], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(log, 'a'), stderr=subprocess.STDOUT)
    atexit.register(lambda: p.poll() is None and p.terminate())
    a = None
    for _ in range(240):
        try: a = App(control); a.run('mcp.status'); break
        except Exception: time.sleep(0.25)
    if not (a.call('app.settings-dir', {}).get('result', {}) or {}).get('override'): raise SystemExit(f'{tag}: not on scratch settings')
    for _ in range(40):
        if a.run('mcp.status').get('port') == mcp: break
        time.sleep(0.25)
    else: raise SystemExit(f'{tag}: MCP is not on {mcp}: stopping')
    a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(0.8)
    return p, a, docs, log

def try_run(a, cmd, params={}):
    r = a.call('engine.execute', {'command': cmd, 'params': params})
    return r.get('ok'), (r.get('result') if r.get('ok') else (r.get('error') or r))

CAPTURES = {'ok': True}

def shot(a, name):
    """A capture of `a`'s window, asked on a connection of its own: a window macOS doesn't draw
    (hidden on another Space or behind a full-screen app) never answers, and then no capture is
    taken for the rest of the run."""
    if not CAPTURES['ok']:
        return None
    p = os.path.join(OUT, name + '.png')
    try:
        c = App(a.s.getpeername()[1]); c.s.settimeout(15)
        c.call('window.focus', {'id': 0}); c.call('ui.screenshot', {'path': p})
    except Exception:
        CAPTURES['ok'] = False
        print('   (no capture: the window draws no frames, so captures are skipped)')
        return None
    for _ in range(40):
        if os.path.exists(p) and os.path.getsize(p) > 0: return name + '.png'
        time.sleep(0.1)
    return None

def quit(p, a, tag):
    a.call('window.focus', {'id': 0})
    try: a.call('app.quit', {'force': True})
    except Exception: pass
    try: p.wait(timeout=20)
    except subprocess.TimeoutExpired: p.kill(); print(f'  {tag}: killed after 20 s')

def mcp_open(port, path):
    url = f'http://127.0.0.1:{port}/mcp'
    def post(body, sid=None, raw=False):
        h = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
        if sid: h['Mcp-Session-Id'] = sid
        r = urllib.request.urlopen(urllib.request.Request(url, data=json.dumps(body).encode(), headers=h, method='POST'), timeout=60)
        return r if raw else r.read()
    r = post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-ai-live', 'version': '1'}}}, raw=True)
    sid = r.headers['Mcp-Session-Id']; post({'jsonrpc': '2.0', 'method': 'notifications/initialized'}, sid)
    d = json.loads(post({'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'open_file', 'arguments': {'path': path}}}, sid))
    return d['result'].get('isError'), d['result']['content'][0].get('text', '')

def layers(a):
    d = a.call('document.inspect', {'depth': 8}).get('result', {})
    out = []
    def walk(n):
        for c in n.get('children', []) or []:
            out.append(c); walk(c)
    for page in d.get('pages', []) or []: walk(page)
    if not out: walk(d)
    return out

def png(w, h, rgb):
    raw = b''.join(b'\x00' + bytes(rgb) * w for _ in range(h))
    chunk = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')

def progressive_jpeg(cb):
    # The same 8x8 progressive JPEG the format tests build (one DC scan, all blocks zero: mid grey).
    b = bytearray([0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00]) + bytes([1] * 64)
    b += bytes([0xFF, 0xC2, 0x00, 0x11, 0x08, 0x00, 0x08, 0x00, 0x08, 0x03, 0x01, 0x11, 0x00, 0x02, cb, 0x00, 0x03, 0x11, 0x00])
    b += bytes([0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01]) + bytes(16)
    b += bytes([0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x03, 0xFF, 0xD9])
    return bytes(b)

def center_pixel(png_bytes):
    # A minimal PNG reader (8-bit RGB or RGBA, not interlaced): the pixel at the picture's centre.
    pos, idat, w = 8, b'', 0
    while pos < len(png_bytes):
        n = struct.unpack('>I', png_bytes[pos:pos + 4])[0]; kind = png_bytes[pos + 4:pos + 8]; data = png_bytes[pos + 8:pos + 8 + n]
        if kind == b'IHDR': w, h, depth, color = struct.unpack('>IIBB', data[:10])
        elif kind == b'IDAT': idat += data
        pos += 12 + n
    bpp = 4 if color == 6 else 3
    raw, stride, rows, prev = zlib.decompress(idat), w * bpp, [], bytearray(w * bpp)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a, b, c = (line[i - bpp] if i >= bpp else 0), prev[i], (prev[i - bpp] if i >= bpp else 0)
            pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
            line[i] = (line[i] + [0, a, b, (a + b) // 2, a if pa <= pb and pa <= pc else (b if pb <= pc else c)][f]) & 255
        rows.append(line); prev = line
    row = rows[h // 2]; i = (w // 2) * bpp
    return tuple(row[i:i + 3])

def digest(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()[:12]

pa, a, docs, log_a = launch('a', 7979, 7690)

step('step 1')
# 1. A normal file: made, edited, saved and reopened.
a.run('file.new'); time.sleep(0.3)
f = a.run('frame.create', {'x': 0, 'y': 0, 'width': 400, 'height': 300, 'name': 'Card'})
fid = f.get('id') or (f.get('ids') or [None])[0]
r = a.run('shape.rectangle', {'x': 20, 'y': 20, 'width': 120, 'height': 80, 'parent': fid})
rid = r.get('id') or (r.get('ids') or [None])[0]
a.run('node.set', {'id': rid, 'fills': ['#36BBFF'], 'name': 'Badge'})
poster = os.path.join(docs, 'Card.scrim')
a.run('file.saveAs', {'path': poster})
a.run('node.set', {'id': rid, 'x': 60})
a.run('file.save', {})
a.run('file.close', {'force': True}); time.sleep(0.3)
err, text = mcp_open(7690, poster)
names = sorted(n.get('name') for n in layers(a))
check('a normal file is edited, saved and reopens with its layers', not err and names == ['Badge', 'Card'], names)
print('   shot', shot(a, '1-normal-file'))

step('step 2')
# 2. A .scrim, a .figcraft and a .sketch open.
fig = os.path.join(docs, 'Card.figcraft'); shutil.copyfile(poster, fig)
err, text = mcp_open(7690, fig)
check('a .figcraft opens', not err and sorted(n.get('name') for n in layers(a)) == ['Badge', 'Card'])
sketch = os.path.join(docs, 'sample.sketch'); shutil.copyfile(SAMPLE, sketch)
ok, res = try_run(a, 'file.open', {'path': sketch}); time.sleep(0.5)
check('a .sketch opens', ok and len(layers(a)) > 0, f'{len(layers(a))} layers')
print('   shot', shot(a, '2-sketch'))

step('step 3')
# 3. Copy, paste and duplicate inside a file, and in one whose saved id counter is stale.
step('3: reopen card'); err, text = mcp_open(7690, poster)
step('3: reopened'); before = {n['id']: n.get('name') for n in layers(a)}
badge = next(i for i, n in before.items() if n == 'Badge')
step('3: copy'); a.run('select.set', {'ids': [badge]}); a.run('edit.copy', {}); step('3: paste'); a.run('edit.paste', {}); step('3: pasted')
a.run('select.set', {'ids': [badge]}); a.run('edit.duplicate', {})
after = {n['id']: n.get('name') for n in layers(a)}
check('copy, paste and duplicate add two layers and keep the others', len(after) == len(before) + 2 and all(after.get(i) == n for i, n in before.items()), f'{len(before)} -> {len(after)}')
stale = os.path.join(docs, 'Stale.scrim')
with zipfile.ZipFile(poster) as z, zipfile.ZipFile(stale, 'w', zipfile.ZIP_DEFLATED) as w:
    for item in z.infolist():
        data = z.read(item.filename)
        if item.filename == 'document.json':
            j = json.loads(data); j['next_id'] = 1; data = json.dumps(j).encode()
        w.writestr(item, data)
err, text = mcp_open(7690, stale)
before = {n['id']: n.get('name') for n in layers(a)}
card = next(i for i, n in before.items() if n == 'Card')
a.run('select.set', {'ids': [card]}); a.run('edit.duplicate', {})
after = {n['id']: n.get('name') for n in layers(a)}
check('a file with a stale id counter opens, and a duplicate keeps every layer', not err and len(after) == len(before) + 2 and all(after.get(i) == n for i, n in before.items()), f'{len(before)} -> {len(after)}')

step('step 4')
# 4. An embedded PNG and a progressive JPEG: placed, saved, reopened, drawn; a crafted JPEG refused.
a.run('file.new'); time.sleep(0.3)
a.run('image.place', {'data': base64.b64encode(png(64, 64, (220, 40, 40))).decode(), 'x': 0, 'y': 0})
a.run('image.place', {'data': base64.b64encode(progressive_jpeg(0x11)).decode(), 'x': 100, 'y': 0, 'width': 64, 'height': 64})
ok, res = try_run(a, 'image.place', {'data': base64.b64encode(progressive_jpeg(0x22)).decode(), 'x': 200, 'y': 0})
check('a crafted progressive JPEG is refused as an image, not a crash', not ok and 'JPEG' in json.dumps(res), json.dumps(res)[:140])
pics = os.path.join(docs, 'Pictures.scrim')
a.run('file.saveAs', {'path': pics}); a.run('file.close', {'force': True}); time.sleep(0.3)
err, text = mcp_open(7690, pics)
inspect = a.call('document.inspect', {'depth': 0}).get('result', {})
check('a document with an embedded PNG and a progressive JPEG reopens', not err and len(layers(a)) == 2, f'{len(layers(a))} layers')
drawn = []
for n in sorted(layers(a), key=lambda n: n.get('x', 0)):
    out = a.run('file.export', {'ids': [n['id']], 'format': 'png'})
    drawn.append(center_pixel(base64.b64decode(out.get('data') or out.get('files', [{}])[0].get('data', ''))))
png_ok = len(drawn) == 2 and drawn[0][0] > 180 and drawn[0][1] < 80
grey = len(drawn) == 2 and all(100 <= v <= 160 for v in drawn[1]) and max(drawn[1]) - min(drawn[1]) < 12
check('the reopened PNG draws red and the progressive JPEG draws its grey (exported, no window needed)', png_ok and grey, drawn)
print('   shot', shot(a, '4-images'))

step('step 5')
# 5. A local live session: A hosts its file, B joins it from a second app; edits both ways.
err, text = mcp_open(7690, poster)
host = a.run('collab.host', {'port': 47977})
link = host.get('link')
check('A hosts a local live session', bool(link), (link or '')[:60])
pb, b, _, log_b = launch('b', 7980, 7691)
b.run('collab.join', {'link': link})
seen = []
for _ in range(80):
    seen = sorted(n.get('name') for n in layers(b))
    if 'Badge' in seen: break
    time.sleep(0.25)
check('B joins and gets the file', 'Badge' in seen, seen[:6])
badge = next(n['id'] for n in layers(a) if n.get('name') == 'Badge')
a.run('node.set', {'id': badge, 'name': 'Badge from A'})
for _ in range(80):
    if any(n.get('name') == 'Badge from A' for n in layers(b)): break
    time.sleep(0.25)
check("A's edit reaches B", any(n.get('name') == 'Badge from A' for n in layers(b)))
card_b = next(n['id'] for n in layers(b) if n.get('name') == 'Card')
b.run('node.set', {'id': card_b, 'name': 'Card from B'})
for _ in range(80):
    if any(n.get('name') == 'Card from B' for n in layers(a)): break
    time.sleep(0.25)
check("B's edit reaches A", any(n.get('name') == 'Card from B' for n in layers(a)))
peers = a.run('collab.peers', {})
print('   peers on A:', json.dumps(peers)[:200])
print('   shot', shot(a, '5-live-host'), shot(b, '5-live-guest'))
try_run(b, 'collab.leave', {}); try_run(a, 'collab.stop', {})
quit(pb, b, 'b'); quit(pa, a, 'a')

panics = [l.strip() for lg in (log_a, log_b) for l in open(lg, errors='replace') if 'panick' in l.lower()]
check('no panics in either log', not panics, '; '.join(panics[:3]))
print(f'\n{sum(ok for _, ok in results)} of {len(results)} checks passed; captures in {OUT}')
