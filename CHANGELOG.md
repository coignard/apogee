# Changelog

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
