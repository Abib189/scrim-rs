# Follow the agent, reveal its work (Stage I)

The plan, and what the code held: `docs/design/follow-agent.md`. Tests are in `crates/ui-egui/src/follow_agent_tests.rs` unless named otherwise.

| Check | How | Covered by |
|---|---|---|
| An agent's edit off screen flies the camera there (D2's flight), the zoom kept when it fits | Agent: create a frame far away | `an_agent_edit_off_screen_flies_the_camera_there` |
| Work already in view doesn't move the camera; work just out moves it the least | Agent: a shape in view, then one just past the edge | `work_already_in_view_doesnt_move_the_camera_and_work_just_out_moves_it_the_least` |
| Work that doesn't fit zooms out just enough, never in (`camera.focus-layer {fit}`) | Agent: a frame bigger than the view | `cmd/camera_tests.rs` › `fit_zooms_out_only_when_the_layers_dont_fit` |
| A burst is one smooth move: at most one new flight per 400 ms, never back and forth | Agent: 16 edits in half a second | `a_burst_is_one_smooth_move_at_most_one_flight_per_400_ms` |
| The person's edit never moves the camera, and pauses following once an agent's work has been seen (before that, nothing to pause) | Edit something yourself while an agent works | `the_persons_edit_never_moves_the_camera_and_pauses_following` |
| A scroll mid-flight cancels it and pauses; the "Follow agent" chip resumes and flies to the latest work | Scroll during a flight, then click the chip | `a_scroll_mid_flight_cancels_and_pauses_and_resume_flies_to_the_latest_work` |
| A canvas click pauses; a click on the toolbar doesn't | Click empty canvas, then a tool | `a_canvas_click_pauses_but_a_click_on_the_toolbar_doesnt` |
| An agent's own camera move (`view.set`) doesn't pause | Agent: `view.set`, then an edit | `an_agents_own_camera_move_doesnt_pause_following` |
| Work on another page shows that page first | Agent: edit page 2 while page 1 shows | `work_on_another_page_shows_that_page_first` |
| Animations off (or reduced motion): a jump cut | Settings ▸ General ▸ Animations: Off | `with_animations_off_it_jumps` |
| Nothing moves before the canvas has its real size | An agent edit as the window opens | `nothing_moves_before_the_canvas_has_its_real_size` |
| A move waits while the pointer is down (a drag in a panel), then goes | Hold a slider while an agent edits | `a_move_waits_while_the_pointer_is_down_then_goes` |
| Big batches frame the artboard; a new top-level frame is framed whole | Agent: 70 shapes in one batch | `big_batches_frame_the_artboard_and_a_new_top_level_frame_is_framed_whole` |
| The AI tab says "Following Claude · Poster"; `camera.follow-agents` answers the state (MCP route) | Watch the AI tab while Claude works | `the_ai_tab_says_who_and_what_it_follows_and_the_command_answers_the_state` |
| The switch is saved (Settings ▸ General, the AI tab, the palette); on by default | Turn it off, restart | `cmd/camera_tests.rs` › `follow_agents_turns_on_off_and_resumes` |
| An Activity row click selects and focuses its layers (D2) | Click an agent's change in the AI tab | `an_activity_row_click_selects_and_focuses_its_layers` |
| I2: an agent's 16-layer batch enters in creation order, 40 ms apart (fade and 4% scale, 220 ms), within 1.2 s | Watch an agent build a poster | `reveal_tests.rs` › `a_16_layer_batch_enters_in_order_and_the_document_never_sees_it`, `a_batch_is_staggered_in_creation_order_within_the_cap` |
| Over 60 layers, only the top-level groups enter | Agent: 70 shapes at once | `over_60_layers_only_the_roots_of_what_was_created_enter` |
| Render-only: MCP `screenshot`, an export and `inspect_document` mid-reveal equal those after; the document, its revision and undo history are untouched | Agent: screenshot mid-build | `screenshot_export_and_inspect_mid_reveal_equal_those_after`, `a_16_layer_batch_…` |
| No entrance for the person's edits, undo or redo, file open, Animations Off, or "Reveal agent work" off | Each in turn | `the_persons_edits_undo_redo_a_file_open_animations_off_and_the_switch_skip_it` |
| Paused: work in view still enters; work off screen is drawn final and never half-faded | Scroll away, let the agent work, scroll to it | `while_paused_work_in_view_reveals_and_work_off_screen_is_never_left_hidden` |
| A move tweens; a delete fades out; dropped frames finish the reveal | Agent: move, delete; hide the window mid-reveal | `a_move_tweens_a_delete_fades_and_dropped_frames_finish_it` |
| The reveal and the working indicator run in the same frames | Watch a batch | `the_reveal_and_the_working_indicator_run_in_the_same_frames` |
| Under load: static row, then no pulse, then a 0.6 s cap, in that order; the fade stays | A large file | `under_load_the_steps_come_in_order_and_the_fade_stays` |
| The dots' wave is deterministic, smooth and loops (1.6 s) | — | `working_indicator_tests.rs` › `the_wave_is_deterministic_smooth_and_loops` |
| It fades in over 160 ms, out over 400 ms after 600 ms idle; with motion off it shows still | Watch the agent stop | `it_fades_in_over_160_ms_and_out_over_400_ms_after_600_ms_idle`, `it_fades_out_once_the_agent_is_idle_and_the_dots_switch_is_its_own` |
| The row never overlaps a label (Ridgeline at 10, 22, 50, 100%), drops columns on a narrow artboard, then hides, and stays on its artboard | Zoom out while an agent works | `on_ridgeline_at_four_zooms_…`, `the_row_drops_columns_on_a_narrow_artboard_then_hides_and_never_touches_a_label` |
| The outline has no handles and no hit area; the person's selection wins | Click the outline; select the artboard | `the_outline_has_no_handles_and_no_hit_area_and_the_persons_selection_wins` |
| One polite status for screen readers: "Claude is working on Poster" | VoiceOver | `one_polite_status_names_the_agent_and_the_artboard` |
| Animations Off: the static row | Animations: Off | `with_animations_off_the_row_is_still_and_with_motion_it_moves` |
| Brightest dot at 3:1 on the canvas and both pages, both themes | — | `at_its_brightest_the_row_reads_at_3_to_1_in_both_themes` |
| The two switches are saved and answer over the route | Turn them off, restart | `the_switches_are_saved_and_answer_over_the_route` |
| I3: the switches and `fit` answer their state over MCP's `run_command`, are listed by `list_commands`, and never touch the document | MCP: run each | `crates/mcp/tests/follow_agent.rs` › `the_switches_answer_their_state_over_mcp_and_leave_the_document_alone` |
| I3: the system's reduced motion (Animations: Follow system): a jump cut, no entrance, the static row | System Settings ▸ Accessibility ▸ Reduce motion | `follow_agent_tests.rs` › `the_systems_reduced_motion_is_a_jump_no_entrance_and_the_static_row` |
| The landing's own long frame (the edit itself on a big file) delays the reveal and the indicator; a second stall, or a hidden window, finishes it; what lands while hidden is final | An agent batch on the Fixture | `reveal_tests.rs` › `a_long_landing_frame_delays_the_reveal_and_dropped_frames_finish_it`; `working_indicator_tests.rs` › `a_long_frame_after_the_agents_edit_isnt_idle_time` |
| An agent's edit after a quiet spell (an idle window draws no frames) still enters; one answered while the window is hidden is drawn final | Leave Scrim idle, then let an agent edit | `reveal_tests.rs` › `an_edit_after_a_quiet_minute_still_enters`, `a_long_landing_frame_…` |
| One slow canvas render doesn't step the degradation down (only new renders count) | — | `reveal_tests.rs` › `one_slow_render_doesnt_degrade` |
| I4 live: the poster build over the real MCP endpoint (flight, staggered reveal and dots in the same frames), the paused chip after a scroll and Resume, paused work in view, off-screen work final when scrolled to (0 px), Animations Off (0 px outside the canvas mid-reveal; finals identical after a fresh render), frame times on the Fixture | `docs/scrim-ports/follow-agent/README.md` | I4 captures (manual) |
