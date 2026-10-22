# Fix T6 part A: the grace, centring and the reveal in the real app (22 Oct 2026)

`s6.py BINARY SCRATCH OUT` runs the debug app (at `59a45d2`) on a scratch HOME and a scratch settings folder, with MCP on 7690 and the control channel on 7979.
- **The agent:** an MCP client on the app's HTTP endpoint, named `local-agent-mode-scrim (via mcp-remote 0.14.3)`. It builds four batches of 18 layers (an artboard, then 17 layers in it in one `batch`), each 3,000 units from the last, 20, 36 and 50 s apart, then calls `finish_working`.
- **The sampler** reads `ui.inspect` and `view.get` every 100 ms.
- **Settings ▸ Diagnostics** is read from the scratch settings file after the app quits.
- **The figures:** `s6report.py s6.json` prints them, kept in `s6-report.txt`.

**The screen was locked for this run** (`CGSSessionScreenIsLocked`), so the app counted its window as hidden.

| Check | Result |
|---|---|
| The dots, between the first batch and `finish_working` | working state on at full opacity in **953 of 953** captures |
| The dots drawn on the canvas | 950 of 953. The 3 misses are 20.3, 56.3 and 106.7 s, each just after a batch landed: the flight between artboards 3,000 units apart, while neither has room for the row on screen |
| After `finish_working` (+1 s) | not working in 9 of 9 |
| Diagnostics: the working state | exactly one `begin` (first request) and one `end` (`finish_working`); **no grace expiry**, and no "grace too short" at the 60 s default |
| Batch 0–2, settled 3 s later | **0.00 px** between the canvas's centre and the centre of the layers just changed |
| Batch 3 | **0.00 px** from the centre of the artboard with its layers. They landed in the same pass, so their union is the artboard |
| The zoom | 0.853, 0.833, 0.831, 0.830. At each new artboard, its 640 units fit with 10% on each side of the canvas (0.8 × canvas width ÷ 640); never raised. The canvas narrowed a few pixels during the run |
| The reveal | not exercised: every new layer was drawn final at once, "landed while the window was hidden", as before for a hidden window |

**Not verified on a real window:**
- the reveal waiting for following's flight (T3), which the tests cover (`reveal_wait_tests.rs`);
- the dots' look on screen.
The run should be repeated with the screen unlocked.
