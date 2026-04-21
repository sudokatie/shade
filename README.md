# Shade

Privacy-first personal analytics. All data stays local.

## Why?

Every "screen time" app wants your data. They track you to sell insights, optimize engagement, or just because they can. Shade does the opposite: your data never leaves your machine. No accounts. No cloud sync. No telemetry. Just you and your own usage patterns.

## Features

- **Screen Time Tracking** - Know exactly where your hours go
- **App Categories** - Group apps by type (Development, Browsers, Communication, etc.)
- **Time Goals** - Set daily limits per app or category with warnings at 80% and 100%
- **Daily Summaries** - Quick view of today's usage with progress toward goals
- **TUI Dashboard** - Beautiful terminal interface for exploring your data
- **JSON/CSV Export** - Your data in open formats, whenever you want it
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

# Export your data (JSON)
shade export -o my-data.json --from 2026-01-01 --to 2026-01-30

# Export to CSV
shade export -o screen-time.csv --format csv --csv-type apps
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
| `shade export` | Export data to JSON or CSV |
| `shade category list` | List all categories |
| `shade category add <bundle_id> <category>` | Add app to a category |
| `shade category remove <bundle_id> <category>` | Remove app from category |
| `shade category show <category>` | Show apps in a category |
| `shade goals list` | List all time goals |
| `shade goals add <target> <minutes>` | Add a time goal (use --category for categories) |
| `shade goals remove <target>` | Remove a time goal |
| `shade goals status` | Show progress toward all goals |

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

# Custom categories (merged with built-in defaults)
categories:
  - name: Work
    patterns:
      - com.mycompany.app
      - com.custom.tool
  - name: Gaming
    patterns:
      - com.steam.app
```

## Categories

Shade comes with built-in categories for common apps (Browsers, Development, Communication, etc.). You can also define your own categories that override or extend the defaults.

### Managing Categories

```bash
# List all categories and app counts
shade category list

# Add an app to a category
shade category add com.example.app "My Category"

# Remove an app from a category
shade category remove com.example.app "My Category"

# Show all apps in a category
shade category show Development
```

User-defined categories in config.yaml take precedence over built-in defaults, so you can recategorize any app to fit your workflow.

## Time Goals

Set daily limits to keep yourself accountable. Goals can target specific apps or entire categories.

### Managing Goals

```bash
# List all goals
shade goals list

# Add a 2-hour limit for Social category
shade goals add Social 120 --category

# Add a 30-minute limit for a specific app
shade goals add com.twitter.Twitter 30

# Check progress toward all goals
shade goals status

# Remove a goal
shade goals remove Social --category
```

### Goal Status Output

```
Goal Progress (Today):

  Social                     45m / 120m ( 37.5%) [OK] (cat)
    1h 15m left
  com.twitter.Twitter        28m /  30m ( 93.3%) [WARNING] (app)
    2m left
  Entertainment             125m / 120m (104.2%) [OVER LIMIT] (cat)
```

Goals warn at 80% by default and show "OVER LIMIT" when exceeded. Configure goals in `~/.shade/config.yaml`:

```yaml
goals:
  - target: Social
    is_category: true
    daily_limit_minutes: 120
    warn_at_percent: 80
  - target: com.twitter.Twitter
    is_category: false
    daily_limit_minutes: 30
```

## Export

Export your data for external analysis or backup. Shade supports JSON and CSV formats.

### JSON Export

```bash
# Export last 30 days (default)
shade export -o my-data.json

# Export specific date range
shade export -o january.json --from 2026-01-01 --to 2026-01-31
```

JSON export includes daily summaries, category breakdowns, and app-by-app details.

### CSV Export

```bash
# Export per-app breakdown (default csv-type)
shade export -o apps.csv --format csv

# Export daily totals
shade export -o daily.csv --format csv --csv-type daily

# Export category breakdown
shade export -o categories.csv --format csv --csv-type categories

# Specific date range
shade export -o week.csv --format csv --csv-type apps --from 2026-03-01 --to 2026-03-07
```

CSV types:
- **apps** - Per-app time per day (date, app name, bundle ID, duration)
- **daily** - Total screen time per day
- **categories** - Time per category per day

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

- **macOS** - Full support (uses NSWorkspace/AppleScript for app tracking)
- **Linux** - X11 support (uses x11rb for active window, screensaver extension for idle)
- **Windows** - Full support (uses Win32 GetForegroundWindow, GetLastInputInfo for idle)

### Linux Notes

Linux support requires X11. Wayland is not currently supported due to the lack of a standard protocol for getting the active window (each compositor has its own approach).

To use on Linux:
1. Ensure you're running an X11 session (or XWayland)
2. The screensaver extension must be available for idle detection

### Windows Notes

Windows support uses the Win32 API to track the foreground window and detect idle time.

## Roadmap

### v0.2 (Mostly Complete)
- [x] Linux support (X11 via x11rb)
- [x] Windows support (Win32 GetForegroundWindow)
- [ ] Wayland support (compositor-specific, deferred - no standard protocol)

See FEATURE-BACKLOG.md in the clawd repo for detailed acceptance criteria.

## License

MIT

## Author

Katie
