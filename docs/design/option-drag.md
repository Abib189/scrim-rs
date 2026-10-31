# Option-drag to duplicate (Fix AE, 31 Oct 2026)

**Before:** an Option-drag copied only if Option was already down at the press. The choice was fixed then: Option pressed mid-drag moved the original, and Option let go mid-drag still dropped a copy. Escape with Option still held matched no shortcut (Escape is bound with no modifiers), so the drag went on and the release committed the copy. The copy cursor showed only over selected layers, though unselected ones were copied too. The copy was made in the drag's own preview code, not by Duplicate, so its name ignored Rename duplicated layers. A frame dropped a short way off overlapped its original and was nested inside it. `version.autosave` didn't check for a gesture in progress, so it could save the preview.

**Now:**
1. **Option decides throughout the drag** (`crates/engine/src/tools/move_drag.rs`). While it is held, the originals stay and copies follow the pointer; pressed mid-drag, the original snaps back. Let go, the copies go and the originals move again. The release uses Option as it is at that moment (`move_up` re-previews when it changed since the last move).
2. **The release runs `edit.duplicate`** with the drop's offset (`dx`, `dy` in the original's parent), as one undo step called Duplicate. It selects the copies, and one ⌘Z removes them and selects the originals. The preview makes its copies with the same function (`cmd::duplicate::copies`), so names, components (a main component copies to a new one) and instances (copied as instances) are the same as Cmd+D's. There is no second duplicate code path.
3. **The copies are only a preview** until the release: no command runs, nothing is journaled, no undo step and no autosave (`version.autosave` now refuses during any gesture). Escape, with Option or any other modifier held, cancels a drag in progress, and the document is the one from before the press, byte for byte.
4. **Placement:** a copy goes in the same parent, directly above its original. Dropped over another frame, it goes into that frame, as a moved layer would. Its own original is never a drop target. In auto layout, it takes the slot under the pointer and the original keeps its slot. Dropped on empty canvas, a copy of an auto layout child leaves the frame. `edit.duplicate` takes `parent` and `index` for these cases, and the drag shares the placing code with it (`cmd::duplicate::place`).
5. **Unchanged:**
   - Option+Shift locks to one axis, and snapping and guides apply to the copy (it snaps to its original too).
   - Option on empty canvas makes a marquee. Option on a resize handle resizes from the centre.
   - Option with the scroll wheel, and in text editing, behave as before.
   - Locked layers aren't copied and hidden layers aren't hit.
   - The drag threshold and click-to-select are the same.
   - No MCP tool or setting was added.
6. **Cursor:** with Option held over any unlocked layer a drag would move, selected or not, the copy cursor shows; let go, or over empty canvas, it doesn't.

## Decisions (overrule them)

- **Undo selects the originals**, the selection the drag started with. Pressing an unselected layer selects it before the drag begins (as before), so that layer, not the earlier selection, is what comes back.
- **A copy is never dropped into its own original.** Option-dragging a frame a little used to nest the copy inside the frame it came from. The originals are now excluded as drop targets, as the moving layers always were.
- **Option over another layer, with something selected, shows the copy cursor.** It used to show the plain arrow there, because Option also measures from the selection to that layer. The measurement still shows. A press there selects that layer and an Option-drag copies it, so the cursor now says so (`cursor_tests`' row changed with it).
- **Escape with any modifier cancels a drag in progress**, not only with Option (Shift+Escape and Cmd+Escape too). Without a drag, Escape with modifiers does what it did.
- **`edit.duplicate` gained `parent` and `index`.** Agents can use them through `run_command`. It is not a new tool.
- **The autosave guard covers every gesture**: resizing, rotating and drawing too, not only Option-drags.

## Tests

- `crates/engine/src/option_drag_tests.rs` (18 tests). Seven fail on the old code:
  - Option pressed mid-drag;
  - Option let go mid-drag;
  - both orders, and Option let go just before the release;
  - a frame dropped over its own original;
  - component naming as Cmd+D;
  - the copy cursor over an unselected layer;
  - the command log and autosave check.

  The other eleven already held and guard the behaviour:
  - copy count, position, selection and parent;
  - a plain move;
  - undo and redo;
  - the engine-level Escape;
  - a multiple selection;
  - the auto layout slot;
  - reparenting;
  - Option+Shift;
  - snapping;
  - locked layers;
  - the marquee and resize-from-centre.
- `crates/ui-egui/src/option_drag_ui_tests.rs` (3 tests, through the window), all three failing on the old code:
  - Escape with Option held cancels;
  - Option pressed and let go mid-drag switch on the next frame without the pointer moving;
  - the copy cursor follows Option over an unselected layer.

## Live check (31 Oct 2026)

A real window: the debug app with a scratch `HOME` and `SCRIM_SETTINGS_DIR`, MCP on 7690 and control on 7979. Pointer and modifier input went through the window's own input (`ui.input` with `alt`/`shift` and `hold`); the system pointer was never moved. The script, its output and the captures (before, mid-drag, after release and after undo for each case) are in `option-drag/` (`live.py`, `live-output.txt`, `captures.png`).
1. **A frame with children** (Card › Inner › Dot, and Tag), Option-dragged 330 to the right:
   - mid-drag, the copy is at 430 and the original stays at 100;
   - the release leaves the same, with one Duplicate step;
   - Undo removes the copy and selects Card again.
2. **Auto layout:** A, Option-dragged past C. Its copy takes the fourth slot (x 220) and A stays first. Undo leaves A, B, C.
3. **Option+Shift:** Chip dragged by (240, 20). The copy lands at (800, 420), on the horizontal only. Undo removes it.
4. **Option pressed mid-drag**, the pointer still:
   - the original snaps back to (560, 420) and a copy is at the pointer;
   - Escape with Option held cancels, with the document and history identical to before the press.

Undo was run as Edit ▸ Undo (`ui.menu.invoke edit.undo`), the menu item ⌘Z runs. The control channel's `cmd` flag also sets `ctrl` (`control.rs`, `ui.input`), so its ⌘Z reaches the keymap as Cmd+Ctrl+Z and matches nothing. The same is why ⌘W couldn't be sent in Fix AD. Not changed here.
