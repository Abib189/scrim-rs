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
