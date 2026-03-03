//! Goal progress tracking and warnings

use crate::config::TimeGoal;
use std::collections::HashMap;

/// Progress toward a time goal
#[derive(Debug, Clone)]
pub struct GoalProgress {
    /// The goal being tracked
    pub goal: TimeGoal,
    /// Time used in minutes
    pub used_minutes: u32,
    /// Percentage of limit used (0-100+)
    pub percent_used: f32,
    /// Whether warning threshold reached
    pub warning_triggered: bool,
    /// Whether limit exceeded
    pub limit_exceeded: bool,
}

impl GoalProgress {
    /// Create new progress for a goal
    pub fn new(goal: TimeGoal, used_minutes: u32) -> Self {
        let percent_used = if goal.daily_limit_minutes > 0 {
            (used_minutes as f32 / goal.daily_limit_minutes as f32) * 100.0
        } else {
            0.0
        };
        let warning_triggered = percent_used >= goal.warn_at_percent as f32;
        let limit_exceeded = percent_used >= 100.0;

        Self {
            goal,
            used_minutes,
            percent_used,
            warning_triggered,
            limit_exceeded,
        }
    }

    /// Remaining minutes before limit
    pub fn remaining_minutes(&self) -> i32 {
        self.goal.daily_limit_minutes as i32 - self.used_minutes as i32
    }

    /// Format remaining time as human-readable string
    pub fn remaining_display(&self) -> String {
        let remaining = self.remaining_minutes();
        if remaining <= 0 {
            "Over limit".to_string()
        } else if remaining < 60 {
            format!("{}m left", remaining)
        } else {
            format!("{}h {}m left", remaining / 60, remaining % 60)
        }
    }
}

/// Warning level for goal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    /// Under warning threshold
    Normal,
    /// At or over warning threshold, under limit
    Warning,
    /// At or over limit
    Exceeded,
}

impl GoalProgress {
    /// Get the warning level
    pub fn warning_level(&self) -> WarningLevel {
        if self.limit_exceeded {
            WarningLevel::Exceeded
        } else if self.warning_triggered {
            WarningLevel::Warning
        } else {
            WarningLevel::Normal
        }
    }
}

/// Check all goals against current usage
///
/// - app_usage: map of bundle_id -> minutes used today
/// - category_usage: map of category -> minutes used today
/// - goals: list of configured goals
pub fn check_goals(
    app_usage: &HashMap<String, u32>,
    category_usage: &HashMap<String, u32>,
    goals: &[TimeGoal],
) -> Vec<GoalProgress> {
    goals
        .iter()
        .map(|goal| {
            let used = if goal.is_category {
                category_usage.get(&goal.target).copied().unwrap_or(0)
            } else {
                app_usage.get(&goal.target).copied().unwrap_or(0)
            };
            GoalProgress::new(goal.clone(), used)
        })
        .collect()
}

/// Get only goals that have triggered warnings
pub fn get_warnings(progress: &[GoalProgress]) -> Vec<&GoalProgress> {
    progress
        .iter()
        .filter(|p| p.warning_triggered)
        .collect()
}

/// Get only goals that have exceeded limits
pub fn get_exceeded(progress: &[GoalProgress]) -> Vec<&GoalProgress> {
    progress
        .iter()
        .filter(|p| p.limit_exceeded)
        .collect()
}

/// Weekly summary comparing usage to goals
#[derive(Debug, Clone)]
pub struct WeeklySummary {
    /// Goal target
    pub target: String,
    /// Whether target is a category
    pub is_category: bool,
    /// Daily limit in minutes
    pub daily_limit: u32,
    /// Minutes used each day (index 0 = oldest)
    pub daily_usage: Vec<u32>,
    /// Total minutes for the week
    pub total_minutes: u32,
    /// Days where limit was exceeded
    pub days_exceeded: u32,
    /// Average daily usage
    pub avg_daily_minutes: f32,
}

impl WeeklySummary {
    /// Create from daily usage data
    pub fn new(goal: &TimeGoal, daily_usage: Vec<u32>) -> Self {
        let total_minutes: u32 = daily_usage.iter().sum();
        let days_exceeded = daily_usage.iter().filter(|&&m| m > goal.daily_limit_minutes).count() as u32;
        let avg_daily_minutes = if !daily_usage.is_empty() {
            total_minutes as f32 / daily_usage.len() as f32
        } else {
            0.0
        };

        Self {
            target: goal.target.clone(),
            is_category: goal.is_category,
            daily_limit: goal.daily_limit_minutes,
            daily_usage,
            total_minutes,
            days_exceeded,
            avg_daily_minutes,
        }
    }

    /// Weekly adherence percentage (days within limit / total days)
    pub fn adherence_percent(&self) -> f32 {
        if self.daily_usage.is_empty() {
            100.0
        } else {
            let within_limit = self.daily_usage.len() as u32 - self.days_exceeded;
            (within_limit as f32 / self.daily_usage.len() as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_progress_under_limit() {
        let goal = TimeGoal::for_app("com.example.app", 120);
        let progress = GoalProgress::new(goal, 60);

        assert_eq!(progress.percent_used, 50.0);
        assert!(!progress.warning_triggered);
        assert!(!progress.limit_exceeded);
        assert_eq!(progress.warning_level(), WarningLevel::Normal);
    }

    #[test]
    fn test_goal_progress_at_warning() {
        let goal = TimeGoal::for_app("com.example.app", 100);
        let progress = GoalProgress::new(goal, 80); // 80%

        assert_eq!(progress.percent_used, 80.0);
        assert!(progress.warning_triggered);
        assert!(!progress.limit_exceeded);
        assert_eq!(progress.warning_level(), WarningLevel::Warning);
    }

    #[test]
    fn test_goal_progress_exceeded() {
        let goal = TimeGoal::for_app("com.example.app", 60);
        let progress = GoalProgress::new(goal, 90);

        assert_eq!(progress.percent_used, 150.0);
        assert!(progress.warning_triggered);
        assert!(progress.limit_exceeded);
        assert_eq!(progress.warning_level(), WarningLevel::Exceeded);
    }

    #[test]
    fn test_remaining_display() {
        let goal = TimeGoal::for_app("com.example.app", 120);
        let progress = GoalProgress::new(goal, 30);
        assert_eq!(progress.remaining_display(), "1h 30m left");

        let goal2 = TimeGoal::for_app("com.example.app", 60);
        let progress2 = GoalProgress::new(goal2, 45);
        assert_eq!(progress2.remaining_display(), "15m left");

        let goal3 = TimeGoal::for_app("com.example.app", 60);
        let progress3 = GoalProgress::new(goal3, 90);
        assert_eq!(progress3.remaining_display(), "Over limit");
    }

    #[test]
    fn test_check_goals_app() {
        let mut app_usage = HashMap::new();
        app_usage.insert("com.example.app".to_string(), 45);
        let category_usage = HashMap::new();
        let goals = vec![TimeGoal::for_app("com.example.app", 60)];

        let progress = check_goals(&app_usage, &category_usage, &goals);
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].used_minutes, 45);
    }

    #[test]
    fn test_check_goals_category() {
        let app_usage = HashMap::new();
        let mut category_usage = HashMap::new();
        category_usage.insert("Social".to_string(), 90);
        let goals = vec![TimeGoal::for_category("Social", 60)];

        let progress = check_goals(&app_usage, &category_usage, &goals);
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].used_minutes, 90);
        assert!(progress[0].limit_exceeded);
    }

    #[test]
    fn test_get_warnings() {
        let mut app_usage = HashMap::new();
        app_usage.insert("app1".to_string(), 50); // 50% - no warning
        app_usage.insert("app2".to_string(), 85); // 85% - warning
        let category_usage = HashMap::new();
        let goals = vec![
            TimeGoal::for_app("app1", 100),
            TimeGoal::for_app("app2", 100),
        ];

        let progress = check_goals(&app_usage, &category_usage, &goals);
        let warnings = get_warnings(&progress);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].goal.target, "app2");
    }

    #[test]
    fn test_weekly_summary() {
        let goal = TimeGoal::for_app("com.example.app", 60);
        let daily = vec![45, 70, 55, 80, 30, 65, 50]; // 7 days

        let summary = WeeklySummary::new(&goal, daily);
        assert_eq!(summary.total_minutes, 395);
        assert_eq!(summary.days_exceeded, 3); // 70, 80, 65 exceed 60
        assert!((summary.avg_daily_minutes - 56.43).abs() < 0.1);
        assert!((summary.adherence_percent() - 57.14).abs() < 0.1); // 4/7
    }

    #[test]
    fn test_weekly_summary_empty() {
        let goal = TimeGoal::for_app("com.example.app", 60);
        let summary = WeeklySummary::new(&goal, vec![]);

        assert_eq!(summary.total_minutes, 0);
        assert_eq!(summary.days_exceeded, 0);
        assert_eq!(summary.adherence_percent(), 100.0);
    }
}
