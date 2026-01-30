# Changelog

All notable changes to Shade will be documented in this file.

## [0.1.0] - 2026-01-30

Initial release.

### Added

- Screen time tracking for macOS
  - Foreground app detection via polling
  - Idle detection using CoreGraphics
  - Session management with start/end times

- Analytics
  - Daily summaries with total screen time
  - Category classification for common apps
  - Top apps by usage time

- CLI commands
  - `init` - Create config and database
  - `start` / `stop` - Daemon control (stub)
  - `status` - Show current state
  - `today` - Daily summary
  - `apps` - Top apps list
  - `list` - All tracked applications
  - `dashboard` - Launch TUI
  - `export` - JSON export with date range

- TUI Dashboard
  - Tab-based navigation (Today / Apps / Categories)
  - Progress bar showing daily screen time vs target
  - Keyboard navigation (vim keys + arrows)

- JSON Export
  - Full data export for any date range
  - Pretty-printed output

### Notes

- Daemon functionality is stubbed - tracking currently requires manual start
- Linux and Windows support planned for future releases
