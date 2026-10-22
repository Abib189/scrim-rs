# Fix U2: the T6 live run again, with the dots held at the canvas's edge (22 Oct 2026)

The same script as Fix T6 part A (`../t6/s6.py`, `../t6/s6report.py`, unchanged), run on the debug app built from the U2 change, on a scratch HOME and a scratch settings folder, with MCP on 7690 and the control channel on 7979. Four batches of 18 layers, each 3,000 units from the last, 20, 36 and 50 s apart, then `finish_working`; `ui.inspect` and `view.get` sampled every 100 ms. The figures are in `s6-report.txt`, from `s6.json`.

**The screen was unlocked this time**, so the window counted as visible.

| Check | T6 (before U2) | U2 |
|---|---|---|
| The dots drawn, between the first batch and `finish_working` | 950 of 953 | **804 of 804** |
| Captures mid-flight (the camera between two artboards' centres) | 3 without dots | **17, all with dots**, among them 4010, 5609, 8606 and 11424, where neither artboard is on screen |
| The working state | one `begin`, one `end` (`finish_working`), no grace expiry | the same |
| Each batch, settled 3 s later | 0.00 px from the centre | 0.00 px from the centre of its layers, all four |
| The zoom | 0.853 → 0.830 | 0.853, 0.833, 0.831, 0.830 |
| The reveal | not exercised (hidden window) | every layer entered; 14 captures saw entrances held for the flight (T3) |

The sampler took fewer captures than in T6 (804 against 953) because the visible window draws every frame. The dots' look on screen was not compared with a screenshot.
