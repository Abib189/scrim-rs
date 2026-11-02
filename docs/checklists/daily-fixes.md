# Daily-annoyance and performance fixes from upstream (Fix AI stage 2c)

`docs/design/upstream-sync.md` › Ported from upstream (stage 2c).

| Check | How | Covered by |
|---|---|---|
| Alone in a live session, moving the pointer, you stay connected | Join a session where nobody else is, keep moving the pointer for over a minute: no "Reconnecting" | `crates/collab-net/src/driver_tests.rs` › `a_lone_participant_moving_the_pointer_stays_connected` |
| A host that stops answering is still noticed | Stop the host (kill it) while you move the pointer: after 45 s, "Reconnecting" | `driver_tests.rs` › `a_dead_connection_is_still_dropped_while_the_pointer_moves` |
| Recovery copies don't need the window to draw | Edit a file, hide or cover the window (or minimise it), wait past 10 s: a recovery copy with the edit exists in the settings folder's `Recovery/<pid>/`; the window asks for no frames meanwhile | `apps/figcraft/src/recovery_tests.rs` › `an_edit_is_copied_after_the_interval_with_no_frame_after_it`; `apps/figcraft/src/recovery/writer_tests.rs` (the schedule on a fake clock); `recovery_tests.rs` › `copies_follow_saves_and_a_normal_quit_leaves_nothing` (no frame asked while a copy waits) |
