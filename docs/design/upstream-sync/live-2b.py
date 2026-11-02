"""live-2b.py: Fix AI stage 2b's live check (the lost-work fixes) on a scratch app: a scratch HOME
and SCRIM_SETTINGS_DIR, control 7979, MCP 7690. Never port 7680, never the person's settings, files
or pointer: input goes through the window's own control channel (`ui.key`, `ui.input`), and files
are written by this script in its scratch folder.

Steps:
- LW-1: ruler guides added to a page, saved, the file closed and reopened: the guides are there;
  a file with the page's `guides` key twice opens, unchanged on disk, and saves a valid file.
- LW-2: a file with `null` numbers opens with them reset and one note, unchanged on disk.
- LW-3: a file holding more images than the budget (20,005 images, past the 20,000-image budget)
  opens, is edited and saved: the saved file still holds every image.
- LW-4: Save (Cmd+S) after another process changed the file shows the three choices and Escape
  cancels (file untouched); the choices' commands each do what they say; auto-save leaves a
  changed file alone.
- LW-5: a version preview open when the app is killed: the next start recovers the working
  document, not the version.
Run from the repository root after `cargo build -p figcraft`:
`python3 docs/design/upstream-sync/live-2b.py`."""
import atexit, hashlib, json, os, signal, struct, subprocess, sys, tempfile, time, zipfile, zlib
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
HERE = tempfile.mkdtemp(prefix='scrim-fix-ai-2b-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
CONTROL, MCP = 7979, 7690
results = []
T0 = time.time()
def step(m): print(f'[{time.time()-T0:6.1f}s] {m}', flush=True)

def check(what, ok, detail=''):
    results.append((what, bool(ok)))
    print(('PASS ' if ok else 'FAIL ') + what + (f' ({detail})' if detail else ''), flush=True)

HOME = os.path.join(HERE, 'home'); SETTINGS = os.path.join(HERE, 'settings')
DOCS = os.path.join(HOME, 'Documents', 'Scrim')
for d in (HOME, SETTINGS, DOCS): os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'theme': 'light', 'getting_started_dismissed': True,
                     'mcp': {'port': MCP, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(SETTINGS, 'state.json'), 'w'))
LOG = os.path.join(HERE, 'app.log')
PROCS = []

def launch():
    env = dict(os.environ, HOME=HOME, SCRIM_SETTINGS_DIR=SETTINGS)
    p = subprocess.Popen([APP, '--control', str(CONTROL)], env=env, cwd=HOME, stdin=subprocess.DEVNULL, stdout=open(LOG, 'a'), stderr=subprocess.STDOUT)
    PROCS.append(p)
    a = None
    for _ in range(240):
        try: a = App(CONTROL); a.run('mcp.status'); break
        except Exception: time.sleep(0.25)
    if a is None: raise SystemExit('the app did not answer on its control port')
    if not (a.call('app.settings-dir', {}).get('result', {}) or {}).get('override'): raise SystemExit('not on scratch settings: stopping')
    for _ in range(40):
        if a.run('mcp.status').get('port') == MCP: break
        time.sleep(0.25)
    else: raise SystemExit(f'MCP is not on {MCP}: stopping')
    a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(0.5)
    return p, a

atexit.register(lambda: [p.poll() is None and p.terminate() for p in PROCS])

def run(a, cmd, params={}):
    r = a.call('engine.execute', {'command': cmd, 'params': params})
    return r.get('ok'), (r.get('result') if r.get('ok') else (r.get('error') or r))

CAPTURE = {'tried': False, 'path': None}

def shot(a, name):
    """One capture of the window, asked on a connection of its own with a time limit: a window
    macOS doesn't draw never answers, and then no capture is taken."""
    if CAPTURE['tried']:
        return
    CAPTURE['tried'] = True
    path = os.path.join(HERE, name + '.png')
    try:
        c = App(CONTROL); c.s.settimeout(15)
        c.call('ui.screenshot', {'path': path})
        for _ in range(40):
            if os.path.exists(path) and os.path.getsize(path) > 0:
                CAPTURE['path'] = path; return
            time.sleep(0.1)
    except Exception:
        pass
    print('   (no capture: the window draws no frames, so captures are skipped)', flush=True)

def sha(path): return hashlib.sha256(open(path, 'rb').read()).hexdigest()

def document(path):
    with zipfile.ZipFile(path) as z: return json.loads(z.read('document.json'))

def names(path):
    d = document(path)
    return sorted(n['name'] for n in d['nodes'].values() if n.get('type') != 'page')

def page_guides(path):
    d = document(path)
    return [n.get('guides', []) for n in d['nodes'].values() if n.get('type') == 'page'][0]

def edited(path, edit):
    """`path`'s archive written again with document.json passed through `edit` (text)."""
    with zipfile.ZipFile(path) as z: members = [(i.filename, z.read(i.filename)) for i in z.infolist()]
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in members:
            z.writestr(name, edit(data.decode()).encode() if name == 'document.json' else data)

def png(k):
    """A 1x1 PNG, a colour of its own for each k."""
    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    px = bytes([0, k & 255, (k >> 8) & 255, (k >> 16) & 255, 255])
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(px)) + chunk(b'IEND', b'')

p, a = launch()
try:
    # LW-1: guides through save, close and reopen.
    step('LW-1: ruler guides survive save and reopen')
    run(a, 'file.new', {})
    run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 40, 'height': 40, 'name': 'Card'})
    run(a, 'guide.add', {'vertical': True, 'offset': 120})
    run(a, 'guide.add', {'vertical': False, 'offset': 64})
    guides = os.path.join(DOCS, 'Guides.scrim')
    ok, _ = run(a, 'file.save', {'path': guides})
    run(a, 'file.close', {'force': True})
    ok2, r = run(a, 'file.open', {'path': guides})
    moved = run(a, 'guide.move', {'index': 1, 'offset': 64})[0] and not run(a, 'guide.move', {'index': 2, 'offset': 1})[0]
    run(a, 'guide.add', {'vertical': True, 'offset': 300})
    ok3, _ = run(a, 'file.save', {})
    check('LW-1 guides are there after save, close and reopen, and a second save opens',
          ok and ok2 and moved and ok3 and len(page_guides(guides)) == 3, f'{page_guides(guides)}')
    run(a, 'file.close', {'force': True})
    broken = os.path.join(DOCS, 'Broken guides.scrim')
    run(a, 'file.new', {}); run(a, 'file.save', {'path': broken}); run(a, 'file.close', {'force': True})
    edited(broken, lambda t: t.replace('"type":"page"', '"type":"page","guides":[{"vertical":false,"offset":64.0}],"guides":[{"vertical":true,"offset":120.0}]', 1))
    before = sha(broken)
    ok, r = run(a, 'file.open', {'path': broken})
    unchanged = sha(broken) == before
    copy = os.path.join(DOCS, 'Broken guides saved.scrim')
    ok2, _ = run(a, 'file.saveAs', {'path': copy})
    check('LW-1 a file with the page guides key twice opens unchanged on disk and saves a valid file',
          ok and unchanged and ok2 and len(page_guides(copy)) == 2, f'{r if not ok else page_guides(copy)}')
    run(a, 'file.close', {'force': True})

    # LW-2: null numbers.
    step('LW-2: a file with null numbers opens')
    nulls = os.path.join(DOCS, 'Null numbers.scrim')
    run(a, 'file.new', {}); run(a, 'shape.rectangle', {'x': 5, 'y': 5, 'width': 10, 'height': 10, 'name': 'Badge'})
    run(a, 'file.save', {'path': nulls}); run(a, 'file.close', {'force': True})
    def nullify(t):
        d = json.loads(t)
        for n in d['nodes'].values():
            if n.get('name') == 'Badge': n['opacity'] = None; n['width'] = None
        return json.dumps(d)
    edited(nulls, nullify)
    before = sha(nulls)
    ok, r = run(a, 'file.open', {'path': nulls})
    reps = (r or {}).get('repairs', []) if ok else []
    check('LW-2 a file with null numbers opens with one note, unchanged on disk',
          ok and sha(nulls) == before and sum('null' in x for x in reps) == 1, f'{reps if ok else r}')
    run(a, 'file.close', {'force': True})

    # LW-3: more images than the budget.
    step('LW-3: 20,005 images, past the 20,000-image budget')
    big = os.path.join(DOCS, 'Many images.scrim')
    run(a, 'file.new', {}); run(a, 'file.save', {'path': big}); run(a, 'file.close', {'force': True})
    imgs = {}
    for k in range(20005):
        b = png(k); imgs[hashlib.sha256(b).hexdigest()] = b
    with zipfile.ZipFile(big) as z: doc = json.loads(z.read('document.json'))
    doc['imageIndex'] = [{'hash': h, 'width': 1, 'height': 1} for h in imgs]
    with zipfile.ZipFile(big, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('document.json', json.dumps(doc))
        for h, b in imgs.items(): z.writestr('images/' + h, b)
    ok, r = run(a, 'file.open', {'path': big})
    run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 10, 'height': 10, 'name': 'Edit'})
    ok2, r2 = run(a, 'file.save', {})
    with zipfile.ZipFile(big) as z: kept = sum(1 for n in z.namelist() if n.startswith('images/'))
    check('LW-3 saving a file past the image budget keeps every image', ok and ok2 and kept == 20005, f'{kept} images in the saved file; {r2 if not ok2 else ""}')
    run(a, 'file.close', {'force': True})

    # LW-4: another process changes the open file.
    step('LW-4: Save after another process changed the file')
    mine = os.path.join(DOCS, 'Shared.scrim')
    theirs_src = os.path.join(HERE, 'Theirs.scrim')
    run(a, 'file.new', {}); run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 70, 'height': 70, 'name': 'Theirs'})
    run(a, 'file.save', {'path': theirs_src}); run(a, 'file.close', {'force': True})
    theirs = open(theirs_src, 'rb').read()
    run(a, 'file.new', {}); run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 10, 'height': 10, 'name': 'Mine'})
    run(a, 'file.save', {'path': mine})
    def change_it():
        run(a, 'shape.rectangle', {'x': 20, 'y': 0, 'width': 10, 'height': 10, 'name': 'Unsaved'})
        open(mine, 'wb').write(theirs)
    change_it()
    a.call('ui.key', {'key': 's', 'cmd': True}); time.sleep(0.5)
    asked = (a.call('ui.inspect', {}).get('result') or {}).get('saveConflict')
    shot(a, 'save-conflict')
    a.call('ui.input', {'events': [{'key': 'Escape'}]}); time.sleep(0.5)
    after = (a.call('ui.inspect', {}).get('result') or {}).get('saveConflict')
    check('LW-4 Cmd+S shows the three choices and Escape cancels, the file untouched',
          asked == 'Shared' and after is None and open(mine, 'rb').read() == theirs, f'asked={asked!r} after={after!r}')
    ok, _ = run(a, 'file.save', {'overwrite': True})
    check('LW-4 Overwrite (file.save overwrite) replaces their changes', ok and 'Unsaved' in names(mine), f'{names(mine)}')
    change_it()
    ok, _ = run(a, 'file.revert', {})
    ok2, r = run(a, 'file.saveAs', {'path': os.path.join(HERE, 'reloaded.scrim')})
    check('LW-4 Reload from disk (file.revert) brings their file in', ok and ok2 and names(os.path.join(HERE, 'reloaded.scrim')) == ['Theirs'], '')
    run(a, 'file.close', {'force': True})
    run(a, 'file.open', {'path': mine})
    change_it()
    copy = os.path.join(DOCS, 'Shared copy.scrim')
    ok, _ = run(a, 'file.saveAs', {'path': copy})
    check('LW-4 Save as a copy (file.saveAs) never asks and leaves their file', ok and open(mine, 'rb').read() == theirs and 'Unsaved' in names(copy), '')
    run(a, 'file.close', {'force': True})
    # Auto-save.
    run(a, 'pref.autoSave', {'on': True})
    run(a, 'file.open', {'path': mine})
    change_it()
    time.sleep(6)
    check('LW-4 auto-save leaves a file another process changed alone', open(mine, 'rb').read() == theirs, '')
    run(a, 'pref.autoSave', {'on': False})
    run(a, 'file.close', {'force': True})

    # LW-5: a crash during a version preview.
    step('LW-5: kill the app during a version preview')
    run(a, 'file.new', {}); run(a, 'shape.rectangle', {'x': 0, 'y': 0, 'width': 10, 'height': 10, 'name': 'Version one'})
    run(a, 'file.saveVersion', {'name': 'One'})
    run(a, 'shape.rectangle', {'x': 20, 'y': 0, 'width': 10, 'height': 10, 'name': 'Working'})
    ok, _ = run(a, 'version.preview', {'index': 0})
    # A recovery copy is made every 10 s while there are unsaved changes, on the window's frames:
    # each control call draws one, so ask until the copy of this document is there.
    def copied():
        for root, _, files in os.walk(os.path.join(SETTINGS, 'Recovery')):
            for f in files:
                try:
                    if f.endswith('.scrim') and 'Working' in names(os.path.join(root, f)): return True
                except Exception: pass
        return False
    for _ in range(90):
        if copied(): break
        a.call('ui.inspect', {}); time.sleep(0.5)
    step(f'recovery copy written: {copied()}')
    os.kill(p.pid, signal.SIGKILL); p.wait()
    p, a = launch()
    time.sleep(1.5)
    # Every document the start brought back, by its layers.
    ok_list, listed = run(a, 'doc.list', {})
    docs = listed if ok_list and isinstance(listed, list) else []
    back = []
    for i in range(len(docs)):
        run(a, 'doc.switch', {'index': i})
        rec = os.path.join(HERE, f'recovered-{i}.scrim')
        if run(a, 'file.saveAs', {'path': rec})[0]:
            back.append(names(rec))
    lw5 = [n for n in back if 'Version one' in n]
    check('LW-5 the next start recovers the working document, not the version shown',
          ok and len(lw5) == 1 and 'Working' in lw5[0], f'recovered {back}')
finally:
    try: a.call('app.quit', {})
    except Exception: pass
    time.sleep(1)
    for q in PROCS:
        if q.poll() is None: q.terminate()

print(f'capture: {CAPTURE["path"] or "none"}')
print(f'{sum(ok for _, ok in results)}/{len(results)} passed')
sys.exit(0 if all(ok for _, ok in results) else 1)
