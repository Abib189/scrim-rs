"""ab6_design.py APP SCRATCH WINBOUNDS: Fix AB6, the Design tab's idle repaints on a scratch app
(scratch HOME and settings, MCP on 7690, control 7979), with the system pointer sampled before and
after each 10 s, and the scratch window's bounds, so a reading with the pointer over the window is
known as such. Up to three tries; prints each."""
import json, os, subprocess, sys, time

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'docs', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App

app_bin, scratch, wb = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, os.path.join(home, 'Documents', 'Scrim')):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'theme': 'light', 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False}}}, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
proc = subprocess.Popen([app_bin, '--control', '7979'], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(os.path.join(scratch, 'app.log'), 'w'), stderr=subprocess.STDOUT)
a = None
for _ in range(240):
    try:
        a = App(7979)
        a.run('mcp.status')
        break
    except Exception:
        time.sleep(0.25)
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('not on a scratch settings folder')
if a.run('mcp.status').get('port') == 7680:
    raise SystemExit("on 7680, the person's own Scrim")

def sample():
    out = subprocess.run([wb, str(proc.pid)], capture_output=True, text=True, timeout=20, stdin=subprocess.DEVNULL).stdout.split('\n')
    wins = [tuple(map(int, l.split()[1:])) for l in out if l.startswith('window')]
    ptr = next((tuple(map(int, l.split()[1:])) for l in out if l.startswith('pointer')), None)
    inside = any(x <= ptr[0] <= x + w and y <= ptr[1] <= y + h for x, y, w, h in wins) if ptr else None
    return {'windows': wins, 'pointer': ptr, 'pointerInside': inside}

a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False, 'rightTab': 'design'})
for attempt in range(3):
    a.call('window.focus', {'id': 0})
    a.call('ui.input', {'events': [{'leave': True}]})
    time.sleep(4)
    before = sample()
    a.call('ui.set', {'resetFrameStats': True})
    time.sleep(10)
    i = a.call('ui.inspect').get('result', {})
    after = sample()
    print(json.dumps({'try': attempt + 1, 'before': before, 'after': after, 'framesIn10s': i.get('framesDrawn'),
                      'repaintsPerSecond': i.get('repaintsPerSecond'), 'recent': [(c['at'].split('/')[-1], c['frames']) for c in i.get('repaintCausesRecent') or []]}), flush=True)
    if not before['pointerInside'] and not after['pointerInside']:
        break
a.call('window.focus', {'id': 0})
try:
    a.call('app.quit', {})
except ValueError:
    pass
try:
    proc.wait(timeout=20)
    print('quit', proc.returncode)
except subprocess.TimeoutExpired:
    proc.kill()
    print('killed after 20 s')
