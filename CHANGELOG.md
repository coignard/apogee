# Changelog

## 0.5.3

### Changed

- Bumped bitflags from 2.13.0 to 2.13.1.
- Bumped bytemuck from 1.25.1 to 1.25.2.
- Bumped cc from 1.2.67 to 1.4.0.
- Bumped cfg_aliases from 0.2.1 to 0.2.2.
- Bumped clap from 4.6.2 to 4.6.4.
- Bumped clap_derive from 4.6.1 to 4.6.4.
- Bumped coremidi-sys from 3.2.0 to 3.2.1.
- Bumped either from 1.16.0 to 1.17.0.
- Bumped foreign-types-macros from 0.2.3 to 0.2.4.
- Bumped futures-core from 0.3.32 to 0.3.33.
- Bumped futures-task from 0.3.32 to 0.3.33.
- Bumped futures-util from 0.3.32 to 0.3.33.
- Bumped glob from 0.3.3 to 0.3.4.
- Bumped jni-min-helper from 0.3.2 to 0.3.3.
- Bumped libc from 0.2.186 to 0.2.189.
- Bumped portable-atomic from 1.13.1 to 1.14.0.
- Bumped proc-macro2 from 1.0.106 to 1.0.107.
- Bumped quick-xml from 0.39.4 to 0.41.0.
- Bumped quote from 1.0.46 to 1.0.47.
- Bumped serde_json from 1.0.150 to 1.0.151.
- Bumped simd-adler32 from 0.3.9 to 0.3.10.
- Bumped simd_cesu8 from 1.1.1 to 1.2.0.
- Bumped thiserror from 2.0.18 to 2.0.19.
- Bumped thiserror-impl from 2.0.18 to 2.0.19.
- Bumped toml_edit from 0.25.12+spec-1.1.0 to 0.25.13+spec-1.1.0.
- Bumped toml_parser from 1.1.2+spec-1.1.0 to 1.1.3+spec-1.1.0.
- Bumped wayland-backend from 0.3.15 to 0.3.16.
- Bumped wayland-client from 0.31.14 to 0.31.15.
- Bumped wayland-scanner from 0.31.10 to 0.31.11.
- Bumped winnow from 1.0.3 to 1.0.4.
- Bumped zerocopy from 0.8.54 to 0.8.55.
- Bumped zerocopy-derive from 0.8.54 to 0.8.55.
- Bumped zmij from 1.0.21 to 1.0.23.

## 0.5.2

### Changed

- Bumped bytemuck from 1.25.0 to 1.25.1.
- Bumped bytemuck_derive from 1.10.2 to 1.11.0.
- Bumped cc from 1.2.66 to 1.2.67.
- Bumped zerocopy from 0.8.53 to 0.8.54.
- Bumped zerocopy-derive from 0.8.53 to 0.8.54.

## 0.5.1

### Changed
- Bumped `bytes` to 1.12.1
- Bumped `cc` to 1.2.66
- Bumped `crossbeam-channel` to 0.5.16
- Bumped `crossbeam-deque` to 0.8.7
- Bumped `crossbeam-epoch` to 0.9.20
- Bumped `crossbeam-utils` to 0.8.22
- Bumped `jobserver` to 0.1.35
- Bumped `memchr` to 2.8.3
- Bumped `pxfm` to 0.1.30
- Bumped `rustversion` to 1.0.23
- Bumped `zerocopy` to 0.8.53
- Bumped `zerocopy-derive` to 0.8.53

## 0.5.0

### Added

- `--nostalgie[=<preset>]` enables a CRT display effect ported from cool-retro-term, rendered as a wgpu post-processing pass over the framebuffer.
- Fullscreen toggle with `F11` or `Alt`+`Enter`.

## 0.4.0

On macOS the MIDI output now uses CoreMIDI directly with host-timestamped, driver-scheduled delivery instead of midir's immediate send. Each message is handed to the driver stamped with the exact host time derived from the emulated CPU cycle on which the music program emitted it, so notes land on their cycle-accurate beat without the scheduler wake-up jitter of a busy-wait followed by an immediate send. The note-to-note intervals computed by the assembly program are reproduced exactly at the output. Every other platform continues to use midir's immediate-send API and is unchanged.

### Added

- `MidiConn` MIDI-output abstraction with two `cfg`-gated backends: a newtype over `midir::MidiOutputConnection` off macOS, and a `coremidi::OutputPort` + `coremidi::Destination` pair on macOS exposing `send_now()` and a timestamped `send_at()`.
- macOS host-clock helpers `now_host()` / `nanos_to_host()` bound to `AudioGetCurrentHostTime` / `AudioConvertNanosToHostTime` from the CoreAudio framework.
- macOS `run_midi_thread` that anchors the emulated cycle counter to a CoreMIDI host-time grid `(anchor_host, anchor_cycle)` and schedules every message at `anchor_host + delta_cycles / cpu_freq`, letting the driver deliver it on time; it re-anchors and calls `coremidi::flush()` on stalls, pauses, and hard resets.

### Changed

- MIDI backend is now platform-specific in `Cargo.toml`: `coremidi 0.9.1` for `cfg(target_os = "macos")`, `midir 0.11.0` for `cfg(not(target_os = "macos"))`.
- `AppConfig::midi_out` is now `Option<MidiConn>` instead of `Option<midir::MidiOutputConnection>`.
- The inline MIDI thread closure is extracted into a `cfg`-gated `run_midi_thread`; the non-macOS path keeps the existing spin-sleep-then-send timing unchanged.
- `silence_active_notes()` now takes `&mut MidiConn` and sends through `send_now()`.
- MIDI device discovery and `--midi` resolution are factored into `cfg`-gated `list_midi_outputs()` / `open_midi_output()`; on macOS they enumerate `coremidi::Destinations` and select by name or index.

## 0.3.3

### Changes
- Bumped cpal from 0.17.3 to 0.18.1

## 0.3.2

### Added

- `KeyboardTranslator` struct replacing the stateless `map_keycode()` function; tracks active physical keys and synthesises `Shift`, `Ctrl`, and `Lang` modifier events that bracket each base key press, correctly separating host modifier state from the emulated modifier state
- `KeyboardLayout` enum with three variants selectable via `--keyboard-layout`: - `Smart` (default): maps OS logical characters directly to emulator keys, deriving `Shift` and `Lang` from the character value; supports transparent Latin and Cyrillic input regardless of the host keyboard layout setting
- `Qwerty`: maps physical QWERTY scan codes to emulator keys; useful when the host layout is already set to match the target machine
- `Jcuken`: maps physical QWERTY scan codes to JCUKEN positions for a fixed hardware-style Russian layout
- `keyboard_layout` field added to `AppConfig`

### Changed

- `Key::End` renamed to `Key::Clear`; `Key::PageDown` renamed to `Key::LineFeed`; `Key::Equal` renamed to `Key::Colon`; `Key::Backquote` renamed to `Key::At`; `Key::Quote` renamed to `Key::Caret`; `Key::Alt` renamed to `Key::Lang`, all names now match the physical key labels on the Apogee BK-01 keyboard
- `--crt` flag renamed to `--gigascreen`; `is_crt` / `is_crt_blend` fields renamed to `gigascreen` in `VideoRenderer`, `ReplayMetadata`, and `AppConfig`; the feature blends consecutive frames to simulate the gigascreen technique
- MIDI strobe detection changed from rising-edge on bit 0 (`STROBE_BIT`) to active-low OBF on bit 7 (`OBF_BIT`), matching the `KR580VV55A` handshake signal; `last_port_c` field removed from `MidiInterface`; `MidiInterface::update()` now returns `bool`; `Bus` pulses port C `!0x40` then `0xFF` when `update()` returns `true` to complete the acknowledge cycle
- `UserPeripheral::update()` now returns `bool` propagated from the active peripheral; `RomDisk` returns `false`
- Repeat key events are now filtered before reaching `KeyboardTranslator`, preventing duplicate emulator key events on held keys

### Fixed

- KR580VI80 (iz80): XCHG instruction cycle count was incorrect upstream; corrected in iz80 0.5.1, test snapshots regenerated accordingly

## 0.3.1

### Added

- `FontBanks` struct on `Bus` grouping `rows: [bool; 64]` and `previous_row: usize`; derives `Serialize` / `Deserialize` with `BigArray` support, replacing the two bare fields
- `lsb_buffer: u8` field on `TimerChannel` staging the first byte of a two-byte counter write; `reload_value` assembled atomically in `trigger_load()` only after the MSB arrives, matching the physical data bus buffer described in the i8253 datasheet

### Changed

- `Bus::font_banks` field type changed from two bare fields to `FontBanks`; `Machine::font_banks()` updated to return `&self.bus.font_banks.rows`
- `SPECIAL_CODE_MASK` widened from `0xFC` to `0xF0`: the KR580VG75 decoder circuit checks only the upper nibble to classify a byte as a special code and reads control flags directly from bits 0-1; bits 2-3 are don't-care on real chip despite the datasheet reserving them, so `0xFF` is now correctly recognised as End of Screen + Stop DMA

### Fixed

- KR580VG75: bytes with bits 2-3 set (e.g. `0xFF`) now accepted as valid special codes, matching the behaviour of the physical decoder exploited by the majority of Apogee BK-01 software that uses `0xFF` as a universal end-of-screen sentinel
- KR580VI53: two-byte counter writes no longer corrupt the running period via a spurious intermediate reload; square-wave channels now complete the current half-period before adopting the new reload constant, eliminating the phase inversion and duty-cycle distortion that caused audible pitch drift on every note change
- KR580VI53 Mode 0: channel now transitions to `WaitLoad` on LSB receipt, halting the counter until the MSB arrives per datasheet Counter Loading section

## 0.3.0

Version 0.3.0 is a complete rewrite of all core chip emulations. The timing model is now different from emu80-based emulators and from all prior versions of this project.

The VG75 interrupt request now fires at the beginning of the last display row as the i8275 datasheet specifies; previously it fired after that row, at the start of vertical retrace. The display pipeline now implements the hardware dual row buffer: DMA-fetched data fills a back buffer while the front buffer is rendered, so the displayed image lags the memory read by exactly one character row as on real hardware. DMA burst spacing is now counted strictly in character clocks (CCLK) with authentic HRQ/HLDA bus arbitration, replacing a raw tick countdown that drifted out of phase with the video signal.

### Added

- `display_row_buffer` and `fill_row_buffer` double-buffering in `KR580VG75`: fill buffer receives DMA bytes while display buffer is rendered; buffers swapped atomically in `begin_row()`
- `display_fifo` and `fill_fifo` replacing the single `fifo` field; `fill_fifo_pos` overflow detection and `display_fifo_pos` tracking fill position across double-buffer swap so FIFO reads start at the correct offset on overflow
- `CharAttrBehavior` struct and `CHAR_ATTR_DEFS` table extended to 16 entries covering the full attribute index space, replacing the parallel `CHAR_ATTR_VSP`/`CHAR_ATTR_LTEN` arrays
- `STATUS_LIGHT_PEN` constant; `trigger_light_pen()` storing `crt_x`/`crt_scan_row`; `lpen_x`/`lpen_y` fields; `ReadLpen` now returns actual captured coordinates
- `drq()` and `current_row()` on `KR580VG75` for bus-level tracking replacing the removed `set_inte()` / `row_font_bank()` interface
- `DmaChannel` struct holding address and count per channel; `DmaOperation` enum derived from count register upper bits, in `KR580VT57`
- `drq[]`, `hlda`, and `last_serviced_channel` fields on `KR580VT57` for proper HRQ/HLDA handshake
- `hrq()`, `set_drq()`, `set_hlda()`, `dma_transfer_cycle()` on `KR580VT57`; `dma_transfer_cycle()` selects next channel with rotating or fixed priority
- `hardware_reset()` on `KR580VT57` reinitializing transient state without clearing channel config
- `read()` on `KR580VT57` exposing address/count registers and clearable TC status byte
- `MODE_TC_STOP` disabling channel on terminal count; `MODE_AUTO_LOAD` copying ch3 into ch2 on ch2 terminal count via `STATUS_UPDATE_FLAG`; `MODE_ROTATING_PRIORITY` cycling service order across channels
- `BytePhase::reset()` and `apply_to_u16()` on `KR580VT57` eliminating repeated match/shift boilerplate
- `Mode`, `Direction` enums in `KR580VV55A` replacing CWR bit-flag checks
- `GroupConfig` and `Config` structs in `KR580VV55A` encapsulating group A/B configuration
- `output_latch_a/b/c`, `input_latch_a/b`, and `pin_state_a/b/c` fields on `KR580VV55A` separating CPU and peripheral data paths
- `port_c_base_value()` composing input pins and output latch per direction config; `cpu_read_port_c_status_word()` exposing IBF/OBF/INTR/INTE flags; `peripheral_write_c()` with falling-edge detection for STB/ACK signals; `update_interrupts()` deriving INTR_A and INTR_B from INTE, IBF/OBF, and strobe state
- `hardware_reset()` on `KR580VV55A` initializing all pins high and applying default all-input Mode 0 control word
- `font_banks: [bool; 64]` and `previous_row: usize` fields on `Bus` for per-row INTE tracking
- `font_banks()` accessor on `Machine` returning `&[bool; 64]`
- `OPEN_BUS_VALUE`, `STATUS_VALID_BITS_MASK`, `STATUS_LIGHT_PEN`, `SPECIAL_CODE_EOF_BIT`, `SPECIAL_CODE_STOP_DMA_BIT`, `MAX_LINES_PER_ROW`, `UNDERLINE_MSB_THRESHOLD` constants replacing magic numbers
- `COMMAND_SHIFT`, `RESET_VR_ROWS_SHIFT`, `RESET_UNDERLINE_LINE_SHIFT`, `CHAR_MSB_MASK`, `CHAR_CODE_MASK`, `BURST_COUNT_MASK`, `BURST_SPACE_SHIFT`, `BURST_SPACE_MASK`, `BURST_SPACE_MULTIPLIER`, `CURSOR_X_MASK`, `CURSOR_Y_MASK` constants replacing inline magic numbers in VG75 parameter decode
- `UPDATE_SCREENSHOTS` environment variable for regenerating PNG snapshots in-place; saves `_before`/`_after` side-by-side images when an existing snapshot changes
- `Deserialize` derived on all `KR580VT57` and `KR580VV55A` types for snapshot restore and deterministic replay

### Changed

- `KR580VG75` fully rewritten with cycle-accurate timing; `begin_row()` replaces `next_row()`/`prepare_next_frame()`/`next_frame()`; frame counter and field attribute reset now driven by `crt_scan_row` wrap; parsed symbols indexed by `crt_scan_row` instead of `crt_cur_row`
- IRQ raised at `n_rows - 1` (beginning of last display row) instead of at `n_rows` (start of vertical retrace); `vblank` returns `true` at `n_rows`
- DMA burst spacing counted strictly in CCLK ticks via `cclk_wait_timer` in `tick_char()` instead of CPU-cycle offsets; DMA state tracked through `DmaState` enum (`Idle`, `Requesting`, `WaitingSpace`)
- DMA handshake in `machine.rs` rewritten around `hrq()`/`set_hlda()`/`dma_transfer_cycle()`; when HRQ is asserted the CPU does not execute, channel 2 reads one byte from RAM and passes it to `vg75.dack()`, and `elapsed_cycles` is fixed at 4; `halt_cycles` model removed
- `vg75.tick()` no longer accepts `vt57` or `ram` arguments; DRQ signal fed from `vg75.drq()` into `vt57.set_drq(2)` in the machine loop
- Font bank tracking moved from `KR580VG75` into the machine loop; `font_banks[r]` set to current INTE state for each row completed since the previous tick; `set_inte()`, `finalize_font_banks()`, `row_font_bank()`, `cpu_inte`, `cur_font_bank`, `prev_row`, and `row_font_banks` removed from `KR580VG75`
- `render_frame()` accepts `font_banks: &[bool; 64]` directly instead of calling `row_font_bank()` on the VG75; all three call sites in `app/mod.rs` updated
- `SPECIAL_CODE_MASK`/`VAL` corrected from `0xF1`/`0xF0` to `0xFC`/`0xF0`; stop-DMA bit and EOF bit split into `SPECIAL_CODE_STOP_DMA_BIT` and `SPECIAL_CODE_EOF_BIT`
- `CMD_PRESET_COUNTERS` now initializes `crt_scan_row` to the last scan row instead of 0; `reset_field_attributes()` extracted
- `start_raster_if_not_started` sets `crt_scan_row` to the last row so `begin_row()` fires on the first CCLK tick rather than skipping the initial fill cycle
- `CMD_RESET` now clears `STATUS_INT_REQUEST` alongside `INT_ENABLE` and `VIDEO_ENABLE`; redundant `STATUS_VIDEO_ENABLE` clear removed from `CMD_PRESET_COUNTERS`
- `STATUS_IMPROPER_CMD` set on write with a pending command instead of eagerly on each `CMD_RESET`/`LoadCursor`/`ReadCursor`/`ReadLpen` dispatch
- Blanking restructured in `render_current_row()`: `is_forced_blank` and `is_effectively_blanked` introduced; special codes detected before character type dispatch; field attribute update skipped when blanked; `und_line > UNDERLINE_MSB_THRESHOLD` VSP override moved outside character type branches to apply unconditionally after all symbol rendering
- `BLINK_FAST_DIVISOR_MASK`/`BLINK_SLOW_DIVISOR_MASK` renamed to `BLINK_DIV_16_MASK`/`BLINK_DIV_32_MASK`; `ATTR_TRANSPARENT_MASK`/`VAL` replaced with `FIELD_ATTRIBUTE_MASK`/`VAL`; `ATTR_PSEUDOGRAPHIC_MASK`/`VAL`/`EXCLUSION` replaced with `CHAR_ATTRIBUTE_MASK`/`VAL`/`EXCLUSION`
- `n_chars` clamped to `MAX_CHARS` on reset parameter decode
- `KR580VT57` fully rewritten; `enabled: bool` replaced with per-channel mode bitmask and `is_enabled(channel)`; `ch2_addr`/`ch2_count`/`ch3_addr`/`ch3_count` flat fields replaced with `DmaChannel` array
- `KR580VV55A` fully rewritten; `read()`/`write()` split into `cpu_read()`/`cpu_write()` and `peripheral_read_{a,b,c}()`/`peripheral_write_{a,b,c}()`; BSR now correctly updates `INTE_A_IN`, `INTE_A_OUT`, and `INTE_B` alongside `output_latch_c`
- `tape_out` read via `sys_vv55.peripheral_read_c() & 0x01` directly; `is_tape_out_active()` and `TAPE_OUT_BIT_MASK` removed from `KR580VV55A`
- `Bus` adapted to the new `KR580VV55A` cpu/peripheral read-write split
- `Kr580Vt57` and `serde_big_array` imports dropped from `KR580VG75`; `serde_big_array` import added to `bus.rs` for `font_banks`
- Test snapshots regenerated following `KR580VG75` and `KR580VT57` refactor

### Fixed

- Data port read outside `ReadLpen` context now returns `OPEN_BUS_VALUE` (`0xFF`) instead of stale register state; spurious `Reset` and `LoadCursor` read-back cases removed
- Status port output masked with `STATUS_VALID_BITS_MASK` consistently across all read paths
- FIFO read position on overflow: `display_fifo_pos` now tracks `fill_fifo_pos` across the double-buffer swap, so `render_current_row()` starts reading from the correct offset
- Cursor underline: `set_lten(true)` called directly instead of passing computed `blink_state`, matching the block-cursor blink guard above it
- Spaced row blanking added to `render_current_row()`; DMA request skipped for spaced rows in `begin_row()`

## 0.2.5

### Added

- `TimerMode` enum replacing raw `u8` mode field: `InterruptOnTerminalCount`, `HardwareRetriggerableOneShot`, `RateGenerator`, `SquareWave`, `SoftwareTriggeredStrobe`, `HardwareTriggeredStrobe`
- `ChannelState` enum formalising the channel lifecycle: `Unprogrammed`, `WaitLoad`, `WaitTrigger`, `Counting`; counter decrements only in `Counting` state, preventing premature ticks before the reload constant is fully written
- `LatchState` enum isolating the latch command into `LsbOnly(u8)`, `MsbOnly(u8)`, and `LsbThenMsb(u8, u8)` variants; repeated latch commands ignored while latch is pending, matching datasheet behaviour; reading `LsbThenMsb` transitions to `MsbOnly` without disturbing `RwPhase`
- `RwPhase::toggle()` helper replacing manual reassignment at both read and write sites
- `set_gate()` on `TimerChannel` with explicit rising and falling edge detection; rising edge arms hardware-triggered modes (1, 5) and reloads rate generators (2, 3); falling edge forces `out_pin` high in modes 2 and 3
- `decrement_binary_value` and `decrement_bcd_value` free functions with correct wrap-around through zero; BCD correction applied per-tetrad via `BCD_CORRECTIONS` table; Mode 3 BCD decrements run in a loop to preserve inter-tetrad carry
- `strobe_fired` guard on `TimerChannel`: Mode 4 and 5 strobe pulse lasts exactly one system tick; further counter wrap does not re-assert the output
- `Deserialize` derived on all `Kr580Vi53` types to support snapshot restore and deterministic replay

### Changed

- Mode 3 square wave rewritten: counter decrements by 2 per tick; odd reload values produce asymmetric half-periods with the high phase one tick longer than the low phase; special-cased constants for reload values 1 and 3 removed
- Mode 0 two-byte write now forces `ChannelState::WaitLoad` on LSB receipt, blocking counting until MSB arrives
- Gate level sensitivity check unified into `TimerMode::is_gate_level_sensitive()`; tick returns early for all level-sensitive modes while gate is low
- `reload_pending` path sets `working_counter` from `effective_reload_value()` and resets `strobe_fired` before returning; `out_pin` initialised per mode on every reload
- Replay metadata fields `autorun`, `color_mode`, `is_crt`, and `sample_rate` now override CLI arguments when running in playback mode, guaranteeing deterministic audio timing and display state
- RKA validation, SHA-256 computation, and `rom_name` extraction consolidated into a single destructuring block; redundant intermediate variables removed
- MIDI initialisation refactored into a single expression returning `Option`; non-Unix fallback now explicitly returns `None` via `cfg(not(unix))` branch
- Test snapshots regenerated following `Kr580Vi53` refactor; channel fields renamed and mode values changed from integers to enum variant names

## 0.2.4

### Added

- `MachineConfig` struct encapsulating machine construction parameters (`system_rom`, `sample_rate`, `rka`, `romdisk`, `midi_enabled`, `rom_name`) with `new_machine()` factory method; `App::new()` now accepts `MachineConfig` instead of a pre-built `Machine`, deferring construction to the emulation thread
- Hard reset via F7: machine reinstantiated via `MachineConfig::new_machine()`, recorder and player cleared, MIDI ring buffer flushed via `MidiThreadMsg::HardReset` with active notes silenced
- Fast-forward via F9 hold: holding F9 for 500 ms outside pause activates fast-forward mode: audio output suppressed, every fifth vblank rendered, emulation runs uncapped; releasing F9 deactivates fast-forward and resets the MIDI sync anchor
- `MidiThreadMsg` enum replacing the raw `(Vec<u8>, u64)` tuple channel; variants `Event`, `HardReset`, and `SetFastForward` allow the MIDI thread to respond to reset and speed-change signals while blocked on timing
- `SetFastForward` handling in the MIDI thread: in fast-forward mode events are dispatched immediately without cycle-accurate delay; anchor is cleared on transition back to normal speed
- `ReplayRecorder::is_empty()` predicate; replay is no longer auto-saved on exit and saved explicitly via F10 or on `Quit` only when the recorder is non-empty

### Changed

- MIDI thread sleep replaced with `recv_deadline`-based loop: `HardReset` and `SetFastForward` messages are acted upon immediately even while the thread is waiting for an event's target timestamp, eliminating stuck notes and preventing stale-anchor accumulation across resets
- `DumpSnapshot` and `SaveReplay` commands no longer carry `rom_name`; the name is captured from `MachineConfig` inside the emulation closure
- RKA header interpretation simplified: the second 16-bit word is now always treated as an inclusive end address; the dual-interpretation heuristic comparing dump-size vs end-address readings is removed; files where `end_addr < start_addr` are rejected with `InvalidRkaLength`
- `emu_cycle` gains a `fast_forward: bool` parameter; audio samples are skipped entirely and the shared `midi_encode_buf` is replaced with a per-call allocation, removing the mutable buffer argument from the signature
- Test snapshots regenerated following the iz80 update; corrected ORA cycle counts and shift instruction timing, invalidating all previously recorded snapshots

## 0.2.3

### Changed

- RKA checksum validation simplified to exclude the last byte from the checksum accumulation, matching authentic Monitor behaviour; the previous dual-variant check (including/excluding last byte) is removed
- Size heuristic in `validate_rka` switched from `saturating_sub`-based comparison to `abs_diff`, making the intent explicit
- Checksum byte extraction uses `checked_sub` + `get` instead of a manual length guard, eliminating the redundant fallback branch
- `ChecksumMismatch` error now always reports the single canonical expected value; test assertions updated to match exact expected/got fields

## 0.2.2

### Fixed

- MIDI thread now named `"midi"` and joined on exit, preventing detached thread teardown
- MIDI channel switched from `bounded(4096)` to `unbounded` to prevent event drops under burst load
- MIDI output now sends individual Note Off messages for all active notes before All Notes Off and All Sound Off on teardown, eliminating stuck notes when the connection closes mid-playback
- Stale events are now drained and tracked before the anchor reset on lag recovery, preventing missed note-offs from discarded messages
- MIDI sync anchor now initialised with a 1.5-frame presentation delay offset, eliminating chunking jitter on the first burst of events
- `try_send` replaced with `send` in `emu_cycle` for MIDI messages, preventing silent event drops when the receiver is momentarily slow
- `WINDOW_SCALE`, `AUDIO_LATENCY_FRAMES_NUMER/DENOM`, and `MIDI_SYNC_LAG_FRAMES` extracted as named constants, replacing inline magic numbers

## 0.2.1

### Changed

- Emulation loop moved from the winit `about_to_wait` callback to a dedicated `emulation` thread; the event loop now only receives rendered frames and dispatches input, eliminating hangs when the window loses focus on macOS
- Key events, pause/step commands, and snapshot requests sent to the emulation thread via an unbounded `crossbeam` command channel; rendered frames returned via a bounded(2) frame channel
- `App` no longer owns `Machine` or `VideoRenderer`; both are moved into the emulation thread at construction time
- `App::cycle()` and `App::process_midi_events()` replaced by free functions `emu_cycle()` and `send_frame()` running on the emulation thread
- `App::dump_snapshot()` replaced by free function `dump_snapshot()` invoked from the emulation thread via `EmulationCommand::DumpSnapshot`
- Replay recording timestamps captured on the emulation thread at the point of key application, replacing the previous winit-side timestamping
- Emulation thread exits cleanly when the command channel disconnects or a `Quit` command is received; joined in the `exiting` handler

## 0.2.0

### Added

- MIDI output via `--midi [port]` flag: routes bytes written to user PPI port A through a timing-accurate output thread; port may be specified by name or zero-based index
- `--midi-list` flag to enumerate available MIDI output ports
- `MidiInterface` peripheral in `core::peripherals::midi`: captures port A bytes on rising edge of strobe bit (port C bit 0) into a timestamped ring buffer of up to 256 entries
- `UserPeripheral` enum in `core::peripherals` wrapping `RomDisk`, `MidiInterface`, and `None`; replaces the direct `romdisk` field on `Bus` with a unified `user_slot`
- `Machine::plug_user_peripheral()` to attach a `UserPeripheral` at runtime
- `Machine::drain_midi_out()` to drain the MIDI output buffer via callback
- `current_cycle` field on `Bus` propagates the running CPU cycle counter into peripheral writes for MIDI timestamping
- MIDI output thread with `SpinSleeper`-based cycle-accurate scheduling: events are dispatched at their recorded cycle timestamp relative to a live anchor; anchor resets when lag exceeds three frame durations
- All Notes Off and All Sound Off sent to all 16 channels on MIDI connection teardown
- Virtual MIDI port creation on Unix when the requested port name is not found among existing ports
- `AppConfig` struct consolidating `App::new()` parameters (`debug_mode`, `recorder`, `player`, `rom_name`, `midi_out`)
- `App::cycle()` private method encapsulating one machine tick, audio push, and MIDI drain
- `--rka` and `--rom` named flags as alternatives to the positional `file` argument; positional argument continues to dispatch by extension as before
- `midir`, `midly`, and `spin_sleep` dependencies added

### Changed

- `Bus` field `romdisk: RomDisk` replaced by `user_slot: UserPeripheral`; `port_a_out` is now forwarded to `user_slot.update()` alongside `port_b`, `port_c`, and `current_cycle`
- `Machine::load_rom()` renamed to `Machine::load_rka()`; ROM disk loading path removed from it and moved to `plug_user_peripheral()`
- ROM disk and MIDI interface are mutually exclusive; specifying both simultaneously is rejected at startup with a descriptive error
- Audio disconnection is now detected and propagated correctly inside `App::cycle()`, unifying the error path between step-frame and normal execution

## 0.1.6

### Fixed

- `port_in` and `port_out` on `Bus` now implement 8080 port address mirroring. The 8-bit port number is duplicated into both bytes of the 16-bit address (e.g. port `0xEC` to address `0xECEC`) and forwarded to `peek` / `poke`, matching the memory-mapped I/O model of the Apogee BK-01 hardware. Previously both methods were no-ops, which silently discarded all port traffic and broke programs that drive the VI53 timer via `OUT` instructions

## 0.1.5

### Added

- Debug mode with `--debug` flag exposing hotkeys: F8 (pause/resume), F9 (step one frame while paused), F10 (dump snapshot)
- Replay recording via `--record` flag (requires `--debug`): key events and snapshot markers serialised to JSON on exit with intermediate saves on each snapshot
- Replay playback via `--play <file>`: replays recorded input deterministically; keyboard input blocked during playback
- `ReplayRecorder`, `ReplayPlayer`, `ReplayMetadata`, `ReplayEvent`, `ReplayAction` types in new `core::debug` module
- `MachineState` struct serialisable to JSON, exposing cycle count, PC, and a SHA-256 hash of RAM
- `Machine::validate_rka()` extracted as a public static method; called independently before `load_rom` in `main()`
- `Machine::cycle_count()` and `Machine::state()` accessors
- `dump_snapshot()` on `App`: writes `<name>.json` (machine state) and `<name>.png` (frame buffer) side by side
- SHA-256 hashes for bundled assets moved to sidecar `.sha256` files included at compile time via `include_str!`; hardcoded hash strings removed from source
- `ChecksumMismatch` variant on `MachineError` now carries `expected` and `got` fields for diagnostic output
- Window resizes to match new video dimensions when `render_frame` reports a resolution change
- `serde`, `serde-big-array`, `serde_json`, `image`, `assert-json-diff`, `test-generator` dependencies added
- `Serialize` / `Deserialize` derived on all core chip structs, peripheral structs, `ColorMode`, `Key`, and `ParsedSymbol`; RAM and parsed frame serialised as SHA-256 hashes to keep snapshots compact
- Debug flags (`--debug`, `--record`, `--play`) hidden from `--help` unless `--debug` is present on the command line

### Changed

- `App::new()` extended with `debug_mode`, `recorder`, `player`, and `rom_name` parameters
- `App` gains `paused`, `step_frame`, `recorder`, `player`, and `rom_name` fields; `about_to_wait` branches on pause/step state before entering the audio-driven tick loop
- `ControlFlow::Wait` used while paused (replaces unconditional `Poll`), eliminating busy-spin when emulation is suspended
- `load_rom` no longer handles ROM disk path inline; `.rom` extension validated in `main()` before the call; error message simplified to a single generic context string
- `Box::new([0; N])` replaced with `vec![...].into_boxed_slice().try_into().unwrap()` for large stack-allocated arrays (`ram`, `parsed_frame`) to avoid stack overflow on debug builds
- `autorun` loop rewritten as a `step_by` iterator over `DEFAULT_FRAME_CYCLES` instead of a manual `cycles_done` accumulator
- `rom_name` and `rom_sha256` derived from the loaded file path; `"monitor"` / `SYSTEM_ROM_HASH` used as defaults when no file is provided
- `app.fatal_error` taken with `.take()` instead of moved, allowing `App` to remain valid through the `exiting` handler
- `exiting` handler on `App` saves recorder state on clean exit

## 0.1.4

### Added

- `DEFAULT_FRAME_CYCLES` and `MAX_FRAME_CYCLES` compile-time constants derived directly from VG75 and CPU hardware specs; replace all remaining magic cycle and latency numbers
- `is_raster_running()` accessor on `Kr580Vg75`

### Changed

- Synchronization model replaced: wall-clock / delta-time loop removed in favour of audio-buffer-driven execution; `machine.run(elapsed_secs, ...)` to `machine.tick(push_sample)` returning a `bool` vblank flag
- Frame rendering decoupled from the tick callback; `render_frame` closure removed from the machine API as rendering is triggered in the event loop only when `tick()` returns `true`
- Throttle guard replaced with a hot `ControlFlow::Poll` + `yield_now()` spin against a hardware-derived 1.5-frame audio latency watermark, eliminating OS-sleep wake-up jitter
- `AudioMixer` phase accumulator reworked to operate on `master_clock_hz` and `cpu_divider` directly instead of a pre-divided `cpu_freq`; removes rounding error and makes drift mathematically impossible
- Audio channel capacity changed from hardcoded `8192` to `sample_rate / 2` (0.5 seconds), providing a reliable shock absorber against OS thread suspension
- `AudioSystem` is now constructed before `Machine`; sample rate is passed at construction time, removing `set_sample_rate()`
- `App::new()` made infallible; audio initialisation moved to `main()`
- `Instant` / `Duration` imports and `last_time` field removed from `App`
- `pending_cycles` field removed from `Machine`

### Removed

- Redundant `rfd` dependency

## 0.1.3

### Changed

- DMA/CRT pipeline (`Kr580Vg75` + `Kr580Vt57`) refactored from monolithic row-fetch into a true cycle-accurate state machine: `fetch_dma_row` removed, `tick()` split into `tick()` (per-CPU-cycle DMA step) and `tick_char()` (character-clock step); CPU is now halted exactly 4 cycles per byte fetched via HRQ, while the VG75 manages its own internal FIFO delays (7 and 3 cycles) through a dedicated `dma_timer` counter
- `dma_bytes_left` / `dma_space_counter` fields replaced by `cur_burst_pos`, `dma_timer`, `dma_paused`, and `need_extra_byte` to track per-cycle burst state
- `next_row()` and `next_frame()` no longer accept `vt57` / `ram` arguments; DMA is driven cycle-by-cycle from the machine loop instead
- Square wave generation in `Kr580Vi53` modes 3 and 7 now implements real hardware asymmetries for edge-case reload values: reload `1` to 32769 high / 32768 low; reload `3` to 2 high / 32769 low (previously both fell through to incorrect `div_ceil` logic)
- `reload_latch` intermediate field introduced in `TimerChannel` to correctly stage LSB/MSB writes before committing to `reload`
- Default audio sample rate changed from 44 100 Hz to 48 000 Hz to align with modern OS audio mixers and reduce resampling jitter
- `Instant::now()` / delta-time calculation in the Winit event loop moved to after the audio-queue throttle check, preventing time-delta accumulation during backpressure stalls

## 0.1.2

### Added

- `--force` / `-f` CLI flag: skips RKA validation and loads the file anyway, tolerating inverted address ranges, truncated payloads, and missing checksums
- SHA-256 integrity check for bundled assets (`apogee.rom`, `sga.bin`) on startup
- `err_rx` channel on `AudioSystem` for propagating runtime audio stream errors to the main loop
- `fatal_error` field on `App` for structured fatal error reporting

### Changed

- `main()` now returns `Result<()>`; all `eprintln!` + `process::exit` replaced with `anyhow` error propagation
- `App::new()` and `AudioSystem::new()` now return `Result<Self>` instead of being infallible
- `load_rom` signature extended with `force: bool` parameter and migrated from `Result<(), &'static str>` to `anyhow::Result<()>`
- Audio stream error callback now sends errors over a channel instead of printing to stderr
- `is_beeper_active()` renamed to `is_tape_out_active()` and constant `BEEPER_BIT_MASK` renamed to `TAPE_OUT_BIT_MASK` to reflect actual hardware function
- `AudioMixer::tick()` parameter renamed from `beeper_state` to `tape_out_state`

## 0.1.1

### Added

- `--autorun` / `-a` CLI flag: executes 2,000,000 warm-up cycles before injecting the RKA payload, bypassing manual system monitor interaction
- Authentic RKA checksum validation replicating the 8080 ADD/ADC algorithm; invalid files are rejected with a descriptive error
- `memory_map` module in `bus.rs` with symbolic address range constants
- `is_beeper_active()` helper on `Kr580Vv55a`
- `PitRwMode`, `PitPhase` enums in `Kr580Vi53`; `BytePhase` enum in `Kr580Vt57`
- Named constants for all previously magic numbers across all chip modules

### Changed

- Emulation loop is now delta-time driven with a 50 ms spike cap
- Audio throttle timer resets on wake, eliminating crackling on window move/minimize
- Halt and normal CPU cycles unified into a single execution path in `machine.rs`
- DMA timing model extended with burst count and inter-burst spacing
- FIFO in `Kr580Vg75` replaced from `Vec` to fixed-size `[u8; 16]` array
- `load_rom` now returns `Result<(), &'static str>` instead of being infallible

## 0.1.0

### Added

- Initial commit
