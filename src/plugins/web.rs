use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use std::process::Command;

pub struct WebSearchPlugin {
    engines: Vec<SearchEngine>,
}

struct SearchEngine {
    prefix: &'static str,
    name: &'static str,
    url_template: &'static str,
    icon: &'static str,
}

impl WebSearchPlugin {
    pub fn new() -> Self {
        let engines = vec![
            SearchEngine {
                prefix: "!g",
                name: "Google",
                url_template: "https://www.google.com/search?q={}",
                icon: "google",
            },
            SearchEngine {
                prefix: "!ddg",
                name: "DuckDuckGo",
                url_template: "https://duckduckgo.com/?q={}",
                icon: "duckduckgo",
            },
            SearchEngine {
                prefix: "!w",
                name: "Wikipedia",
                url_template: "https://en.wikipedia.org/wiki/Special:Search?search={}",
                icon: "wikipedia",
            },
            SearchEngine {
                prefix: "!gh",
                name: "GitHub",
                url_template: "https://github.com/search?q={}",
                icon: "github",
            },
            // Fallback for general search
            SearchEngine {
                prefix: "?",
                name: "DuckDuckGo Search",
                url_template: "https://duckduckgo.com/?q={}",
                icon: "web-browser",
            },
        ];
        Self { engines }
    }
}

impl LauncherPlugin for WebSearchPlugin {
    fn id(&self) -> &str {
        "web_search"
    }

    fn accepts(&self, query: &str) -> bool {
        let q = query.trim_start();
        self.engines.iter().any(|eng| {
            q == eng.prefix || q.starts_with(&format!("{} ", eng.prefix))
        })
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim_start();
        let engine = match self.engines.iter().find(|eng| {
            q == eng.prefix || q.starts_with(&format!("{} ", eng.prefix))
        }) {
            Some(eng) => eng,
            None => return Vec::new(),
        };

        // Extract the actual search term
        let term = if q == engine.prefix {
            ""
        } else {
            &q[engine.prefix.len() + 1..]
        };

        if term.trim().is_empty() {
            return vec![SearchResult {
                id: format!("web:{}:", engine.name),
                title: format!("Search on {}", engine.name),
                description: Some(format!("Type query to search on {}", engine.name)),
                icon: Some(engine.icon.to_string()),
                score: 900, // Show web searches high up
                last_used: None,
            }];
        }

        vec![SearchResult {
            id: format!("web:{}:{}", engine.name, term),
            title: format!("Search {} for '{}'", engine.name, term.trim()),
            description: Some(format!("Opens default browser at {}", engine.name)),
            icon: Some(engine.icon.to_string()),
            score: 900,
            last_used: None,
        }]
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if !result_id.starts_with("web:") {
            return ExecutionResult::Error("Invalid web search action ID".to_string());
        }

        // Parse engine and search term
        let parts: Vec<&str> = result_id.splitn(3, ':').collect();
        if parts.len() < 3 {
            return ExecutionResult::KeepOpen;
        }

        let engine_name = parts[1];
        let term = parts[2];

        if term.trim().is_empty() {
            return ExecutionResult::KeepOpen;
        }

        let engine = match self.engines.iter().find(|eng| eng.name == engine_name) {
            Some(eng) => eng,
            None => return ExecutionResult::Error(format!("Unknown search engine: {}", engine_name)),
        };

        // Percent-encode the search query manually to avoid external crate dependencies
        let encoded_term: String = term
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                    (b as char).to_string()
                } else if b == b' ' {
                    "+".to_string()
                } else {
                    format!("%{:02X}", b)
                }
            })
            .collect();

        let url = engine.url_template.replace("{}", &encoded_term);

        match Command::new("xdg-open").arg(&url).spawn() {
            Ok(_) => ExecutionResult::CloseLauncher,
            Err(e) => ExecutionResult::Error(format!("Failed to open browser: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_accepts() {
        let plugin = WebSearchPlugin::new();
        assert!(plugin.accepts("!g rust docs"));
        assert!(plugin.accepts("? what is wayland"));
        assert!(plugin.accepts("!gh gtk-rs"));
        assert!(!plugin.accepts("google rust docs")); // no prefix
    }

    #[test]
    fn test_web_query() {
        let plugin = WebSearchPlugin::new();
        let results = plugin.query("!g modular launcher");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Search Google for 'modular launcher'");
        assert_eq!(results[0].id, "web:Google:modular launcher");
    }
}
