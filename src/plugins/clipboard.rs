use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use gtk::gdk;
use gtk::prelude::*;
use std::sync::{Arc, Mutex};



pub struct ClipboardPlugin {
    history: Arc<Mutex<Vec<String>>>,
    matcher: SkimMatcherV2,
}

impl ClipboardPlugin {
    pub fn new(history: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            history,
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl LauncherPlugin for ClipboardPlugin {
    fn id(&self) -> &str {
        "clipboard"
    }

    fn accepts(&self, query: &str) -> bool {
        // Only accept if explicitly gated by "c:" or "clip:" prefix
        let trimmed = query.trim();
        trimmed.starts_with("c:") || trimmed.starts_with("clip:")
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim();
        
        // Strip prefix if present
        let search_term = if q.starts_with("clip:") {
            q["clip:".len()..].trim()
        } else if q.starts_with("c:") {
            q["c:".len()..].trim()
        } else {
            q
        };

        let history = self.history.lock().unwrap();
        
        let mut results = Vec::new();
        for (idx, item) in history.iter().rev().enumerate() {
            let truncated_title = if item.len() > 50 {
                format!("{}...", &item[..47])
            } else {
                item.clone()
            };

            let score = if search_term.is_empty() {
                // Return in reverse chronological order
                100 - idx as i32
            } else if let Some(score) = self.matcher.fuzzy_match(item, search_term) {
                score as i32
            } else {
                continue;
            };

            results.push(SearchResult {
                id: format!("clip:{}", idx), // Store index in reverse order to map back to original index
                title: truncated_title,
                description: Some(format!("Clipboard: {} chars", item.len())),
                icon: Some("edit-copy".to_string()),
                score,
                last_used: None,
            });
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if let Some(index_str) = result_id.strip_prefix("clip:") {
            let idx = match index_str.parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return ExecutionResult::Error("Invalid clipboard index".to_string()),
            };

            let history = self.history.lock().unwrap();
            // Since we queried in reverse order (newest first), the index corresponds to:
            // history.len() - 1 - idx
            if idx < history.len() {
                let actual_idx = history.len() - 1 - idx;
                let text = history[actual_idx].clone();

                let child = std::process::Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn();
                
                let mut success = false;
                if let Ok(mut child) = child {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        if stdin.write_all(text.as_bytes()).is_ok() {
                            drop(stdin);
                            if let Ok(status) = child.wait() {
                                if status.success() {
                                    success = true;
                                }
                            }
                        }
                    }
                }

                if !success {
                    if let Some(display) = gdk::Display::default() {
                        let clipboard = display.clipboard();
                        clipboard.set_text(&text);
                    }
                }
                ExecutionResult::CloseLauncher
            } else {
                ExecutionResult::Error("Clipboard history item not found".to_string())
            }
        } else {
            ExecutionResult::Error("Invalid clipboard action ID".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_query_empty() {
        let history = Arc::new(Mutex::new(vec![
            "first copy".to_string(),
            "second copy".to_string(),
        ]));
        let plugin = ClipboardPlugin::new(history);

        // Empty query should return both items, newest first
        let results = plugin.query("");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "second copy");
        assert_eq!(results[1].title, "first copy");
    }

    #[test]
    fn test_clipboard_query_filter() {
        let history = Arc::new(Mutex::new(vec![
            "apple item".to_string(),
            "banana item".to_string(),
        ]));
        let plugin = ClipboardPlugin::new(history);

        let results = plugin.query("banana");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "banana item");
    }
}
