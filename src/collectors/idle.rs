//! Idle detection for Shade
//!
//! Detects how long since the user last interacted with the system.

/// Default idle threshold in seconds (5 minutes)
pub const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 300;

/// CGEventSourceStateID values for raw FFI
#[cfg(target_os = "macos")]
mod ffi {
    /// Combined session state
    pub const COMBINED_SESSION_STATE: i32 = 0;

    /// CGEventType values
    pub const KEY_DOWN: u32 = 10;
    pub const MOUSE_MOVED: u32 = 5;
    pub const LEFT_MOUSE_DOWN: u32 = 1;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    }
}

/// Get seconds since last user input
///
/// Returns the number of seconds since any keyboard or mouse input.
/// Returns 0 if unable to determine.
#[cfg(target_os = "macos")]
pub fn seconds_since_last_input() -> f64 {
    unsafe {
        // Check seconds since last keyboard event
        let keyboard_idle =
            ffi::CGEventSourceSecondsSinceLastEventType(ffi::COMBINED_SESSION_STATE, ffi::KEY_DOWN);

        // Check seconds since last mouse event
        let mouse_idle = ffi::CGEventSourceSecondsSinceLastEventType(
            ffi::COMBINED_SESSION_STATE,
            ffi::MOUSE_MOVED,
        );

        // Check seconds since last click
        let click_idle = ffi::CGEventSourceSecondsSinceLastEventType(
            ffi::COMBINED_SESSION_STATE,
            ffi::LEFT_MOUSE_DOWN,
        );

        // Return the minimum (most recent activity)
        keyboard_idle.min(mouse_idle).min(click_idle)
    }
}

/// Get seconds since last user input (non-macOS stub)
#[cfg(not(target_os = "macos"))]
pub fn seconds_since_last_input() -> f64 {
    // Return 0 on unsupported platforms (always "active")
    0.0
}

/// Check if the user is currently idle
///
/// # Arguments
/// * `threshold_secs` - Number of seconds of inactivity to consider idle
pub fn is_idle(threshold_secs: u64) -> bool {
    seconds_since_last_input() >= threshold_secs as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_since_last_input_returns_reasonable_value() {
        let idle_secs = seconds_since_last_input();
        // Should return a non-negative value
        assert!(idle_secs >= 0.0);
        // Should be less than a day (sanity check)
        assert!(idle_secs < 86400.0);
    }

    #[test]
    fn test_is_idle_with_high_threshold() {
        // With a very high threshold, user should not be considered idle
        // (unless the test machine has been idle for hours)
        let result = is_idle(3600); // 1 hour threshold
                                    // This test just verifies the function runs without panicking
                                    // The actual result depends on system state
        let _ = result;
    }

    #[test]
    fn test_is_idle_with_zero_threshold() {
        // With zero threshold, any idle time makes user idle
        // This should return true in most cases since even a few ms count
        let idle_secs = seconds_since_last_input();
        let result = is_idle(0);
        // If we have any idle time, result should be true
        if idle_secs > 0.0 {
            assert!(result);
        }
    }

    #[test]
    fn test_default_threshold_constant() {
        assert_eq!(DEFAULT_IDLE_THRESHOLD_SECS, 300);
    }
}
