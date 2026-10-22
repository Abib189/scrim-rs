"""s2.py BINARY SCRATCH OUT — Fix S2: the input edges in the real window, light and dark.
Starts the app on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules
9; MCP on 7690, never 7680), opens examples/ridgeline-app.figcraft, and captures with `ui.screenshot`:
1. the properties panel (a frame selected: its number fields and paint rows) with the Layers search
   open;
2. Settings ▸ General (its text fields, dropdowns and the nudge amounts);
3. the AI tab with the Describe a change composer.
Each in the light and the dark theme. Then quits."""
import json, os, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 7}}, open(os.path.join(settings, 'state.json'), 'w'))
doc = os.path.join(os.path.dirname(os.path.abspath(sys.argv[0])), '../../../examples/ridgeline-app.figcraft')
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
app = subprocess.Popen([exe, '--control', '7979', os.path.abspath(doc)], env=env, stdout=open(os.path.join(out, 'app.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
for _ in range(120):
    try:
        a = App(7979)
        a.run('mcp.status')
        break
    except Exception:
        time.sleep(0.25)
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')
a.call('window.focus', {'id': 0})
a.call('ui.set', {'windowSize': [1440, 900], 'home': False})
time.sleep(1.0)
frame = a.run('node.query', {'query': '//FRAME'})
first = (frame.get('nodes') or frame.get('results') or [{}])[0].get('id') if isinstance(frame, dict) else None


def shot(name):
    a.call('ui.input', {'events': [{'move': [700, 880]}]})
    time.sleep(0.8)
    r = a.call('ui.screenshot', {'path': os.path.join(out, name + '.png')})
    print(name, r.get('ok'), r.get('error'), flush=True)


for theme in ('light', 'dark'):
    a.call('ui.set', {'theme': theme, 'settings': False})
    if first is not None:
        a.run('select.set', {'ids': [first]})
    a.call('ui.set', {'rightTab': 'design', 'leftTab': 'file', 'layerFilter': 'Trail'})
    shot(f'{theme}-1-properties-and-layers-search')
    a.call('ui.set', {'layerFilter': None, 'settings': 'general'})
    shot(f'{theme}-2-settings-general')
    a.call('ui.set', {'settings': False, 'rightTab': 'ai'})
    shot(f'{theme}-3-ai-tab-composer')
a.call('app.quit', {})
try:
    app.wait(timeout=20)
except subprocess.TimeoutExpired:
    app.terminate()
