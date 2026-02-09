# Shade

Privacy-first personal analytics. All data stays local.

## Why?

Every "screen time" app wants your data. They track you to sell insights, optimize engagement, or just because they can. Shade does the opposite: your data never leaves your machine. No accounts. No cloud sync. No telemetry. Just you and your own usage patterns.

## Features

- **Screen Time Tracking** - Know exactly where your hours go
- **App Categories** - Group apps by type (Development, Browsers, Communication, etc.)
- **Daily Summaries** - Quick view of today's usage with progress toward goals
- **TUI Dashboard** - Beautiful terminal interface for exploring your data
- **JSON Export** - Your data in an open format, whenever you want it
- **Idle Detection** - Pauses tracking when you step away

## Quick Start

```bash
# Initialize Shade
shade init

# Start tracking (runs in background)
shade start

# Check today's screen time
shade today

# Open the dashboard
shade dashboard

# Export your data
shade export -o my-data.json --from 2026-01-01 --to 2026-01-30
```

## Installation

### From Source

```bash
git clone https://github.com/sudokatie/shade
cd shade
cargo build --release
cp target/release/shade /usr/local/bin/
```

### Homebrew (coming soon)

```bash
brew install sudokatie/tap/shade
```

## Commands

| Command | Description |
|---------|-------------|
| `shade init` | Create config and database |
| `shade start` | Start the tracking daemon |
| `shade stop` | Stop the daemon |
| `shade status` | Show daemon status and today's time |
| `shade today` | Detailed summary of today's usage |
| `shade apps` | Top apps by usage time |
| `shade list` | All tracked applications |
| `shade dashboard` | Interactive TUI dashboard |
| `shade export` | Export data to JSON |

## Configuration

Config lives at `~/.shade/config.yaml`:

```yaml
# Database location
db_path: ~/.shade/shade.db

# Seconds of inactivity before pausing tracking
idle_timeout_secs: 300

# How often to check the foreground app
collection_interval_secs: 1

# Track window titles (privacy-sensitive)
track_window_titles: false
```

## Data Storage

Everything stays in `~/.shade/`:
- `shade.db` - SQLite database with all your data
- `config.yaml` - Your configuration

The database is plain SQLite. Query it directly if you want:

```bash
sqlite3 ~/.shade/shade.db "SELECT name, SUM(duration) FROM sessions JOIN apps ON ... GROUP BY name"
```

## Privacy

Shade collects:
- Which apps are in the foreground
- How long each app is used
- When you're idle (but not what you're doing)

Shade never:
- Sends data anywhere
- Records keystrokes or screen content
- Tracks browser history (unless you explicitly enable an extension)
- Phones home for updates or analytics

Your data. Your machine. Period.

## Platform Support

- **macOS** - Full support (uses NSWorkspace for app tracking)
- **Linux** - In development
- **Windows** - In development

## Roadmap

### v0.2 (Planned)
- [ ] Linux support (X11 via x11rb, Wayland experimental)
- [ ] Windows support (Win32 GetForegroundWindow)

See FEATURE-BACKLOG.md in the clawd repo for detailed acceptance criteria.

## License

MIT

## Author

Katie
