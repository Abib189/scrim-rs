"""live.py: Fix AG's live check on a scratch app (scratch HOME and settings, control 7979, MCP 7690).
A poster is saved as .scrim and reopened through MCP; a .figcraft copy of it is opened and saved
(Save asks for a .scrim beside it: the suggestion is read from the app's log, the native panel is
closed by quitting); the committed Sketch sample is imported with its report; Home is captured
after. Input goes through the window's own control channel; the system pointer is never moved.
Run from the repository root after `cargo build -p figcraft`: `python3 docs/design/scrim-format/live.py`."""
import hashlib, json, os, shutil, subprocess, sys, tempfile, time, atexit, urllib.request, zipfile
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
HERE = tempfile.mkdtemp(prefix='scrim-fix-ag-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
scratch = os.path.join(HERE, 'scratch')
home, settings, out = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings'), os.path.join(HERE, 'out')
docs = os.path.join(home, 'Documents', 'Scrim')
for d in (home, settings, docs, out): os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'theme': 'light', 'getting_started_dismissed': True, 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
log_path = os.path.join(scratch, 'app.log')

def launch(*files):
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings, RUST_LOG='warn,figcraft_ui_egui::dialogs=info')
    p = subprocess.Popen([APP, '--control', '7979', *files], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(log_path, 'a'), stderr=subprocess.STDOUT)
    atexit.register(lambda: p.poll() is None and p.terminate())
    a = None
    for _ in range(240):
        try: a = App(7979); a.run('mcp.status'); break
        except Exception: time.sleep(0.25)
    if not (a.call('app.settings-dir', {}).get('result', {}) or {}).get('override'): raise SystemExit('not on scratch settings')
    for _ in range(40):
        if a.run('mcp.status').get('port') == 7690: break
        time.sleep(0.25)
    else: raise SystemExit('MCP is not on 7690: stopping')
    return p, a

class Mcp:
    URL = 'http://127.0.0.1:7690/mcp'
    def __init__(self):
        self.sid, self.n = None, 1
        r = self._post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-ag-live', 'version': '1'}}}, raw=True)
        self.sid = r.headers['Mcp-Session-Id']
        self._post({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
    def _post(self, body, raw=False):
        h = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
        if self.sid: h['Mcp-Session-Id'] = self.sid
        r = urllib.request.urlopen(urllib.request.Request(self.URL, data=json.dumps(body).encode(), headers=h, method='POST'), timeout=60)
        return r if raw else r.read()
    def tool(self, name, args):
        self.n += 1
        d = json.loads(self._post({'jsonrpc': '2.0', 'id': self.n, 'method': 'tools/call', 'params': {'name': name, 'arguments': args}}))
        res = d['result']; return res.get('isError'), res['content'][0].get('text', '')

def inp(a, *evs):
    a.call('ui.input', {'events': list(evs)}); time.sleep(0.12)
def idle(a, n=6):
    for _ in range(n): inp(a, {'move': [700, 790]}); time.sleep(0.05)
def shot(a, name):
    p = os.path.join(out, name + '.png')
    a.call('ui.screenshot', {'path': p})
    for _ in range(60):
        idle(a, 1)
        if os.path.exists(p) and os.path.getsize(p) > 0: time.sleep(0.3); return name + '.png'
    return None
def live_frames(a):
    for k in range(5):
        a.call('window.focus', {'id': 0}); time.sleep(0.5)
        p = os.path.join(out, '_probe.png')
        if os.path.exists(p): os.remove(p)
        a.call('ui.screenshot', {'path': p})
        for _ in range(20):
            idle(a, 1)
            if os.path.exists(p) and os.path.getsize(p) > 0: os.remove(p); return k + 1
    raise SystemExit('the window draws no frames')
def digest(path): return hashlib.sha256(open(path, 'rb').read()).hexdigest()[:12]
def state(a, tag):
    i = a.call('ui.inspect').get('result', {})
    d = a.call('document.inspect', {'depth': 0}).get('result', {})
    rep = i.get('importReport') or {}
    print(f'  {tag}: tabs {[t["name"] for t in i.get("tabs", [])]} | path {d.get("path")!r} | dirty {d.get("dirty")} | import report {rep.get("counts")} {len(rep.get("warnings") or [])} warnings')
    return i
def quit(p, a):
    a.call('window.focus', {'id': 0})
    try: a.call('app.quit', {})
    except ValueError: pass
    try: p.wait(timeout=20); print('  quit', p.returncode)
    except subprocess.TimeoutExpired: p.kill(); print('  killed after 20 s')

p, a = launch()
a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(1.0)
print(f'frames after {live_frames(a)} focus')

# 1. A new document saved as .scrim (what the Save as dialog runs once a name is chosen), reopened through MCP.
a.run('file.new'); time.sleep(0.4)
f = a.run('frame.create', {'x': 0, 'y': 0, 'width': 600, 'height': 840, 'name': 'Poster'})
fid = f.get('id') or (f.get('ids') or [None])[0]
a.run('node.set', {'id': fid, 'fills': ['#F1EFE4']})
for (x, y, w, h, col) in ((40, 40, 520, 360, '#7F7500'), (40, 420, 250, 250, '#36BBFF'), (310, 420, 250, 250, '#CEC5FA')):
    r = a.run('shape.rectangle', {'x': x, 'y': y, 'width': w, 'height': h, 'parent': fid})
    a.run('node.set', {'id': r.get('id') or (r.get('ids') or [None])[0], 'fills': [col]})
poster = os.path.join(docs, 'Poster.scrim')
print('saveAs (no extension given):', a.run('file.saveAs', {'path': poster[:-len('.scrim')]}))
with zipfile.ZipFile(poster) as z:
    print('  .scrim container:', sorted(n for n in z.namelist() if not n.startswith('images/')), '| format', json.loads(z.read('document.json'))['format'])
first = digest(poster)
a.run('file.save', {}); print('  second save byte-stable:', digest(poster) == first)
state(a, '1 saved'); print('   shot', shot(a, '1-saved-as-scrim'))
a.run('file.close', {'force': True}); time.sleep(0.3)
m = Mcp()
err, text = m.tool('open_file', {'path': poster})
r = json.loads(text)
print('  MCP open_file .scrim: isError', err, '| name', r['document'].get('name'), '| path', r['document'].get('path') == poster, '| repairs', r['opened'].get('repairs'))
state(a, '2 reopened'); print('   shot', shot(a, '2-reopened'))
a.run('file.close', {'force': True}); time.sleep(0.3)

# 3. A .figcraft (the same format under its earlier name): opened, edited, Save. Save asks for a
# .scrim beside it (read from the log); the .figcraft is never written.
old = os.path.join(docs, 'Festival.figcraft')
shutil.copyfile(poster, old)
before = digest(old)
err, text = m.tool('open_file', {'path': old})
print('  MCP open_file .figcraft: isError', err, '| name', json.loads(text)['document'].get('name'))
a.run('shape.rectangle', {'x': 700, 'y': 0, 'width': 80, 'height': 80})
err, text = m.tool('save_file', {})
print('  MCP save_file without a path on it: isError', err, '|', text[:120])
state(a, '3 .figcraft opened and edited'); print('   shot', shot(a, '3-figcraft-opened'))
before_menu = open(log_path).read().count('Save as: suggesting')
a.call('ui.menu.invoke', {'id': 'file.save'}); time.sleep(1.5)
lines = [l.strip() for l in open(log_path) if 'Save as: suggesting' in l]
print('  File > Save asked for:', lines[before_menu:] or lines[-1:])
print('  .figcraft unchanged:', digest(old) == before, '| files', sorted(os.listdir(docs)))
quit(p, a)  # closes the native Save panel with the app

# 4. The Sketch sample, opened as Finder would open it (the app started with the file): a new
# unsaved document and its import report. Home after.
sample = os.path.join(ROOT, 'crates', 'format', 'tests', 'fixtures', 'sketch', 'sample.sketch')
local = os.path.join(docs, 'sample.sketch'); shutil.copyfile(sample, local)
p, a = launch(local)
a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(1.0)
print(f'frames after {live_frames(a)} focus')
for _ in range(20):
    if a.call('ui.inspect').get('result', {}).get('importReport'): break
    idle(a, 2)
i = state(a, '4 Sketch import')
print('  report:', json.dumps((i.get('importReport') or {}).get('warnings', [])[:4]))
print('   shot', shot(a, '4-sketch-import-report'))
err, text = Mcp().tool('open_file', {'path': local})
w = json.loads(text)['opened'].get('warnings') or []
print('  MCP open_file .sketch: isError', err, '|', len(w), 'warnings, e.g.', w[:2])
print('  sample unchanged:', digest(local) == digest(sample))
a.call('ui.set', {'home': True}); idle(a, 10); time.sleep(0.8); idle(a, 4)
print('   shot', shot(a, '5-home-after'))
print('log errors:', [l.strip() for l in open(log_path) if 'panick' in l.lower()][:5])
print('captures in', out)
quit(p, a)
