//! TUI rendering

use super::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs},
    Frame,
};

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help bar
        ])
        .split(frame.area());
    
    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_help(frame, chunks[2]);
}

/// Render the tab bar
fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = vec![
        Tab::Today.title().into(),
        Tab::Apps.title().into(),
        Tab::Categories.title().into(),
    ];
    
    let selected = match app.current_tab {
        Tab::Today => 0,
        Tab::Apps => 1,
        Tab::Categories => 2,
    };
    
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Shade"))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    
    frame.render_widget(tabs, area);
}

/// Render main content based on current tab
fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_tab {
        Tab::Today => render_today(frame, app, area),
        Tab::Apps => render_apps(frame, app, area),
        Tab::Categories => render_categories(frame, app, area),
    }
}

/// Render Today view
fn render_today(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Total time + progress
            Constraint::Min(0),    // Quick stats
        ])
        .split(area);
    
    // Total time and progress bar
    let total_block = Block::default()
        .borders(Borders::ALL)
        .title("Screen Time Today");
    
    let inner = total_block.inner(chunks[0]);
    frame.render_widget(total_block, chunks[0]);
    
    let progress_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);
    
    let time_text = Paragraph::new(format!("Total: {}", app.total_time_str()))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(time_text, progress_chunks[0]);
    
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(app.day_progress())
        .label(format!("{:.0}% of 8h target", app.day_progress() * 100.0));
    frame.render_widget(gauge, progress_chunks[1]);
    
    // Quick stats
    let stats_block = Block::default()
        .borders(Borders::ALL)
        .title("Summary");
    
    let stats_inner = stats_block.inner(chunks[1]);
    frame.render_widget(stats_block, chunks[1]);
    
    let stats = match &app.today_summary {
        Some(summary) => {
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Categories: "),
                    Span::styled(
                        format!("{}", summary.category_breakdown.len()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  Apps tracked: "),
                    Span::styled(
                        format!("{}", summary.top_apps.len()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
            ];
            
            if let Some(top) = summary.top_apps.first() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("  Top app: "),
                    Span::styled(&top.name, Style::default().fg(Color::Cyan)),
                ]));
            }
            
            lines
        }
        None => vec![
            Line::from(""),
            Line::from("  No data yet. Start tracking to see stats."),
        ],
    };
    
    let stats_para = Paragraph::new(stats);
    frame.render_widget(stats_para, stats_inner);
}

/// Render Apps list
fn render_apps(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = match &app.today_summary {
        Some(summary) => summary
            .top_apps
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let hours = a.seconds / 3600;
                let minutes = (a.seconds % 3600) / 60;
                let content = format!("{:>2}. {:30} {:>2}h {:>2}m", i + 1, a.name, hours, minutes);
                
                let style = if i == app.selected_index {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                ListItem::new(content).style(style)
            })
            .collect(),
        None => vec![ListItem::new("No data")],
    };
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Top Apps Today"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    
    frame.render_widget(list, area);
}

/// Render Categories breakdown
fn render_categories(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = match &app.today_summary {
        Some(summary) => summary
            .category_breakdown
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let hours = c.seconds / 3600;
                let minutes = (c.seconds % 3600) / 60;
                let content = format!("{:20} {:>2}h {:>2}m", c.category, hours, minutes);
                
                let style = if i == app.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                ListItem::new(content).style(style)
            })
            .collect(),
        None => vec![ListItem::new("No data")],
    };
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Time by Category"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    
    frame.render_widget(list, area);
}

/// Render help bar
fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = " q: Quit | Tab/←→: Switch view | ↑↓/j/k: Navigate | r: Refresh ";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, area);
}
