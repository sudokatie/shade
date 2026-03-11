//! Focus mode analysis for Shade
//!
//! Detects focus vs. distraction patterns from app usage data.

use crate::db::{Database, SessionWithApp};
use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Minimum session duration to count as meaningful (seconds)
const MIN_SESSION_DURATION_SECS: i64 = 5;

/// Minimum focus session duration to count as "flow" (seconds)
const FLOW_THRESHOLD_SECS: i64 = 30 * 60; // 30 minutes

/// Focus analysis for a single day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusAnalysis {
    /// The date analyzed
    pub date: NaiveDate,
    /// Number of app switches
    pub switch_count: usize,
    /// Switches per hour of active time
    pub switches_per_hour: f64,
    /// Flow sessions (sustained single-app usage > 30 min)
    pub flow_sessions: Vec<FlowSession>,
    /// Total time in flow state (seconds)
    pub total_flow_secs: i64,
    /// Total active time (seconds)
    pub total_active_secs: i64,
    /// Focus score (0-100)
    pub focus_score: u8,
}

/// A detected flow session (sustained focus)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSession {
    /// Application name
    pub app_name: String,
    /// Bundle ID
    pub bundle_id: String,
    /// Duration in seconds
    pub duration_secs: i64,
    /// Start time (ISO 8601)
    pub started_at: String,
}

impl FocusAnalysis {
    /// Format switches per hour
    pub fn format_switches_per_hour(&self) -> String {
        format!("{:.1}/hr", self.switches_per_hour)
    }

    /// Format flow time
    pub fn format_flow_time(&self) -> String {
        let hours = self.total_flow_secs / 3600;
        let minutes = (self.total_flow_secs % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    /// Get focus score interpretation
    pub fn score_interpretation(&self) -> &'static str {
        match self.focus_score {
            90..=100 => "Excellent",
            70..=89 => "Good",
            50..=69 => "Fair",
            30..=49 => "Poor",
            _ => "Very Distracted",
        }
    }
}

/// Analyze focus patterns for a given date
pub fn analyze_focus(db: &Database, date: NaiveDate) -> Result<FocusAnalysis> {
    let sessions = db.get_sessions_for_date(date)?;

    if sessions.is_empty() {
        return Ok(FocusAnalysis {
            date,
            switch_count: 0,
            switches_per_hour: 0.0,
            flow_sessions: vec![],
            total_flow_secs: 0,
            total_active_secs: 0,
            focus_score: 0,
        });
    }

    // Calculate switch count (transitions between different apps)
    let switch_count = count_app_switches(&sessions);

    // Calculate total active time
    let total_active_secs = calculate_total_active_time(&sessions);

    // Calculate switches per hour
    let active_hours = total_active_secs as f64 / 3600.0;
    let switches_per_hour = if active_hours > 0.0 {
        switch_count as f64 / active_hours
    } else {
        0.0
    };

    // Detect flow sessions
    let flow_sessions = detect_flow_sessions(&sessions);
    let total_flow_secs: i64 = flow_sessions.iter().map(|f| f.duration_secs).sum();

    // Calculate focus score
    let focus_score = calculate_focus_score(switches_per_hour, total_flow_secs, total_active_secs);

    Ok(FocusAnalysis {
        date,
        switch_count,
        switches_per_hour,
        flow_sessions,
        total_flow_secs,
        total_active_secs,
        focus_score,
    })
}

/// Analyze focus for today
pub fn analyze_focus_today(db: &Database) -> Result<FocusAnalysis> {
    let today = Utc::now().date_naive();
    analyze_focus(db, today)
}

/// Count app switches (transitions between different apps)
fn count_app_switches(sessions: &[SessionWithApp]) -> usize {
    if sessions.len() < 2 {
        return 0;
    }

    let mut switches = 0;
    let mut prev_app_id = sessions[0].application_id;

    for session in sessions.iter().skip(1) {
        // Only count if session is meaningful (not just a brief flash)
        let duration = session_duration_secs(session);
        if duration >= MIN_SESSION_DURATION_SECS && session.application_id != prev_app_id {
            switches += 1;
            prev_app_id = session.application_id;
        } else if duration >= MIN_SESSION_DURATION_SECS {
            prev_app_id = session.application_id;
        }
    }

    switches
}

/// Calculate total active time from sessions
fn calculate_total_active_time(sessions: &[SessionWithApp]) -> i64 {
    sessions.iter().map(session_duration_secs).sum()
}

/// Get session duration in seconds
fn session_duration_secs(session: &SessionWithApp) -> i64 {
    let end = session.ended_at.unwrap_or_else(Utc::now);
    let duration = end.signed_duration_since(session.started_at);
    duration.num_seconds().max(0)
}

/// Detect flow sessions (sustained single-app usage)
fn detect_flow_sessions(sessions: &[SessionWithApp]) -> Vec<FlowSession> {
    if sessions.is_empty() {
        return vec![];
    }

    let mut flow_sessions = Vec::new();
    let mut current_app_id = sessions[0].application_id;
    let mut current_app_name = sessions[0].app_name.clone();
    let mut current_bundle_id = sessions[0].bundle_id.clone();
    let mut current_start = sessions[0].started_at;
    let mut current_duration: i64 = 0;

    for session in sessions {
        let duration = session_duration_secs(session);

        if session.application_id == current_app_id {
            // Same app, accumulate duration
            current_duration += duration;
        } else {
            // Different app - check if previous was a flow session
            if current_duration >= FLOW_THRESHOLD_SECS {
                flow_sessions.push(FlowSession {
                    app_name: current_app_name.clone(),
                    bundle_id: current_bundle_id.clone(),
                    duration_secs: current_duration,
                    started_at: current_start.to_rfc3339(),
                });
            }

            // Start new accumulation
            current_app_id = session.application_id;
            current_app_name = session.app_name.clone();
            current_bundle_id = session.bundle_id.clone();
            current_start = session.started_at;
            current_duration = duration;
        }
    }

    // Check final session
    if current_duration >= FLOW_THRESHOLD_SECS {
        flow_sessions.push(FlowSession {
            app_name: current_app_name,
            bundle_id: current_bundle_id,
            duration_secs: current_duration,
            started_at: current_start.to_rfc3339(),
        });
    }

    flow_sessions
}

/// Calculate focus score (0-100)
fn calculate_focus_score(switches_per_hour: f64, flow_secs: i64, active_secs: i64) -> u8 {
    if active_secs == 0 {
        return 0;
    }

    // Component 1: Switch frequency (lower is better)
    // 0 switches/hr = 100, 60+ switches/hr = 0
    let switch_score = (100.0 - (switches_per_hour * 100.0 / 60.0).min(100.0)).max(0.0);

    // Component 2: Flow time ratio (higher is better)
    // 100% flow = 100, 0% flow = 0
    let flow_ratio = flow_secs as f64 / active_secs as f64;
    let flow_score = flow_ratio * 100.0;

    // Combined score (weighted)
    // 60% switch frequency, 40% flow time
    let combined = switch_score * 0.6 + flow_score * 0.4;

    combined.round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn make_session(
        id: i64,
        app_id: i64,
        app_name: &str,
        bundle_id: &str,
        start_offset_mins: i64,
        duration_mins: i64,
    ) -> SessionWithApp {
        let base = Utc.with_ymd_and_hms(2026, 3, 11, 9, 0, 0).unwrap();
        let started_at = base + Duration::minutes(start_offset_mins);
        let ended_at = started_at + Duration::minutes(duration_mins);

        SessionWithApp {
            id,
            application_id: app_id,
            started_at,
            ended_at: Some(ended_at),
            ended_idle: false,
            bundle_id: bundle_id.to_string(),
            app_name: app_name.to_string(),
        }
    }

    #[test]
    fn test_count_app_switches_empty() {
        let sessions: Vec<SessionWithApp> = vec![];
        assert_eq!(count_app_switches(&sessions), 0);
    }

    #[test]
    fn test_count_app_switches_single() {
        let sessions = vec![make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 60)];
        assert_eq!(count_app_switches(&sessions), 0);
    }

    #[test]
    fn test_count_app_switches_same_app() {
        let sessions = vec![
            make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 30),
            make_session(2, 1, "VSCode", "com.microsoft.VSCode", 30, 30),
        ];
        assert_eq!(count_app_switches(&sessions), 0);
    }

    #[test]
    fn test_count_app_switches_different_apps() {
        let sessions = vec![
            make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 30),
            make_session(2, 2, "Safari", "com.apple.Safari", 30, 15),
            make_session(3, 1, "VSCode", "com.microsoft.VSCode", 45, 30),
        ];
        assert_eq!(count_app_switches(&sessions), 2);
    }

    #[test]
    fn test_detect_flow_sessions_none() {
        // Sessions all under 30 minutes
        let sessions = vec![
            make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 15),
            make_session(2, 2, "Safari", "com.apple.Safari", 15, 10),
        ];
        let flows = detect_flow_sessions(&sessions);
        assert!(flows.is_empty());
    }

    #[test]
    fn test_detect_flow_sessions_single_long() {
        // One 45-minute session
        let sessions = vec![make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 45)];
        let flows = detect_flow_sessions(&sessions);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].app_name, "VSCode");
        assert_eq!(flows[0].duration_secs, 45 * 60);
    }

    #[test]
    fn test_detect_flow_sessions_accumulated() {
        // Multiple sessions in same app that accumulate to > 30 min
        let sessions = vec![
            make_session(1, 1, "VSCode", "com.microsoft.VSCode", 0, 20),
            make_session(2, 1, "VSCode", "com.microsoft.VSCode", 20, 20),
        ];
        let flows = detect_flow_sessions(&sessions);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].duration_secs, 40 * 60);
    }

    #[test]
    fn test_calculate_focus_score_perfect() {
        // No switches, all flow time
        let score = calculate_focus_score(0.0, 3600, 3600);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_calculate_focus_score_poor() {
        // 60 switches/hr, no flow time
        let score = calculate_focus_score(60.0, 0, 3600);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_calculate_focus_score_mixed() {
        // 10 switches/hr, half flow time
        let score = calculate_focus_score(10.0, 1800, 3600);
        // switch_score = 100 - (10*100/60) = 83.3
        // flow_score = 50
        // combined = 83.3 * 0.6 + 50 * 0.4 = 50 + 20 = 70
        assert!(score >= 65 && score <= 75);
    }

    #[test]
    fn test_focus_analysis_interpretation() {
        let analysis = FocusAnalysis {
            date: Utc::now().date_naive(),
            switch_count: 5,
            switches_per_hour: 2.5,
            flow_sessions: vec![],
            total_flow_secs: 3600,
            total_active_secs: 7200,
            focus_score: 85,
        };
        assert_eq!(analysis.score_interpretation(), "Good");
    }

    #[test]
    fn test_format_switches_per_hour() {
        let analysis = FocusAnalysis {
            date: Utc::now().date_naive(),
            switch_count: 0,
            switches_per_hour: 12.5,
            flow_sessions: vec![],
            total_flow_secs: 0,
            total_active_secs: 0,
            focus_score: 0,
        };
        assert_eq!(analysis.format_switches_per_hour(), "12.5/hr");
    }

    #[test]
    fn test_format_flow_time() {
        let analysis = FocusAnalysis {
            date: Utc::now().date_naive(),
            switch_count: 0,
            switches_per_hour: 0.0,
            flow_sessions: vec![],
            total_flow_secs: 3661, // 1h 1m 1s
            total_active_secs: 0,
            focus_score: 0,
        };
        assert_eq!(analysis.format_flow_time(), "1h 1m");
    }
}
