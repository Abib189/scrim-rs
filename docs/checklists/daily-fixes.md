# Daily-annoyance and performance fixes from upstream (Fix AI stage 2c)

`docs/design/upstream-sync.md` › Ported from upstream (stage 2c).

| Check | How | Covered by |
|---|---|---|
| Alone in a live session, moving the pointer, you stay connected | Join a session where nobody else is, keep moving the pointer for over a minute: no "Reconnecting" | `crates/collab-net/src/driver_tests.rs` › `a_lone_participant_moving_the_pointer_stays_connected` |
| A host that stops answering is still noticed | Stop the host (kill it) while you move the pointer: after 45 s, "Reconnecting" | `driver_tests.rs` › `a_dead_connection_is_still_dropped_while_the_pointer_moves` |
