"""live-2c.py: Fix AI stage 2c's live check on scratch apps: a scratch HOME and SCRIM_SETTINGS_DIR
each, controls 7979 and 7980, MCP 7690 and 7691. Never port 7680, never the person's settings,
files or pointer: input goes through each window's own control channel, files are written by this
script in its scratch folder, and hiding the window goes through NSRunningApplication (no
Accessibility, no keystrokes).

Steps:
- C-1: a crafted FigCraft board file opened at launch (as Finder opens it) shows the notice and is
  byte for byte the same; Save (Cmd+S) shows the three choices and leaves it unchanged; Save a copy
  writes a .scrim without the board marks. A design file opened at launch shows no notice.
- C-3: B joins A's live session; A is idle, B moves its pointer for 70 s: B stays connected (never
  reconnecting) and an edit still reaches A.
- C-5: A's window hidden (draws no frames), an edit, then 13 s (past the 10 s interval): a recovery
  copy with the edit is in the settings folder.
- C-4: MCP inspect_document on a 20,000-layer document: under 8 MiB, with the truncated marker.
- C-2: Quit (File > Quit Scrim, `ui.menu.invoke app.quit`) with unsaved changes asks, and the app
  stays; with nothing unsaved it quits without asking.
Run from the repository root after `cargo build -p figcraft`:
`python3 docs/design/upstream-sync/live-2c.py`."""
import atexit, hashlib, json, os, subprocess, sys, tempfile, time, urllib.request, zipfile
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
HERE = tempfile.mkdtemp(prefix='scrim-fix-ai-2c-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
results = []
T0 = time.time()
def step(m): print(f'[{time.time()-T0:6.1f}s] {m}', flush=True)

def check(what, ok, detail=''):
    results.append((what, bool(ok)))
    print(('PASS ' if ok else 'FAIL ') + what + (f' ({detail})' if detail else ''), flush=True)

def scratch(tag, mcp):
    base = os.path.join(HERE, tag)
    home, settings = os.path.join(base, 'home'), os.path.join(base, 'settings')
    docs = os.path.join(home, 'Documents', 'Scrim')
    for d in (home, settings, docs): os.makedirs(d, exist_ok=True)
    if not os.path.exists(os.path.join(settings, 'state.json')):
        json.dump({'prefs': {'theme': 'light', 'getting_started_dismissed': True,
                             'mcp': {'port': mcp, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
    return home, settings, docs, os.path.join(base, 'app.log')

PROCS = []
atexit.register(lambda: [p.poll() is None and p.terminate() for p in PROCS])

def launch(tag, control, mcp, files=()):
    home, settings, docs, log = scratch(tag, mcp)
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
    p = subprocess.Popen([APP, '--control', str(control), *files], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(log, 'a'), stderr=subprocess.STDOUT)
    PROCS.append(p)
    a = None
    for _ in range(240):
        try: a = App(control); a.run('mcp.status'); break
        except Exception: a = None; time.sleep(0.25)
    if a is None: raise SystemExit(f'{tag}: the app did not answer on its control port')
    if not (a.call('app.settings-dir', {}).get('result', {}) or {}).get('override'): raise SystemExit(f'{tag}: not on scratch settings: stopping')
    for _ in range(40):
        if a.run('mcp.status').get('port') == mcp: break
        time.sleep(0.25)
    else: raise SystemExit(f'{tag}: MCP is not on {mcp}: stopping')
    a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(0.8)
    return p, a, docs, settings, log

def run(a, cmd, params={}):
    r = a.call('engine.execute', {'command': cmd, 'params': params})
    return r.get('ok'), (r.get('result') if r.get('ok') else (r.get('error') or r))

def ui(a): return a.call('ui.inspect', {}).get('result') or {}

def sha(path): return hashlib.sha256(open(path, 'rb').read()).hexdigest()

def document(path):
    with zipfile.ZipFile(path) as z: return json.loads(z.read('document.json'))

def marks(path):
    d = document(path)
    return d.get('kind') == 'board' or any('board' in n for n in d['nodes'].values())

def rewrite(path, edit):
    with zipfile.ZipFile(path) as z: members = [(i.filename, z.read(i.filename)) for i in z.infolist()]
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in members:
            if name == 'document.json':
                d = json.loads(data); edit(d); data = json.dumps(d).encode()
            z.writestr(name, data)

def gone(p, secs):
    for _ in range(int(secs * 4)):
        if p.poll() is not None: return True
        time.sleep(0.25)
    return False

CAPTURE = {'ok': True}
def shot(a, port, name):
    if not CAPTURE['ok']: return None
    path = os.path.join(HERE, name + '.png')
    try:
        c = App(port); c.s.settimeout(15); c.call('ui.screenshot', {'path': path})
        for _ in range(40):
            if os.path.exists(path) and os.path.getsize(path) > 0: return path
            time.sleep(0.1)
    except Exception: pass
    CAPTURE['ok'] = False
    print('   (no capture: the window draws no frames, so captures are skipped)', flush=True)
    return None

def mcp(port):
    url = f'http://127.0.0.1:{port}/mcp'
    def post(body, sid=None, raw=False):
        h = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
        if sid: h['Mcp-Session-Id'] = sid
        r = urllib.request.urlopen(urllib.request.Request(url, data=json.dumps(body).encode(), headers=h, method='POST'), timeout=120)
        return r if raw else r.read()
    r = post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-ai-2c', 'version': '1'}}}, raw=True)
    sid = r.headers['Mcp-Session-Id']; post({'jsonrpc': '2.0', 'method': 'notifications/initialized'}, sid)
    n = [10]
    def call(tool, args):
        n[0] += 1
        raw = post({'jsonrpc': '2.0', 'id': n[0], 'method': 'tools/call', 'params': {'name': tool, 'arguments': args}}, sid)
        d = json.loads(raw)
        return d['result'].get('isError'), d['result']['content'][0].get('text', ''), len(raw)
    return call

def hide(pid, hidden=True):
    js = f'ObjC.import("AppKit"); var a = $.NSRunningApplication.runningApplicationWithProcessIdentifier({pid}); a.{"hide" if hidden else "unhide"}; "ok"'
    return subprocess.run(['osascript', '-l', 'JavaScript', '-e', js], capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL).stdout.strip()

# Files: made by a scratch app, then the board marks written into one the way upstream writes them.
step('making the files')
pm, m, DOCS_M, _, _ = launch('maker', 7979, 7690)
board, design, big = (os.path.join(DOCS_M, n) for n in ('Plan.scrim', 'Design.scrim', 'Big.scrim'))
run(m, 'file.new', {}); ok, r = run(m, 'node.create', {'type': 'frame', 'x': 0, 'y': 0, 'width': 200, 'height': 200, 'name': 'Sticky'})
sticky = r.get('id') if ok else None
run(m, 'file.save', {'path': board}); run(m, 'file.close', {'force': True})
run(m, 'file.new', {}); run(m, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 40, 'height': 40, 'name': 'Card'})
run(m, 'file.save', {'path': design}); run(m, 'file.close', {'force': True})
run(m, 'file.new', {}); run(m, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 10, 'height': 10, 'name': 'Cell'})
run(m, 'file.save', {'path': big}); run(m, 'file.close', {'force': True})
m.call('app.quit', {}); gone(pm, 20)
def crafted(d):
    d['kind'] = 'board'
    for n in d['nodes'].values():
        if n.get('name') == 'Sticky': n['board'] = {'type': 'sticky', 'author': 'Ana'}
rewrite(board, crafted)
def many(d):
    cell = next(n for n in d['nodes'].values() if n.get('name') == 'Cell')
    page = d['nodes'][str(cell['parent'])]
    top = max(int(k) for k in d['nodes'])
    for i in range(20000):
        k = top + 1 + i
        n = json.loads(json.dumps(cell)); n['id'] = k; n['name'] = f'Cell {i}'
        n['transform'] = [1.0, 0.0, 0.0, 1.0, float(i % 140) * 12, float(i // 140) * 12]
        d['nodes'][str(k)] = n; page['children'].append(k)
    d['next_id'] = max(d.get('next_id', 0), top + 20001)
rewrite(big, many)
before = sha(board)

# C-1: the board file opened as Finder opens it.
step('C-1: a board file opened at launch')
pa, a, DOCS_A, SET_A, LOG_A = launch('a', 7979, 7690, [board])
time.sleep(1.0)
seen = ui(a).get('boardFile')
check('C-1 a board file shows the notice when it opens', seen == {'kind': 'opened', 'name': 'Plan'}, f'{seen}')
check('C-1 opening it leaves its bytes unchanged', sha(board) == before)
print('   shot', shot(a, 7979, 'c1-notice'))
run(a, 'shape.rectangle', {'x': 300, 'y': 0, 'width': 10, 'height': 10, 'name': 'Mine'})
a.call('ui.key', {'key': 's', 'cmd': True}); time.sleep(0.6)
seen = ui(a).get('boardFile')
check('C-1 Save (Cmd+S) shows the three choices and writes nothing', seen == {'kind': 'saving', 'name': 'Plan'} and sha(board) == before, f'{seen}')
copy = os.path.join(DOCS_A, 'Plan copy.scrim')
ok, r = run(a, 'file.saveAs', {'path': copy})
check('C-1 Save a copy writes a .scrim design and leaves the board file', ok and os.path.exists(copy) and not marks(copy) and sha(board) == before and marks(board), f'{r if not ok else ""}')

step('C-1: a design file opened at launch')
pb, b, DOCS_B, SET_B, LOG_B = launch('b', 7980, 7691, [design])
time.sleep(1.0)
check('C-1 a design file shows no notice', ui(b).get('boardFile') is None, f"{ui(b).get('boardFile')}")

# C-3: a live session where only B moves its pointer.
step('C-3: B alone moves its pointer for 70 s')
host = a.run('collab.host', {'port': 47977})
link = host.get('link')
b.run('collab.join', {'link': link})
joined = False
for _ in range(80):
    if (ui(b).get('live') or {}).get('status') == 'live': joined = True; break
    time.sleep(0.25)
check('C-3 B joins A\'s live session', joined, f"{(ui(b).get('live') or {}).get('status')}")
statuses = set()
t = time.time(); k = 0
while time.time() - t < 70:
    k += 1
    b.run('collab.cursor', {'x': 100 + (k % 50) * 3, 'y': 80 + (k % 30) * 2})
    statuses.add((ui(b).get('live') or {}).get('status'))
    time.sleep(0.2)
status = (ui(b).get('live') or {}).get('status')
a = App(7979)  # A's control connection sat idle for the minute: a fresh one
check('C-3 after 70 s in which only B moved its pointer, B is still connected and never reconnected',
      status == 'live' and statuses == {'live'} and (ui(a).get('live') or {}).get('peers') == 1, f'statuses seen {sorted(map(str, statuses))}, {k} moves')
ok, r = run(b, 'shape.rectangle', {'x': 500, 'y': 0, 'width': 10, 'height': 10, 'name': 'From B after a minute'})
# A host whose window draws no frames (B's window covers it) applies what arrives once it draws
# again (found here, older than this stage, not fixed in it): bring it to the front first.
covered = not a.call('ui.input', {'events': [{'move': [5, 5]}]}).get('ok')
a.call('window.focus', {'id': 0}); time.sleep(1.5)
arrived = False
for _ in range(80):
    d = a.call('document.inspect', {'depth': 4}).get('result', {})
    if 'From B after a minute' in json.dumps(d): arrived = True; break
    time.sleep(0.25)
check('C-3 and B\'s edit then reaches A', ok and arrived, f'A was covered: {covered}')
try: run(b, 'collab.leave', {}); run(a, 'collab.stop', {})
except Exception: pass

# C-5: recovery copies with the window hidden.
step('C-5: an edit while the window is hidden')
run(a, 'file.close', {'force': True})
run(a, 'file.new', {}); run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 10, 'height': 10, 'name': 'Hidden edit'})
print('   hide:', hide(pa.pid)); time.sleep(1.5)
probe = a.call('ui.input', {'events': [{'move': [10, 10]}]})
hidden = not probe.get('ok') and 'hidden' in json.dumps(probe)
run(a, 'shape.rectangle', {'x': 20, 'y': 0, 'width': 10, 'height': 10, 'name': 'Copied while hidden'})
time.sleep(13)
rec = os.path.join(SET_A, 'Recovery', str(pa.pid))
copies = [os.path.join(rec, f) for f in (os.listdir(rec) if os.path.isdir(rec) else []) if f.endswith('.scrim')]
has = any('Copied while hidden' in json.dumps(document(c)) for c in copies)
check('C-5 with the window hidden (drawing no frames), an edit is copied after the interval', hidden and has, f'hidden={hidden} copies={len(copies)}')
print('   unhide:', hide(pa.pid, False)); time.sleep(1.0)

# C-4: a capped MCP reply.
step('C-4: inspect_document on 20,000 layers')
call = mcp(7690)
big_a = os.path.join(DOCS_A, 'Big.scrim')
open(big_a, 'wb').write(open(big, 'rb').read())  # inside A's MCP file root (Fix W)
e, text, _ = call('open_file', {'path': big_a})
e2, text2, size = call('inspect_document', {'depth': 4})
t = json.loads(text2).get('truncated') if not e2 else None
check('C-4 inspect_document on a 20,000-layer document is under 8 MiB with the truncated marker',
      not e and not e2 and size <= 8 << 20 and t and t.get('requestedDepthBytes', 0) > (8 << 20),
      f'{size} bytes on the wire; marker {json.dumps(t)[:220] if t else (text if e else text2[:200])}')

# C-2: Quit.
step('C-2: Quit with unsaved changes, then with none')
run(b, 'shape.rectangle', {'x': 60, 'y': 0, 'width': 10, 'height': 10, 'name': 'Unsaved'})
b.call('ui.menu.invoke', {'id': 'app.quit'})
stayed = not gone(pb, 4)
print('   shot', shot(b, 7980, 'c2-quit-question'))
check('C-2 File > Quit Scrim with unsaved changes asks: the app stays', stayed)
pb.terminate(); gone(pb, 10)
for _ in range(6):
    if not run(a, 'file.close', {'force': True})[0]: break
a.call('ui.menu.invoke', {'id': 'app.quit'})
check('C-2 File > Quit Scrim with nothing unsaved quits without asking', gone(pa, 20))

panics = [l.strip() for lg in (LOG_A, LOG_B) for l in open(lg, errors='replace') if 'panick' in l.lower()]
check('no panic in either app log', not panics, '; '.join(panics)[:200])
print(f'\n{sum(ok for _, ok in results)}/{len(results)} passed in {time.time()-T0:.0f} s; scratch: {HERE}')
sys.exit(0 if all(ok for _, ok in results) else 1)
