# Changelog

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
