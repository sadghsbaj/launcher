use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const HALF_LIFE_SECONDS: f64 = 7.0 * 24.0 * 60.0 * 60.0; // 7 days decay half-life
const LAMBDA: f64 = 0.69314718056 / HALF_LIFE_SECONDS; // ln(2) / half_life

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FrecencyStore {
    // Maps a result ID to a list of UNIX timestamps (seconds) when it was executed
    pub launches: HashMap<String, Vec<u64>>,
}

impl FrecencyStore {
    /// Gets the path to the frecency JSON store
    fn store_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let mut path = PathBuf::from(home);
        path.push(".cache");
        path.push("wayland-launcher");
        path.push("frecency.json");
        path
    }

    /// Loads the frecency store from disk
    pub fn load() -> Self {
        let path = Self::store_path();
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Saves the frecency store to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::store_path();
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create config dir: {}", e));
            }
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize frecency data: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write frecency file: {}", e))?;

        Ok(())
    }

    /// Registers a launch occurrence for a result ID and saves the updated store
    pub fn register_use(&mut self, id: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entries = self.launches.entry(id.to_string()).or_default();
        entries.push(now);

        // Keep only the last 50 activations per key to avoid file size bloat
        if entries.len() > 50 {
            entries.drain(0..entries.len() - 50);
        }

        let _ = self.save();
    }

    /// Calculates the current frecency score for a result ID
    pub fn get_score(&self, id: &str, now: u64) -> f64 {
        let entries = match self.launches.get(id) {
            Some(entries) => entries,
            None => return 0.0,
        };

        let mut total_score = 0.0;
        for &timestamp in entries {
            if now >= timestamp {
                let dt = (now - timestamp) as f64;
                // e^(-lambda * dt)
                total_score += (-LAMBDA * dt).exp();
            } else {
                // If timestamp is somehow in the future, count it as a full weight of 1.0
                total_score += 1.0;
            }
        }
        total_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frecency_decay() {
        let mut store = FrecencyStore::default();
        let id = "test_action";
        let now = 1000000;

        // Add a launch right now
        store.launches.entry(id.to_string()).or_default().push(now);
        assert!((store.get_score(id, now) - 1.0).abs() < 1e-5);

        // Add a launch one half-life ago
        let half_life_ago = now - HALF_LIFE_SECONDS as u64;
        store.launches.entry(id.to_string()).or_default().push(half_life_ago);
        
        // Combined score should be roughly 1.5 (1.0 for now, 0.5 for half life ago)
        let combined_score = store.get_score(id, now);
        assert!((combined_score - 1.5).abs() < 1e-3, "Expected 1.5, got {}", combined_score);
    }
}
