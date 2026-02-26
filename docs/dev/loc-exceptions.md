# LoC Exceptions

This project targets Rust source files under ~1000 lines.  
Exceptions are documented here with rationale.

## `src/lib.rs` (~1305 LoC)

- **Reason**: central text-cleaning engine with tightly-coupled hot-path logic
  and extensive inline unit tests for Unicode edge cases.
- **Mitigation plan**:
  - Extract streaming and classification helpers into dedicated modules.
  - Move large unit-test groups to integration tests where practical.
