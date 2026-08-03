use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Deserialize;
use std::process::Command;

pub struct WindowSwitcherPlugin {
    matcher: SkimMatcherV2,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct HyprlandWorkspace {
    #[serde(default)]
    id: i32,
    #[serde(default)]
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct HyprlandClient {
    address: Option<String>,
    class: Option<String>,
    title: Option<String>,
    workspace: Option<HyprlandWorkspace>,
}

fn get_hyprland_signature() -> Option<String> {
    if let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if !sig.is_empty() {
            return Some(sig);
        }
    }
    
    // Fallback: search in $XDG_RUNTIME_DIR/hypr
    if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        let path = std::path::PathBuf::from(xdg_runtime).join("hypr");
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
    }

    // Fallback: search in /tmp/hypr
    let path = std::path::PathBuf::from("/tmp/hypr");
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }

    None
}

impl WindowSwitcherPlugin {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    fn get_hyprland_windows(&self) -> Result<Vec<HyprlandClient>, String> {
        #[cfg(test)]
        {
            return Err("Forced fallback to mock windows in tests".to_string());
        }

        #[cfg(not(test))]
        {
            let sig = get_hyprland_signature();
            let mut cmd = Command::new("hyprctl");
            cmd.args(&["clients", "-j"]);
            if let Some(ref s) = sig {
                cmd.env("HYPRLAND_INSTANCE_SIGNATURE", s);
            }
            let output = cmd.output()
                .map_err(|e| format!("Failed to run hyprctl: {}", e))?;

            if !output.status.success() {
                return Err("hyprctl command returned non-zero exit code".to_string());
            }

            let json_str = String::from_utf8(output.stdout)
                .map_err(|e| format!("Failed to parse hyprctl output as UTF-8: {}", e))?;

            let clients: Vec<HyprlandClient> = serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed to deserialize Hyprland JSON: {}", e))?;

            Ok(clients)
        }
    }

    /// Generates mock windows for testing and fallback on non-Hyprland environments
    fn get_mock_windows(&self) -> Vec<HyprlandClient> {
        vec![
            HyprlandClient {
                address: Some("0xmock1".to_string()),
                class: Some("firefox".to_string()),
                title: Some("Mozilla Firefox (Mock)".to_string()),
                workspace: Some(HyprlandWorkspace { id: 1, name: "1".to_string() }),
            },
            HyprlandClient {
                address: Some("0xmock2".to_string()),
                class: Some("kitty".to_string()),
                title: Some("Terminal - Cargo Watch (Mock)".to_string()),
                workspace: Some(HyprlandWorkspace { id: 2, name: "2".to_string() }),
            },
            HyprlandClient {
                address: Some("0xmock3".to_string()),
                class: Some("codium".to_string()),
                title: Some("Visual Studio Code - launcher (Mock)".to_string()),
                workspace: Some(HyprlandWorkspace { id: 3, name: "3".to_string() }),
            },
        ]
    }
}

impl LauncherPlugin for WindowSwitcherPlugin {
    fn id(&self) -> &str {
        "window_switcher"
    }

    fn accepts(&self, query: &str) -> bool {
        // Triggers if query starts with "w:" or if the query is empty (shows all windows)
        query.trim().starts_with("w:") || query.trim().is_empty()
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim();
        let search_term = if q.starts_with("w:") {
            q["w:".len()..].trim()
        } else {
            q
        };

        // Try to fetch real windows, fallback to mock if not in Hyprland
        let windows = match self.get_hyprland_windows() {
            Ok(wins) => wins,
            Err(_) => self.get_mock_windows(),
        };

        let mut results = Vec::new();

        for win in windows {
            let win_class = win.class.as_deref().unwrap_or("");
            let win_title = win.title.as_deref().unwrap_or("");
            let win_address = win.address.as_deref().unwrap_or("");

            let score = if search_term.is_empty() {
                50 // Base score for all open windows
            } else {
                let match_text = format!("{} {}", win_class, win_title);
                if let Some(score) = self.matcher.fuzzy_match(&match_text, search_term) {
                    score as i32
                } else {
                    continue;
                }
            };

            results.push(SearchResult {
                id: format!("win:{}", win_address),
                title: win_title.to_string(),
                description: None,
                icon: Some(win_class.to_lowercase()),
                score,
                last_used: None,
            });
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if let Some(address) = result_id.strip_prefix("win:") {
            if address.starts_with("0xmock") {
                // Mock execution success for development
                return ExecutionResult::CloseLauncher;
            }

            let sig = get_hyprland_signature();

            // Get target workspace ID if possible
            let mut target_ws: Option<i32> = None;
            if let Ok(windows) = self.get_hyprland_windows() {
                if let Some(win) = windows.iter().find(|w| w.address.as_deref() == Some(address)) {
                    if let Some(ref ws) = win.workspace {
                        target_ws = Some(ws.id);
                    }
                }
            }

            // Switch workspace first if it's different from the active workspace
            if let Some(ws_id) = target_ws {
                // Get active workspace
                let mut ws_cmd = Command::new("hyprctl");
                ws_cmd.args(["activeworkspace", "-j"]);
                if let Some(ref s) = sig {
                    ws_cmd.env("HYPRLAND_INSTANCE_SIGNATURE", s);
                }
                if let Ok(output) = ws_cmd.output() {
                    #[derive(Deserialize)]
                    struct ActiveWS { id: i32 }
                    if let Ok(active) = serde_json::from_slice::<ActiveWS>(&output.stdout) {
                        if active.id != ws_id {
                            let mut ws_switch_cmd = Command::new("hyprctl");
                            ws_switch_cmd.args(["dispatch", "workspace", &ws_id.to_string()]);
                            if let Some(ref s) = sig {
                                ws_switch_cmd.env("HYPRLAND_INSTANCE_SIGNATURE", s);
                            }
                            let _ = ws_switch_cmd.status();
                        }
                    }
                }
            }

            let mut focus_cmd = Command::new("hyprctl");
            focus_cmd.args(&["dispatch", "focuswindow", &format!("address:{}", address)]);
            if let Some(ref s) = sig {
                focus_cmd.env("HYPRLAND_INSTANCE_SIGNATURE", s);
            }
            match focus_cmd.spawn() {
                Ok(_) => ExecutionResult::CloseLauncher,
                Err(e) => ExecutionResult::Error(format!("Failed to focus window: {}", e)),
            }
        } else {
            ExecutionResult::Error("Invalid window action ID".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_switcher_accepts() {
        let plugin = WindowSwitcherPlugin::new();
        assert!(plugin.accepts("w:"));
        assert!(plugin.accepts("w:firefox"));
        assert!(plugin.accepts(""));
    }

    #[test]
    fn test_window_switcher_query() {
        let plugin = WindowSwitcherPlugin::new();
        let results = plugin.query("w:firefox");
        assert!(results.len() >= 1);
        assert!(results[0].title.contains("Firefox"));
        assert!(results[0].id.starts_with("win:"));
    }
}
