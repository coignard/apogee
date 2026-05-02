# Changelog

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
- Frame rendering decoupled from the tick callback; `render_frame` closure removed from the machine API — rendering is triggered in the event loop only when `tick()` returns `true`
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
