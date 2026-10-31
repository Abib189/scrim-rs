"""live.py: Fix AE's live check on a scratch app (scratch HOME and settings, control 7979, MCP 7690).
Synthetic input goes through the window's egui input (ui.input) with modifier flags; the system
pointer is never moved. Run from the repository root after `cargo build -p figcraft`:
`python3 docs/design/option-drag/live.py` (captures and the scratch HOME go to a temporary folder)."""
import json, os, shutil, subprocess, sys, tempfile, time, atexit, urllib.request
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App
HERE = tempfile.mkdtemp(prefix='scrim-fix-ae-')
APP = os.path.join(os.environ.get('CARGO_TARGET_DIR', 'target'), 'debug', 'figcraft')
scratch = os.path.join(HERE, 'scratch'); shutil.rmtree(scratch, ignore_errors=True)
home, settings, out = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings'), os.path.join(HERE, 'out')
shutil.rmtree(out, ignore_errors=True)
for d in (home, settings, os.path.join(home, 'Documents', 'Scrim'), out): os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'theme': 'light', 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
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
        r = self._post({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'fix-ae-live', 'version': '1'}}}, raw=True)
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

def ids(v): return v.get('id') if isinstance(v, dict) and 'id' in v else (v.get('ids') or [None])[0]
def inp(*evs):
    a.call('ui.input', {'events': list(evs)}); time.sleep(0.12)
def frames(pos, mods, n=4):
    for _ in range(n): inp({'move': pos, **mods}); time.sleep(0.05)
def shot(name, pos, mods):
    p = os.path.join(out, name + '.png')
    a.call('ui.screenshot', {'path': p})
    for _ in range(60):
        frames(pos, mods, 1)
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
            inp({'move': [5, 6]})
            if os.path.exists(p) and os.path.getsize(p) > 0: os.remove(p); return k + 1
    raise SystemExit('the window draws no frames')
def doc():
    return a.call('document.inspect', {'depth': 8}).get('result', {})
def page_kids(d):
    return d['pages'][0].get('children', []) if d.get('pages') else []
def find(nodes, name):
    for n in nodes:
        if n.get('name') == name: return n
        f = find(n.get('children', []) or [], name)
        if f: return f
def summary(d, names):
    out_ = {}
    for nm in names:
        hits = []
        def walk(ns, parent):
            for n in ns:
                if n.get('name') == nm: hits.append((parent, n.get('x'), n.get('y'), n.get('id')))
                walk(n.get('children', []) or [], n.get('name'))
        walk(page_kids(d), 'page'); out_[nm] = hits
    return out_
def state(tag, names):
    d = doc(); i = a.call('ui.inspect').get('result', {})
    print(f'  {tag}: {json.dumps(summary(d, names))} | selection {d.get("selection")} | undo {d.get("history", {}).get("undo", [])[:3]} | cursor {i.get("cursor")} | interaction {(i.get("tool") or {}).get("busy")}')
    return d

a.call('ui.set', {'windowSize': [1280, 800]}); time.sleep(1.0)
a.run('file.new'); time.sleep(0.5)
card = ids(a.run('frame.create', {'x': 100, 'y': 100, 'width': 300, 'height': 200, 'name': 'Card'}))
a.run('node.set', {'id': card, 'fills': ['#F1EFE4']})
inner = ids(a.run('frame.create', {'x': 160, 'y': 30, 'width': 120, 'height': 120, 'name': 'Inner', 'parent': card}))
a.run('node.set', {'id': inner, 'fills': ['#7F7500']})
a.run('shape.ellipse', {'x': 30, 'y': 30, 'width': 60, 'height': 60, 'parent': inner, 'name': 'Dot'})
a.run('shape.rectangle', {'x': 20, 'y': 150, 'width': 80, 'height': 24, 'parent': card, 'name': 'Tag'})
row = ids(a.run('frame.create', {'x': 100, 'y': 400, 'width': 330, 'height': 80, 'name': 'Row'}))
a.run('node.set', {'id': row, 'layout': 'horizontal', 'gap': 10, 'padding': 10, 'sizingH': 'fixed', 'sizingV': 'fixed', 'fills': ['#E6E6E6']})
cells = {}
for nm, col in (('A', '#7F7500'), ('B', '#36BBFF'), ('C', '#CEC5FA')):
    cells[nm] = ids(a.run('shape.rectangle', {'x': 0, 'y': 0, 'width': 60, 'height': 60, 'parent': row, 'name': nm}))
    a.run('node.set', {'id': cells[nm], 'fills': [col]})
chip = ids(a.run('shape.rectangle', {'x': 560, 'y': 420, 'width': 100, 'height': 60, 'name': 'Chip'}))
a.run('node.set', {'id': chip, 'fills': ['#36BBFF']})
a.run('select.set', {'ids': []})
Z, C = 0.75, (450.0, 320.0)
a.run('view.set', {'zoom': Z, 'cx': C[0], 'cy': C[1]}); time.sleep(0.5)
for _ in range(6): inp({'move': [5, 6]}); time.sleep(0.1)
a.run('view.set', {'zoom': Z, 'cx': C[0], 'cy': C[1]})
for _ in range(6): inp({'move': [5, 6]}); time.sleep(0.1)
r = a.call('ui.inspect').get('result', {})['canvas']['rect']
v = a.run('view.set', {})
def scr(x, y): return [round(r[0] + (x - v['cx']) * v['zoom'] + v['vw'] / 2, 1), round(r[1] + (y - v['cy']) * v['zoom'] + v['vh'] / 2, 1)]
print('canvas rect', r, '| view', {k: v[k] for k in ('zoom', 'cx', 'cy', 'vw', 'vh')}, '| page (100,100) at', scr(100, 100), '| MCP port', a.run('mcp.status').get('port'))

ALT, NONE = {'alt': True}, {}
def option_drag(tag, select_ids, start, end, names, held=ALT, steps=12):
    print(f'{tag}: (frames after {live_frames()} focus)')
    a.run('select.set', {'ids': select_ids}); time.sleep(0.3)
    s0, s1 = scr(*start), scr(*end)
    assert r[0] < s1[0] < r[2] and r[1] < s1[1] < r[3], ('drop outside the canvas', s1, r)
    inp({'hold': held}); frames(s0, held)
    state('before', names); print('   shot', shot(f'{tag}-1-before', s0, held))
    inp({'down': s0, **held})
    for k in range(1, steps + 1):
        t = k / steps; p = [s0[0] + (s1[0] - s0[0]) * t, s0[1] + (s1[1] - s0[1]) * t]
        inp({'move': p, **held}); time.sleep(0.04)
    frames(s1, held)
    state('mid-drag', names); print('   shot', shot(f'{tag}-2-mid-drag', s1, held))
    inp({'up': s1, **held}); frames(s1, held)
    state('after release', names); print('   shot', shot(f'{tag}-3-after-release', s1, held))
    inp({'hold': NONE}); frames(s1, NONE)
    # Edit > Undo, the menu item Cmd+Z runs (the control channel's `cmd` flag also sets ctrl, so its Cmd+Z matches no shortcut).
    top = (doc().get('history', {}).get('undo') or [''])[0]
    if top != 'Duplicate': raise SystemExit(f'top undo step is {top!r}, not Duplicate: not undoing')
    print('   undo via menu:', a.call('ui.menu.invoke', {'id': 'edit.undo'}).get('result')); time.sleep(0.4); frames(s1, NONE)
    state('after Edit > Undo (Cmd+Z)', names); print('   shot', shot(f'{tag}-4-after-undo', s1, NONE))

# 1. A frame with children, dropped 420 to the right (one copy, the original stays).
option_drag('1-frame', [card], (370, 280), (700, 280), ['Card', 'Inner', 'Dot', 'Tag'])
# 2. Auto layout: A copied into the slot after C.
option_drag('2-auto-layout', [cells['A']], (140, 440), (395, 440), ['A', 'B', 'C'])
# 3. Option+Shift: Chip dragged right and a little down, locked to the horizontal.
option_drag('3-option-shift', [chip], (610, 450), (850, 470), ['Chip'], held={'alt': True, 'shift': True})

# 4. Option pressed mid-drag, then Escape with Option held: the document is as before.
print(f'4-toggle-escape: (frames after {live_frames()} focus)')
a.run('select.set', {'ids': [chip]}); before = doc(); s0, s1 = scr(610, 450), scr(760, 540)
inp({'hold': NONE}); frames(s0, NONE); inp({'down': s0})
for k in range(1, 7): inp({'move': [s0[0] + (s1[0] - s0[0]) * k / 6, s0[1] + (s1[1] - s0[1]) * k / 6]})
state('moving, no Option', ['Chip'])
inp({'hold': ALT}); frames(s1, ALT)
state('Option pressed, pointer still', ['Chip']); print('   shot', shot('4-1-option-pressed-mid-drag', s1, ALT))
inp({'key': 'Escape', 'alt': True}); frames(s1, ALT)
state('Escape with Option held', ['Chip'])
inp({'up': s1, 'alt': True}); inp({'hold': NONE}); frames(s1, NONE)
after = doc()
print('   document identical to before the press:', json.dumps(after['pages']) == json.dumps(before['pages']), '| undo identical:', after['history'] == before['history'])

m = Mcp()
err, text = m.tool('inspect_document', {'depth': 1})
print('MCP 7690 inspect_document isError =', err, '|', len(text), 'chars')
print('log errors:', [l.strip() for l in open(os.path.join(scratch, 'app.log')) if 'panick' in l.lower()][:5])
a.call('window.focus', {'id': 0})
try: a.call('app.quit', {})
except ValueError: pass
try: proc.wait(timeout=20); print('quit', proc.returncode)
except subprocess.TimeoutExpired: proc.kill(); print('killed after 20 s')
