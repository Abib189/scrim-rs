"""aa5_measure.py APP SCRATCH OUT.json — Fix AA5: the real app's repaints with nothing happening.
A scratch HOME and settings folder (SCRIM_SETTINGS_DIR), MCP on 7690 (never 7680), control 7979, a
stub `claude` first on PATH (crates/mcp/tests/stub/stub_agent.py), allowed in the scratch settings.
Cases, 10 s each, with nothing sent to the app meanwhile: (a) AI tab, empty; (c) Design tab;
(b) AI tab after one stub run; (d) a tooltip on a card button; (e) the mouse over the panel."""
import json, os, subprocess, sys, time

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'docs', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App

app_bin, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
stub = os.path.join(REPO, 'crates', 'mcp', 'tests', 'stub', 'stub_agent.py')
home, settings, bindir = (os.path.join(scratch, d) for d in ('home', 'settings', 'bin'))
for d in (home, settings, bindir, os.path.join(home, 'Documents', 'Scrim')):
    os.makedirs(d, exist_ok=True)
claude = os.path.join(bindir, 'claude')
open(claude, 'w').write(f'#!/bin/sh\nexec /usr/bin/python3 {stub} "$@"\n')
os.chmod(claude, 0o755)
json.dump({'prefs': {'theme': 'light', 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False},
                     'chat_allowed': [{'agent': 'claude-code', 'path': claude}]}}, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings, PATH=f'{bindir}:/usr/bin:/bin')
proc = subprocess.Popen([app_bin, '--control', '7979'], env=env, cwd=home, stdout=open(os.path.join(scratch, 'app.log'), 'w'), stderr=subprocess.STDOUT)
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
port = a.run('mcp.status').get('port')
if port == 7680:
    raise SystemExit("on 7680, the person's own Scrim")
results = {'mcpPort': port}

def inspect():
    return a.call('ui.inspect').get('result', {})

def measure(name, leave=True, settle=4.0):
    a.call('window.focus', {'id': 0})
    if leave:
        a.call('ui.input', {'events': [{'leave': True}]})
    time.sleep(settle)
    a.call('ui.set', {'resetFrameStats': True})
    time.sleep(10.0)
    i = inspect()
    r = {'framesIn10s': i.get('framesDrawn'), 'repaintsPerSecond': i.get('repaintsPerSecond'),
         'recent': i.get('repaintCausesRecent'), 'causes10s': i.get('repaintCauses')}
    results[name] = r
    print(name, json.dumps({k: r[k] for k in ('framesIn10s', 'repaintsPerSecond')}), json.dumps(r['recent'])[:600], flush=True)
    return i

a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False, 'rightTab': 'ai'})
time.sleep(2)
measure('a_ai_empty')
a.call('ui.set', {'rightTab': 'design'})
measure('c_design')
a.call('ui.set', {'rightTab': 'ai'})
time.sleep(1)
p = inspect().get('agent', {}).get('aiPanel', {})
c = p.get('composer', {}).get('composerRect') or [0, 0, 0, 0]
fx, fy = (c[0] + c[2]) / 2, c[1] + 24
a.call('window.focus', {'id': 0}); time.sleep(0.5)
a.call('ui.input', {'events': [{'move': [fx, fy]}]}); time.sleep(0.2)
a.call('ui.input', {'events': [{'down': [fx, fy]}]}); time.sleep(0.1)
a.call('ui.input', {'events': [{'up': [fx, fy]}]}); time.sleep(0.3)
a.call('ui.input', {'events': [{'text': 'rectangle named Poster'}]})
time.sleep(0.3)
a.call('ui.input', {'events': [{'key': 'Enter', 'cmd': True}]})
state = None
for _ in range(120):
    time.sleep(0.5)
    card = inspect().get('agent', {}).get('aiPanel', {}).get('card') or {}
    state = card.get('state')
    if state in ('done', 'stopped'):
        break
results['run'] = {'cardState': state, 'heading': card.get('heading'), 'rows': card.get('rows')}
print('run', state, card.get('heading'), flush=True)
i = measure('b_ai_card_composer_focused')
card = i.get('agent', {}).get('aiPanel', {}).get('card') or {}
cr = card.get('rect') or [0, 0, 0, 0]
# A click on the card's blank top-right corner takes the focus off the composer.
for ev in ({'move': [cr[2] - 6, cr[1] + 6]}, {'down': [cr[2] - 6, cr[1] + 6]}, {'up': [cr[2] - 6, cr[1] + 6]}):
    a.call('ui.input', {'events': [ev]}); time.sleep(0.15)
measure('b_ai_card')
rects = card.get('buttonRects') or []
if rects:
    r = rects[0]
    x, y = (r[0] + r[2]) / 2, (r[1] + r[3]) / 2
    a.call('ui.input', {'events': [{'move': [x - 300, y]}]}); time.sleep(0.2)
    a.call('ui.input', {'events': [{'move': [x, y]}]})
    time.sleep(1.5)
    measure('d_tooltip_on_card_button', leave=False, settle=1.0)
    pr = i.get('agent', {}).get('aiPanel', {}).get('headerRect') or [0, 0, 0, 0]
    a.call('ui.input', {'events': [{'move': [(pr[0] + pr[2]) / 2, pr[3] + 120]}]})
    measure('e_mouse_over_panel', leave=False, settle=2.0)
json.dump(results, open(out, 'w'), indent=1)
a.call('window.focus', {'id': 0})
a.call('app.quit', {})
try:
    proc.wait(timeout=20)
    print('quit', proc.returncode)
except subprocess.TimeoutExpired:
    proc.kill()
    print('killed after 20 s')
