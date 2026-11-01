"""live.py: Fix AH's live check on a scratch app (scratch HOME and settings, control 7979, MCP 7690).
Each file is opened as Finder or a double-click on Linux and Windows would (the app started with
it): the Penpot sample with its import report (light and dark), the Sketch and FREE samples, a dummy
Affinity file (the closed-format dialog), then Home with the samples' previews. The system pointer
is never moved. Run from the repository root after `cargo build -p figcraft`:
`python3 docs/design/import-formats/live.py`."""
import hashlib, json, os, shutil, subprocess, sys, tempfile, time, atexit, urllib.request
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
HERE = tempfile.mkdtemp(prefix='scrim-fix-ah-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
scratch = os.path.join(HERE, 'scratch')
home, settings, out = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings'), os.path.join(HERE, 'out')
docs = os.path.join(home, 'Documents', 'Scrim')
for d in (home, settings, docs, out): os.makedirs(d, exist_ok=True)
log_path = os.path.join(scratch, 'app.log')
fixtures = os.path.join(ROOT, 'crates', 'format', 'tests', 'fixtures')
files = {}
for kind, name in (('penpot', 'sample.penpot'), ('sketch', 'sample.sketch'), ('free', 'sample.free')):
    src = os.path.join(fixtures, kind, name)
    if os.path.exists(src):
        files[kind] = os.path.join(docs, name); shutil.copyfile(src, files[kind])
files['closed'] = os.path.join(docs, 'logo.afdesign')
open(files['closed'], 'wb').write(b'a dummy file named like an Affinity document')
def digest(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()[:12]
before = {k: digest(v) for k, v in files.items()}

def settings_with(theme, recents):
    json.dump({'prefs': {'theme': theme, 'getting_started_dismissed': True, 'recent_files': recents,
                         'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))

def launch(*args):
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
    p = subprocess.Popen([APP, '--control', '7979', *args], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(log_path, 'a'), stderr=subprocess.STDOUT)
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
    a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(1.0)
    return p, a

def inp(a, *evs):
    a.call('ui.input', {'events': list(evs)}); time.sleep(0.12)
def idle(a, n=6):
    for _ in range(n): inp(a, {'move': [5, 790]}); time.sleep(0.05)
def shot(a, name):
    for k in range(5):
        a.call('window.focus', {'id': 0}); time.sleep(0.3)
        p = os.path.join(out, name + '.png')
        if os.path.exists(p): os.remove(p)
        a.call('ui.screenshot', {'path': p})
        for _ in range(40):
            idle(a, 1)
            if os.path.exists(p) and os.path.getsize(p) > 0: time.sleep(0.3); return name + '.png'
    return None
def ui(a): return a.call('ui.inspect').get('result', {})
def quit(p, a):
    a.call('window.focus', {'id': 0})
    try: a.call('app.quit', {})
    except ValueError: pass
    try: p.wait(timeout=20); print('  quit', p.returncode)
    except subprocess.TimeoutExpired: p.kill(); print('  killed after 20 s')
def mcp_open(path):
    url = 'http://127.0.0.1:7690/mcp'
    def post(body, sid=None, raw=False):
        h = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
        if sid: h['Mcp-Session-Id'] = sid
        r = urllib.request.urlopen(urllib.request.Request(url, data=json.dumps(body).encode(), headers=h, method='POST'), timeout=60)
        return r if raw else r.read()
    r = post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-ah-live', 'version': '1'}}}, raw=True)
    sid = r.headers['Mcp-Session-Id']; post({'jsonrpc': '2.0', 'method': 'notifications/initialized'}, sid)
    d = json.loads(post({'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'open_file', 'arguments': {'path': path}}}, sid))
    return d['result'].get('isError'), d['result']['content'][0].get('text', '')

def imported(kind, themes=('light',)):
    if kind not in files: print(f'{kind}: no sample'); return
    for theme in themes:
        settings_with(theme, [])
        p, a = launch(files[kind])
        for _ in range(30):
            if ui(a).get('importReport'): break
            idle(a, 2)
        i = ui(a); rep = i.get('importReport') or {}
        print(f'{kind} ({theme}): tabs {[t["name"] for t in i.get("tabs", [])]} | counts {rep.get("counts")} | {len(rep.get("warnings") or [])} warnings')
        for w in (rep.get('warnings') or [])[:12]: print(f'    [{w["severity"]}] {w["path"]}: {w["message"]}')
        print('   shot', shot(a, f'{kind}-report-{theme}'))
        if theme == themes[0]:
            err, text = mcp_open(files[kind])
            w = json.loads(text)['opened'].get('warnings') or [] if not err else []
            print(f'  MCP open_file: isError {err} | {len(w)} warnings')
        quit(p, a)

imported('penpot', ('light', 'dark'))
imported('sketch')
imported('free', ('light', 'dark'))

# The closed-format dialog, for a file named like an Affinity document.
settings_with('light', [])
p, a = launch(files['closed'])
idle(a, 6)
print('closed format: tabs', [t['name'] for t in ui(a).get('tabs', [])], '| home', ui(a).get('home'))
print('   shot', shot(a, 'closed-format-dialog'))
quit(p, a)

# Home with the samples in the recent files: their previews.
settings_with('light', [v for k, v in files.items() if k != 'closed'])
p, a = launch()
a.call('ui.set', {'home': True})
for _ in range(40):
    fs = (ui(a).get('homeRecents') or {}).get('files', [])
    if fs and all(f.get('picture') not in ('loading', 'none') for f in fs): break
    idle(a, 2)
print('Home recents:', [(os.path.basename(f['path']), f['name'], f['picture']) for f in (ui(a).get('homeRecents') or {}).get('files', [])])
print('   shot', shot(a, 'home-previews'))
quit(p, a)
print('sources unchanged:', {k: digest(v) == before[k] for k, v in files.items()})
print('log errors:', [l.strip() for l in open(log_path) if 'panick' in l.lower()][:5])
print('captures in', out)
