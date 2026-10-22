"""s6report.py OUT/s6.json — Fix T6 part A's figures: the dots in every 100 ms capture between the first
batch and finish_working, the working events, each batch's centre against the canvas's centre once
the camera has settled, the zoom, and what entered."""
import json, math, sys

d = json.load(open(sys.argv[1]))
steps, samples = d['steps'], d['samples']
first = steps[0]['t']
finish = next(s['t'] for s in steps if s.get('finish'))
during = [s for s in samples if first + 0.5 <= s['t'] <= finish]
on = [s for s in during if s['working'] and (s['alpha'] or 0) >= 0.99]
drawn = [s for s in during if s['dots']]
print(f'captures between the first batch and finish_working: {len(during)}')
print(f'  working state on (dots at full opacity): {len(on)}/{len(during)}')
print(f'  dots drawn on the canvas: {len(drawn)}/{len(during)}')
off = [s['t'] for s in during if not s['dots']]
if off:
    print(f'  not drawn at: {off[:20]}{" …" if len(off) > 20 else ""}')
after = [s for s in samples if s['t'] > finish + 1.0]
print(f'  after finish_working (+1 s): working {sum(1 for s in after if s["working"])}/{len(after)}')
print('batches (settled 3 s after each):')
for st in [s for s in steps if 'batch' in s]:
    at = next((s for s in samples if s['t'] >= st['t'] + 3.0), None)
    if not at:
        continue
    cx, cy, z, vw, vh = at['view']
    # What it just changed: the layers in the artboard (they landed after it), or the artboard and
    # them together (they landed in the same pass): their union is the artboard.
    layers = math.hypot(cx - st['centre'][0], cy - st['centre'][1]) * z
    board = math.hypot(cx - (st['x'] + 320), cy - 240) * z
    which, off = ('its layers (x+300, 150)', layers) if layers <= board else ('the artboard with its layers (x+320, 240)', board)
    print(f"  batch {st['batch']} at x {st['x']}: view ({cx:.1f}, {cy:.1f}) zoom {z:.4f} canvas {vw:.0f}×{vh:.0f}: {off:.2f} px from the centre of {which}, follow {at['follow']}")
print('reveal tallies (newest last):')
for t in (d.get('reveals') or {}).get('recent', []):
    print(f"  created {t['created']} entered {t['entered']} skipped {t['skipped']} follow {t['follow']}")
print('diagnostics:')
for e in d['diagnostics']:
    if e.get('kind') in ('working', 'reveal', 'follow'):
        print(f"  {e['kind']}: {e['detail']}")
