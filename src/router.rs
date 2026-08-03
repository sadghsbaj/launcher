use crate::frecency::FrecencyStore;
use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SearchRouter {
    plugins: Vec<Box<dyn LauncherPlugin>>,
    frecency: Arc<Mutex<FrecencyStore>>,
}

impl SearchRouter {
    pub fn new(frecency: Arc<Mutex<FrecencyStore>>, plugins: Vec<Box<dyn LauncherPlugin>>) -> Self {
        Self { plugins, frecency }
    }

    /// Queries all matching plugins, merges results, applies frecency boosts, and sorts them
    pub fn query(&self, query: &str) -> Vec<SearchResult> {
        let trimmed = query.trim();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut merged_results = Vec::new();
        let is_prefixed = trimmed.starts_with('!') || trimmed.starts_with('?');

        let frecency = self.frecency.lock().unwrap();

        for plugin in &self.plugins {
            // If the query is prefix-gated (like !g), we bypass normal plugin execution
            // unless the plugin specifically handles that prefix.
            if is_prefixed && plugin.id() != "web_search" {
                continue;
            }

            if plugin.accepts(trimmed) {
                let results = plugin.query(trimmed);
                for mut res in results {
                    // Fetch frecency score and apply a weight boost
                    let frecency_score = frecency.get_score(&res.id, now);
                    // Boost the match score (1 frecency unit = +1000 points boost)
                    res.score += (frecency_score * 1000.0) as i32;
                    merged_results.push(res);
                }
            }
        }

        // If no results matched locally, and the query is not empty, fallback to a Google search
        if merged_results.is_empty() && !trimmed.is_empty() {
            merged_results.push(SearchResult {
                id: format!("web:Google:{}", trimmed),
                title: format!("Search Google for '{}'", trimmed),
                description: Some("No local matches found. Search the web instead.".to_string()),
                icon: Some("google".to_string()),
                score: 100,
                last_used: None,
            });
        }

        // Sort by final score descending
        merged_results.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit results to top 15 to ensure speedy UI rendering and clean aesthetics
        merged_results.truncate(15);
        merged_results
    }

    /// Executes a result item by ID, registering frecency on success
    pub fn execute(&self, result_id: &str, shift_pressed: bool) -> ExecutionResult {
        // Find the plugin that owns this result
        // Result IDs are prefixed by plugin type (e.g. "calc:...", "sys:...", "clip:...", "win:...", "web:...")
        let plugin_id = if result_id.starts_with("app:") {
            "applications"
        } else if result_id.starts_with("calc:") {
            "calculator"
        } else if result_id.starts_with("sys:") {
            "system"
        } else if result_id.starts_with("clip:") {
            "clipboard"
        } else if result_id.starts_with("win:") {
            "window_switcher"
        } else if result_id.starts_with("web:") {
            "web_search"
        } else if result_id.starts_with("tr:") {
            "translator"
        } else {
            return ExecutionResult::Error("Unknown execution action prefix".to_string());
        };

        let plugin = match self.plugins.iter().find(|p| p.id() == plugin_id) {
            Some(p) => p,
            None => return ExecutionResult::Error(format!("Executor plugin '{}' not found", plugin_id)),
        };

        let result = plugin.execute(result_id, shift_pressed);

        // If execution succeeded and we closed the launcher, record frecency usage
        if result == ExecutionResult::CloseLauncher {
            let mut frecency = self.frecency.lock().unwrap();
            frecency.register_use(result_id);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::LauncherPlugin;

    struct MockPlugin {
        id: &'static str,
    }

    impl LauncherPlugin for MockPlugin {
        fn id(&self) -> &str {
            self.id
        }
        fn accepts(&self, query: &str) -> bool {
            query == "test"
        }
        fn query(&self, _query: &str) -> Vec<SearchResult> {
            vec![SearchResult {
                id: format!("{}:item", self.id),
                title: "Mock Title".to_string(),
                description: None,
                icon: None,
                score: 10,
                last_used: None,
            }]
        }
        fn execute(&self, _result_id: &str, _shift_pressed: bool) -> ExecutionResult {
            ExecutionResult::CloseLauncher
        }
    }

    #[test]
    fn test_router_query_routing() {
        let frecency = Arc::new(Mutex::new(FrecencyStore::default()));
        let plugins: Vec<Box<dyn LauncherPlugin>> = vec![
            Box::new(MockPlugin { id: "sys" }), // maps to system in execute prefix logic
        ];
        let router = SearchRouter::new(frecency, plugins);

        let results = router.query("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "sys:item");

        let empty_results = router.query("different");
        assert_eq!(empty_results.len(), 1);
        assert_eq!(empty_results[0].title, "Search Google for 'different'");
        assert!(empty_results[0].id.starts_with("web:Google:"));
    }
}
