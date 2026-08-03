use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ApplicationsPlugin {
    apps: Vec<DesktopApp>,
    matcher: SkimMatcherV2,
}

#[derive(Clone, Debug)]
struct DesktopApp {
    id: String,
    name: String,
    exec: String,
    icon: Option<String>,
}

impl ApplicationsPlugin {
    pub fn new() -> Self {
        let mut apps = Vec::new();
        let matcher = SkimMatcherV2::default();

        // Scan standard Linux desktop entry directories
        let mut dirs = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/var/lib/flatpak/exports/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
        ];

        if let Ok(home) = std::env::var("HOME") {
            let mut local_dir = PathBuf::from(&home);
            local_dir.push(".local/share/applications");
            dirs.push(local_dir);

            let mut flatpak_local = PathBuf::from(&home);
            flatpak_local.push(".local/share/flatpak/exports/share/applications");
            dirs.push(flatpak_local);
        }

        for dir in dirs {
            if dir.exists() {
                Self::scan_directory(&dir, &mut apps);
            }
        }

        // Deduplicate applications by ID (filename) so user-local overrides take precedence
        apps.sort_by(|a, b| a.id.cmp(&b.id));
        apps.dedup_by(|a, b| a.id == b.id);

        Self { apps, matcher }
    }

    fn scan_directory(dir: &Path, apps: &mut Vec<DesktopApp>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "desktop") {
                    if let Ok(app) = Self::parse_desktop_file(&path) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    /// Parses a standard .desktop file to extract name, exec, icon, and visibility
    fn parse_desktop_file(path: &Path) -> Result<DesktopApp, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut name = String::new();
        let mut exec = String::new();
        let mut icon = None;
        let mut no_display = false;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_desktop_entry = trimmed == "[Desktop Entry]";
            }
            if !in_desktop_entry {
                continue;
            }

            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim();
                let val = trimmed[pos + 1..].trim();
                match key {
                    "Name" => {
                        if name.is_empty() {
                            name = val.to_string();
                        }
                    }
                    "Exec" => {
                        exec = val.to_string();
                    }
                    "Icon" => {
                        icon = Some(val.to_string());
                    }
                    "NoDisplay" => {
                        if val.to_lowercase() == "true" {
                            no_display = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() || exec.is_empty() || no_display {
            return Err("Incomplete or hidden desktop entry".to_string());
        }

        let id = path.file_name().unwrap().to_string_lossy().to_string();

        Ok(DesktopApp {
            id,
            name,
            exec,
            icon,
        })
    }

    /// Cleans the Exec command string of field codes (%u, %F, etc.)
    fn clean_exec_command(exec: &str) -> String {
        let mut cleaned = String::new();
        for word in exec.split_whitespace() {
            // Skip placeholders like %u, %U, %f, %F, %i, %c, %k
            if word.starts_with('%') {
                continue;
            }
            if !cleaned.is_empty() {
                cleaned.push(' ');
            }
            cleaned.push_str(word);
        }
        cleaned
    }
}


impl LauncherPlugin for ApplicationsPlugin {
    fn id(&self) -> &str {
        "applications"
    }

    fn accepts(&self, query: &str) -> bool {
        let trimmed = query.trim();

        // Avoid matching system execution commands or math equations
        let q_lower = trimmed.to_lowercase();
        if q_lower.starts_with("sys:") || q_lower.starts_with("c:") || q_lower.starts_with("clip:") || q_lower.starts_with("g:") || q_lower.starts_with("google:") || q_lower.starts_with("web:") || q_lower.starts_with("win:") {
            return false;
        }

        // Let standard evaluation run for math
        if q_lower.contains("mitternacht") || q_lower.contains("quadratic") || q_lower.contains("abc-") || q_lower.contains("pq-") || q_lower.starts_with("pq") {
            return false;
        }

        let has_operators = trimmed.chars().any(|c| {
            c == '+' || c == '*' || c == '/' || c == '^' || c == '%' || c == '(' || (c == '-' && trimmed.len() > 1)
        });
        if has_operators {
            return false;
        }

        // An empty query is accepted to return baseline list of apps on startup
        if trimmed.is_empty() {
            return true;
        }

        // Search in desktop files cache
        self.apps.iter().any(|app| {
            self.matcher.fuzzy_match(&app.name, trimmed).is_some() ||
            self.matcher.fuzzy_match(&app.id, trimmed).is_some()
        })
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let trimmed = query.trim();
        if !self.accepts(trimmed) {
            return Vec::new();
        }

        let mut results = Vec::new();
        for app in &self.apps {
            let score = if trimmed.is_empty() {
                50 // Baseline score when empty
            } else {
                let score_name = self.matcher.fuzzy_match(&app.name, trimmed).unwrap_or(0) as i32;
                let score_id = self.matcher.fuzzy_match(&app.id, trimmed).unwrap_or(0) as i32;
                std::cmp::max(score_name, score_id)
            };

            if score == 0 {
                continue;
            }

            results.push(SearchResult {
                id: format!("app:{}", app.id),
                title: app.name.clone(),
                description: None, // Simplified look, no tech subtext as requested
                icon: app.icon.clone(),
                score,
                last_used: None,
            });
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if let Some(app_id) = result_id.strip_prefix("app:") {
            if let Some(app) = self.apps.iter().find(|a| a.id == app_id) {
                let cleaned_exec = Self::clean_exec_command(&app.exec);
                let parts: Vec<String> = cleaned_exec.split_whitespace().map(|s| s.to_string()).collect();

                if parts.is_empty() {
                    return ExecutionResult::Error("Empty exec command after cleaning".to_string());
                }

                // Spawn a new application instance in the background immediately
                let mut cmd = Command::new(&parts[0]);
                if parts.len() > 1 {
                    cmd.args(&parts[1..]);
                }

                match cmd.spawn() {
                    Ok(_) => ExecutionResult::CloseLauncher,
                    Err(e) => ExecutionResult::Error(format!("Failed to start application: {}", e)),
                }
            } else {
                ExecutionResult::Error("Application ID not found in cache".to_string())
            }
        } else {
            ExecutionResult::Error("Invalid application action ID".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_exec_command() {
        assert_eq!(ApplicationsPlugin::clean_exec_command("firefox %u"), "firefox");
        assert_eq!(
            ApplicationsPlugin::clean_exec_command("gimp-2.10 %F"),
            "gimp-2.10"
        );
        assert_eq!(
            ApplicationsPlugin::clean_exec_command("steam -tenfoot %U"),
            "steam -tenfoot"
        );
    }

    #[test]
    fn test_applications_query() {
        let plugin = ApplicationsPlugin {
            apps: vec![
                DesktopApp {
                    id: "firefox.desktop".to_string(),
                    name: "Firefox Web Browser".to_string(),
                    exec: "firefox".to_string(),
                    icon: None,
                },
                DesktopApp {
                    id: "kitty.desktop".to_string(),
                    name: "Kitty Terminal".to_string(),
                    exec: "kitty".to_string(),
                    icon: None,
                },
            ],
            matcher: SkimMatcherV2::default(),
        };

        // Gating test
        assert!(plugin.accepts("fire"));
        assert!(!plugin.accepts("!g fire"));

        // Match query
        let results = plugin.query("fire");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Firefox Web Browser");
        assert_eq!(results[0].id, "app:firefox.desktop");

        // Empty query should return all apps
        let results_empty = plugin.query("");
        assert_eq!(results_empty.len(), 2);
    }
}
