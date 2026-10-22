"""s3.py BINARY SCRATCH OUT — Fix S3: the window buttons over the tab strip, in the real window.
Starts the app on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules 9;
MCP on 7690, never 7680). Each capture is of one window only, by its window id
(`screencapture -l`, with macOS's own red, yellow and green buttons), never the screen:
the main window on Home and in the editor, then a second window (`window.new`), light and dark."""
import json, os, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
sys.path.insert(0, HERE)
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 7}}, open(os.path.join(settings, 'state.json'), 'w'))
helper = os.path.join(scratch, 'window_id')
subprocess.run(['swiftc', '-O', os.path.join(HERE, 'window_id.swift'), '-o', helper], check=True)
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, 'app.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
for _ in range(160):
    try:
        a = App(7979)
        a.run('mcp.status')
        break
    except Exception:
        time.sleep(0.25)
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')


def windows():
    rows = subprocess.run([helper, str(app.pid)], capture_output=True, text=True).stdout.split('\n')
    return [r.split() for r in rows if r.strip()]


def capture(name, index=0):
    time.sleep(1.2)
    w = windows()
    if len(w) <= index:
        print(name, 'no window', w, flush=True)
        return
    path = os.path.join(out, name + '.png')
    subprocess.run(['screencapture', '-x', '-o', '-l', w[index][0], path], check=True)
    print(name, 'window', w[index][1:], flush=True)


a.call('window.focus', {'id': 0})
a.call('ui.set', {'windowSize': [1200, 760]})
a.run('file.new')
for theme in ('light', 'dark'):
    a.call('ui.set', {'theme': theme, 'home': True})
    capture(f'{theme}-1-home')
    a.call('ui.set', {'home': False})
    capture(f'{theme}-2-editor')
r = a.call('window.new', {})
print('window.new', r.get('ok'), r.get('error'), flush=True)
capture('dark-3-second-window', 0)
a.call('app.quit', {})
try:
    app.wait(timeout=20)
except subprocess.TimeoutExpired:
    app.terminate()
