# Changelog

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
