//! Category classification for applications

use std::collections::HashMap;

/// Default categories for common applications
pub fn default_categories() -> HashMap<String, String> {
    let mut map = HashMap::new();
    
    // Browsers
    for bundle_id in [
        "com.apple.Safari",
        "org.mozilla.firefox",
        "com.google.Chrome",
        "com.brave.Browser",
        "com.microsoft.edgemac",
        "com.operasoftware.Opera",
        "org.chromium.Chromium",
    ] {
        map.insert(bundle_id.to_string(), "Browsers".to_string());
    }
    
    // Development
    for bundle_id in [
        "com.microsoft.VSCode",
        "com.sublimetext.4",
        "com.jetbrains.intellij",
        "com.jetbrains.pycharm",
        "com.jetbrains.WebStorm",
        "com.apple.dt.Xcode",
        "com.googlecode.iterm2",
        "com.apple.Terminal",
        "io.alacritty",
        "com.github.wez.wezterm",
        "dev.zed.Zed",
        "com.todesktop.230313mzl4w4u92",  // Cursor
    ] {
        map.insert(bundle_id.to_string(), "Development".to_string());
    }
    
    // Communication
    for bundle_id in [
        "com.apple.MobileSMS",
        "com.apple.mail",
        "com.tinyspeck.slackmacgap",
        "com.hnc.Discord",
        "us.zoom.xos",
        "com.microsoft.teams2",
        "com.skype.skype",
        "com.telegram.desktop",
        "net.whatsapp.WhatsApp",
    ] {
        map.insert(bundle_id.to_string(), "Communication".to_string());
    }
    
    // Productivity
    for bundle_id in [
        "com.apple.Notes",
        "com.apple.reminders",
        "com.apple.iCal",
        "md.obsidian",
        "com.notion.id",
        "com.todoist.mac.Todoist",
        "com.microsoft.Word",
        "com.microsoft.Excel",
        "com.microsoft.Powerpoint",
        "com.apple.iWork.Pages",
        "com.apple.iWork.Numbers",
        "com.apple.iWork.Keynote",
    ] {
        map.insert(bundle_id.to_string(), "Productivity".to_string());
    }
    
    // Entertainment
    for bundle_id in [
        "com.spotify.client",
        "com.apple.Music",
        "com.apple.TV",
        "com.netflix.Netflix",
        "com.amazon.aiv.AIVApp",
        "tv.plex.player",
        "com.apple.podcasts",
    ] {
        map.insert(bundle_id.to_string(), "Entertainment".to_string());
    }
    
    // Social
    for bundle_id in [
        "com.twitter.twitter-mac",
        "com.facebook.Facebook",
        "com.burbn.instagram",
        "com.reddit.Reddit",
    ] {
        map.insert(bundle_id.to_string(), "Social".to_string());
    }
    
    // Design
    for bundle_id in [
        "com.figma.Desktop",
        "com.bohemiancoding.sketch3",
        "com.adobe.Photoshop",
        "com.adobe.Illustrator",
        "com.adobe.InDesign",
    ] {
        map.insert(bundle_id.to_string(), "Design".to_string());
    }
    
    // System
    for bundle_id in [
        "com.apple.finder",
        "com.apple.systempreferences",
        "com.apple.Preview",
        "com.apple.ActivityMonitor",
    ] {
        map.insert(bundle_id.to_string(), "System".to_string());
    }
    
    map
}

/// Merge user-defined categories with defaults
/// 
/// User categories take precedence over defaults.
pub fn merge_categories(
    user_categories: &HashMap<String, String>,
    include_defaults: bool,
) -> HashMap<String, String> {
    if include_defaults {
        let mut merged = default_categories();
        merged.extend(user_categories.iter().map(|(k, v)| (k.clone(), v.clone())));
        merged
    } else {
        user_categories.clone()
    }
}

/// Get the category for a bundle ID
/// 
/// Returns "Uncategorized" if not found.
pub fn get_category_for_bundle_id(
    bundle_id: &str,
    categories: &HashMap<String, String>,
) -> String {
    categories
        .get(bundle_id)
        .cloned()
        .unwrap_or_else(|| "Uncategorized".to_string())
}

/// Categorize a list of applications
/// 
/// Returns a map of bundle_id -> category for the given apps.
pub fn categorize_apps(
    bundle_ids: &[String],
    categories: &HashMap<String, String>,
) -> HashMap<String, String> {
    bundle_ids
        .iter()
        .map(|id| (id.clone(), get_category_for_bundle_id(id, categories)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_categories_contains_browsers() {
        let cats = default_categories();
        assert_eq!(cats.get("com.apple.Safari"), Some(&"Browsers".to_string()));
        assert_eq!(cats.get("com.google.Chrome"), Some(&"Browsers".to_string()));
    }

    #[test]
    fn test_default_categories_contains_development() {
        let cats = default_categories();
        assert_eq!(cats.get("com.microsoft.VSCode"), Some(&"Development".to_string()));
        assert_eq!(cats.get("com.apple.dt.Xcode"), Some(&"Development".to_string()));
    }

    #[test]
    fn test_get_category_for_bundle_id_found() {
        let cats = default_categories();
        assert_eq!(get_category_for_bundle_id("com.apple.Safari", &cats), "Browsers");
    }

    #[test]
    fn test_get_category_for_bundle_id_not_found() {
        let cats = default_categories();
        assert_eq!(
            get_category_for_bundle_id("com.unknown.app", &cats),
            "Uncategorized"
        );
    }

    #[test]
    fn test_categorize_apps() {
        let cats = default_categories();
        let apps = vec![
            "com.apple.Safari".to_string(),
            "com.microsoft.VSCode".to_string(),
            "com.unknown.app".to_string(),
        ];
        
        let result = categorize_apps(&apps, &cats);
        
        assert_eq!(result.get("com.apple.Safari"), Some(&"Browsers".to_string()));
        assert_eq!(result.get("com.microsoft.VSCode"), Some(&"Development".to_string()));
        assert_eq!(result.get("com.unknown.app"), Some(&"Uncategorized".to_string()));
    }

    #[test]
    fn test_merge_categories_with_defaults() {
        let mut user = HashMap::new();
        user.insert("com.custom.app".to_string(), "Custom".to_string());
        user.insert("com.apple.Safari".to_string(), "Web".to_string()); // Override default
        
        let merged = merge_categories(&user, true);
        
        // User custom category
        assert_eq!(merged.get("com.custom.app"), Some(&"Custom".to_string()));
        // User override of default
        assert_eq!(merged.get("com.apple.Safari"), Some(&"Web".to_string()));
        // Default preserved
        assert_eq!(merged.get("com.google.Chrome"), Some(&"Browsers".to_string()));
    }

    #[test]
    fn test_merge_categories_without_defaults() {
        let mut user = HashMap::new();
        user.insert("com.custom.app".to_string(), "Custom".to_string());
        
        let merged = merge_categories(&user, false);
        
        assert_eq!(merged.len(), 1);
        assert_eq!(merged.get("com.custom.app"), Some(&"Custom".to_string()));
        assert!(merged.get("com.apple.Safari").is_none());
    }
}
