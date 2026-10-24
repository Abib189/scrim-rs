"""capture.py APP SCRATCH OUTDIR THEME WIDTH: Fix AB7, the AI panel's design states from the real
app. A scratch HOME and settings folder (theme and right panel width seeded; MCP on 7690, never
7680), a stub `claude` first on PATH and allowed. Driven with ui.input like a person; each state's
window shot saved as OUTDIR/{theme}-{state}.png, with the AI panel's rectangle in states.json."""
import atexit, json, os, subprocess, sys, time

# This file is docs/scrim-ports/ai-panel/ab/capture.py.
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'docs', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App

app_bin, scratch, outdir, theme, width = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
stub = os.path.join(REPO, 'crates', 'mcp', 'tests', 'stub', 'stub_agent.py')
home, settings, bindir = (os.path.join(scratch, d) for d in ('home', 'settings', 'bin'))
for d in (home, settings, bindir, os.path.join(home, 'Documents', 'Scrim'), outdir):
    os.makedirs(d, exist_ok=True)
claude = os.path.join(bindir, 'claude')
open(claude, 'w').write(f'#!/bin/sh\nexec /usr/bin/python3 {stub} "$@"\n')
os.chmod(claude, 0o755)
json.dump({'prefs': {'theme': theme, 'panel_widths': [240, width], 'mcp': {'port': 7690, 'port_chosen': True, 'require_auth': False},
                     'chat_allowed': [{'agent': 'claude-code', 'path': claude}]}}, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings, PATH=f'{bindir}:/usr/bin:/bin')
proc = subprocess.Popen([app_bin, '--control', '7979'], env=env, cwd=home, stdin=subprocess.DEVNULL, stdout=open(os.path.join(scratch, 'app.log'), 'w'), stderr=subprocess.STDOUT)
# A run that stops early doesn't leave the scratch app behind.
atexit.register(lambda: proc.poll() is None and proc.terminate())
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
states = {}

def panel():
    return a.call('ui.inspect').get('result', {}).get('agent', {}).get('aiPanel', {})

def click(x, y):
    a.call('window.focus', {'id': 0})
    time.sleep(0.3)
    for ev in ({'move': [x, y]}, {'down': [x, y]}, {'up': [x, y]}):
        a.call('ui.input', {'events': [ev]})
        time.sleep(0.15)

def until(act, check, tries=6):
    """Do `act` until `check()` holds (the window can miss a click while another app is in front)."""
    for _ in range(tries):
        act()
        time.sleep(0.6)
        if check():
            return True
    print('gave up on', act, flush=True)
    return False

def centre(r):
    return (r[0] + r[2]) / 2, (r[1] + r[3]) / 2

def shot(name):
    a.call('window.focus', {'id': 0})
    a.call('ui.input', {'events': [{'move': [5, 5]}]})
    time.sleep(0.8)
    p = panel()
    path = os.path.join(outdir, f'{theme}-{name}.png')
    take(path)
    states[name] = {'header': p.get('headerRect'), 'card': (p.get('card') or {}).get('state')}
    print(name, 'saved', flush=True)


def take(path):
    """The shot is written on the window's next frame: wait for the file, and ask again (with a frame
    nudged by a pointer move) if the window drew none, as when another app covers it."""
    for _ in range(4):
        if os.path.exists(path):
            os.remove(path)
        a.call('window.focus', {'id': 0})
        a.call('ui.screenshot', {'path': path})
        for _ in range(20):
            a.call('ui.input', {'events': [{'move': [5, 6]}]})
            time.sleep(0.25)
            if os.path.exists(path) and os.path.getsize(path) > 0:
                time.sleep(0.3)
                return
    raise SystemExit(f'no shot written: {path}')

a.call('ui.set', {'windowSize': [1440, 900]})
a.call('window.focus', {'id': 0})
a.run('file.new')
f = a.run('frame.create', {'x': 0, 'y': 0, 'width': 400, 'height': 300, 'name': 'Poster — Welle'})
a.run('select.set', {'ids': [f['id']]})
a.call('ui.set', {'home': False, 'rightTab': 'ai'})
time.sleep(1.5)
# A1: + ▸ Add selection.
until(lambda: click(*centre(panel()['composer']['plus']['rect'])), lambda: panel().get('composer', {}).get('plus', {}).get('open') is True)
rows = panel().get('composer', {}).get('plus', {}).get('menu', {}).get('rows') or []
add = next((r for r in rows if r.get('label') == 'Add selection'), None)
if add:
    click(*centre(add['rect']))
time.sleep(0.6)
shot('a1-quiet' if width > 300 else 'a7-quiet-240')
if width > 300:
    # Take the chip off; then the run: typed and sent with ⌘↩.
    chip = lambda: (panel().get('composer', {}).get('selectedChip') or {}).get('rect')
    if chip():
        until(lambda: click(chip()[2] - 15, (chip()[1] + chip()[3]) / 2), lambda: not chip())

    def send():
        c = panel().get('composer', {}).get('composerRect') or [0, 0, 0, 0]
        click((c[0] + c[2]) / 2, c[1] + 24)
        # ⌘A first, so a retry replaces what a missed attempt typed.
        a.call('ui.input', {'events': [{'key': 'A', 'cmd': True}]})
        a.call('ui.input', {'events': [{'text': 'Build a poster called Welle with batches 6 and slow 25'}]})
        time.sleep(0.3)
        a.call('ui.input', {'events': [{'key': 'Enter', 'cmd': True}]})

    # Sent once the card is there (a click the window missed leaves the panel quiet).
    if not until(send, lambda: panel().get('card') is not None):
        raise SystemExit('the prompt was never sent')
    time.sleep(5.4)
    shot('a2-working')
    for _ in range(90):
        time.sleep(0.5)
        if (panel().get('card') or {}).get('state') in ('done', 'stopped'):
            break
    time.sleep(1.0)
    shot('a3-done')
    card = lambda: panel().get('card') or {}
    until(lambda: click(*centre(card()['steps']['rect'])), lambda: card().get('steps', {}).get('open') is True)
    shot('a4-steps-open')
    until(lambda: click(*centre(card()['steps']['rect'])), lambda: card().get('steps', {}).get('open') is False)
    until(lambda: click(*centre(card()['buttonRects'][1])), lambda: card().get('state') == 'undone')
    shot('a5-undone')
    until(lambda: click(*centre(panel()['composer']['toast']['redoRect'])), lambda: card().get('state') == 'done')
    a.run('select.set', {'ids': [f['id']]})
    until(lambda: click(*centre(panel()['composer']['plus']['rect'])), lambda: panel().get('composer', {}).get('plus', {}).get('open') is True)
    a.call('window.focus', {'id': 0})
    time.sleep(0.5)
    path = os.path.join(outdir, f'{theme}-a6-plus-menu.png')
    p = panel()
    take(path)
    states['a6-plus-menu'] = {'header': p.get('headerRect'), 'menu': p.get('composer', {}).get('plus', {}).get('open')}
    print('a6-plus-menu saved', flush=True)
json.dump(states, open(os.path.join(outdir, f'{theme}-{width}-states.json'), 'w'), indent=1)
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
