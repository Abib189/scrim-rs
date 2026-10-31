"""live.py: Fix AF's live check on a scratch app (scratch HOME and settings, control 7979, MCP 7690).
A poster file is made the way an agent makes one (file.new, then saved without a name, so it
stores "Untitled"), then Home, the tab, the left panel's header and the window title are captured
after opening it through MCP open_file, after a Save as and after a rename in the panel. Text goes
through the window's own input (ui.input); the system pointer is never moved. Run from the
repository root after `cargo build -p figcraft`: `python3 docs/design/doc-name/live.py`."""
import hashlib, json, os, shutil, subprocess, sys, tempfile, time, atexit, urllib.request, zipfile
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
HERE = tempfile.mkdtemp(prefix='scrim-fix-af-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
scratch = os.path.join(HERE, 'scratch')
home, settings, out = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings'), os.path.join(HERE, 'out')
docs = os.path.join(home, 'Documents', 'Scrim')
for d in (home, settings, docs, out): os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'theme': 'light', 'getting_started_dismissed': True, 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
proc = subprocess.Popen([APP, '--control', '7979'], env=dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings), cwd=home, stdin=subprocess.DEVNULL, stdout=open(os.path.join(scratch, 'app.log'), 'w'), stderr=subprocess.STDOUT)
atexit.register(lambda: proc.poll() is None and proc.terminate())
a = None
for _ in range(240):
    try: a = App(7979); a.run('mcp.status'); break
    except Exception: time.sleep(0.25)
if not (a.call('app.settings-dir', {}).get('result', {}) or {}).get('override'): raise SystemExit('not on scratch settings')
for _ in range(40):
    if a.run('mcp.status').get('port') == 7690: break
    time.sleep(0.25)
else: raise SystemExit('MCP is not on 7690: stopping')

class Mcp:
    URL = 'http://127.0.0.1:7690/mcp'
    def __init__(self):
        self.sid, self.n = None, 1
        r = self._post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-af-live', 'version': '1'}}}, raw=True)
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

def inp(*evs):
    a.call('ui.input', {'events': list(evs)}); time.sleep(0.12)
def idle(n=6):
    for _ in range(n): inp({'move': [700, 790]}); time.sleep(0.05)
def shot(name):
    p = os.path.join(out, name + '.png')
    a.call('ui.screenshot', {'path': p})
    for _ in range(60):
        idle(1)
        if os.path.exists(p) and os.path.getsize(p) > 0: time.sleep(0.3); return name + '.png'
    return None
def live_frames():
    # A scratch window under other windows draws no frames: focus it until a screenshot comes back.
    for k in range(5):
        a.call('window.focus', {'id': 0}); time.sleep(0.5)
        p = os.path.join(out, '_probe.png')
        if os.path.exists(p): os.remove(p)
        a.call('ui.screenshot', {'path': p})
        for _ in range(20):
            idle(1)
            if os.path.exists(p) and os.path.getsize(p) > 0: os.remove(p); return k + 1
    raise SystemExit('the window draws no frames')
def stored(path):
    with zipfile.ZipFile(path) as z: return json.loads(z.read('document.json'))['name']
def digest(path): return hashlib.sha256(open(path, 'rb').read()).hexdigest()[:12]
def names(tag):
    i = a.call('ui.inspect').get('result', {})
    d = a.call('document.inspect', {'depth': 0}).get('result', {})
    tabs = [t['name'] for t in i.get('tabs', [])]
    win = (i.get('windows') or [{}])[0]
    recents = [(os.path.basename(f['path']), f['name']) for f in (i.get('homeRecents') or {}).get('files', [])]
    print(f'  {tag}: tabs {tabs} | window title {win.get("title")!r} | document.inspect name {d.get("name")!r} | Home recents {recents}')
    return i

a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(1.0)
print(f'frames after {live_frames()} focus')

# The file: a poster made with file.new and saved without a name (as an agent's file.new + save_file).
poster = os.path.join(docs, 'zeichen-26.figcraft')
a.run('file.new'); time.sleep(0.4)
f = a.run('frame.create', {'x': 0, 'y': 0, 'width': 600, 'height': 840, 'name': 'Poster'})
fid = f.get('id') or (f.get('ids') or [None])[0]
a.run('node.set', {'id': fid, 'fills': ['#F1EFE4']})
for (x, y, w, h, col) in ((40, 40, 520, 360, '#7F7500'), (40, 420, 250, 250, '#36BBFF'), (310, 420, 250, 250, '#CEC5FA')):
    r = a.run('shape.rectangle', {'x': x, 'y': y, 'width': w, 'height': h, 'parent': fid})
    a.run('node.set', {'id': r.get('id') or (r.get('ids') or [None])[0], 'fills': [col]})
t = a.run('text.create', {'x': 40, 'y': 700, 'text': 'ZEICHEN 26', 'parent': fid})
a.run('node.set', {'id': t.get('id') or (t.get('ids') or [None])[0], 'fontSize': 72})
print('saved:', a.run('file.saveAs', {'path': poster}))
a.run('file.close', {'force': True}); time.sleep(0.4)
print('stored name in the file:', repr(stored(poster)), '| sha256', digest(poster))
before = digest(poster)

# 1. Home, the file closed.
a.call('ui.set', {'home': True}); idle(12); time.sleep(1.0); idle(6)
names('1 Home'); print('   shot', shot('1-home'))

# 2. Opened through MCP open_file (what an agent does), then the editor.
m = Mcp()
err, text = m.tool('open_file', {'path': poster})
r = json.loads(text)
print('  MCP open_file: isError', err, '| opened.summary.name', repr(r['opened'].get('summary', {}).get('name')), '| document.name', repr(r['document'].get('name')))
a.call('ui.set', {'home': False}); idle(8)
names('2 opened'); print('   shot', shot('2-opened'))
print('   file bytes unchanged by opening:', digest(poster) == before)

# 3. Save as a new name (what the Save as dialog runs once a name is chosen).
renamed = os.path.join(docs, 'Festival poster.figcraft')
print('  saveAs:', a.run('file.saveAs', {'path': renamed})); idle(8)
names('3 saved as'); print('   shot', shot('3-saved-as'))
print('   stored name in the new file:', repr(stored(renamed)), '| zeichen-26 untouched:', digest(poster) == before)
a.call('ui.set', {'home': True}); idle(10); time.sleep(0.6); idle(4)
names('3b Home after Save as'); print('   shot', shot('3b-home-after-save-as'))
a.call('ui.set', {'home': False}); idle(6)

# 4. Rename in the panel: File > Rename starts the header's field, the name selected; type, Enter.
after_save = digest(renamed)
a.call('ui.menu.invoke', {'id': 'file.rename'}); idle(4)
inp({'text': 'Spring festival poster, final, for print at A1 size'}); idle(2)
print('   shot', shot('4a-renaming'))
inp({'key': 'Enter'}); idle(8)
names('4 renamed'); print('   shot', shot('4-renamed'))
files = sorted(os.listdir(docs))
print('   files on disk:', files, '| Festival poster.figcraft untouched until saved:', digest(renamed) == after_save)
a.call('ui.set', {'home': True}); idle(10); time.sleep(0.6); idle(4)
names('4b Home after rename'); print('   shot', shot('4b-home-after-rename'))
a.call('ui.set', {'home': False}); idle(4)

# Saved, the file stores the name and keeps its file name.
print('  save:', a.run('file.save', {}), '| stored name now', repr(stored(renamed)), '| files', sorted(os.listdir(docs)))
err, text = m.tool('inspect_document', {'depth': 0})
print('MCP 7690 inspect_document isError =', err, '| name', repr(json.loads(text).get('name')))
print('log errors:', [l.strip() for l in open(os.path.join(scratch, 'app.log')) if 'panick' in l.lower()][:5])
print('captures in', out)
a.call('window.focus', {'id': 0})
try: a.call('app.quit', {})
except ValueError: pass
try: proc.wait(timeout=20); print('quit', proc.returncode)
except subprocess.TimeoutExpired: proc.kill(); print('killed after 20 s')
