use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use std::process::Command;
use gtk::gdk;
use gtk::prelude::*;

pub struct SystemPlugin {
    commands: Vec<SystemCommand>,
}

#[derive(Clone)]
struct SystemCommand {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    exec: &'static [&'static str],
}

impl SystemPlugin {
    pub fn new() -> Self {
        let commands = vec![
            SystemCommand {
                id: "sys:lock",
                name: "Lock Screen",
                description: "Lock the current session via loginctl",
                icon: "system-lock-screen",
                exec: &["loginctl", "lock-session"],
            },
            SystemCommand {
                id: "sys:picker",
                name: "Color Picker",
                description: "Pick a color from the screen using XDG Portal",
                icon: "color-picker",
                exec: &[],
            },
            SystemCommand {
                id: "sys:suspend",
                name: "Suspend",
                description: "Suspend the system to RAM",
                icon: "system-suspend",
                exec: &["systemctl", "suspend"],
            },
            SystemCommand {
                id: "sys:reboot",
                name: "Reboot",
                description: "Restart the system",
                icon: "system-reboot",
                exec: &["systemctl", "reboot"],
            },
            SystemCommand {
                id: "sys:shutdown",
                name: "Shutdown",
                description: "Power off the system",
                icon: "system-shutdown",
                exec: &["systemctl", "poweroff"],
            },
        ];
        Self { commands }
    }
}

impl SystemPlugin {
    fn query_kill_targets(&self, target: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // Check if target is a number (could be port or PID)
        if let Ok(num) = target.parse::<u32>() {
            // 1. Check if it's a port
            if let Some(pids) = get_pids_for_port(num) {
                for pid in pids {
                    let comm = get_process_name(pid).unwrap_or_else(|| "unknown".to_string());
                    results.push(SearchResult {
                        id: format!("sys:kill:port:{}:{}", num, pid),
                        title: format!("Kill '{}' (PID {})", comm, pid),
                        description: Some(format!("Listening on port {}", num)),
                        icon: Some("process-stop".to_string()),
                        score: 100,
                        last_used: None,
                    });
                }
            }

            // 2. Check if it's a running PID
            if let Some(comm) = get_process_name(num) {
                results.push(SearchResult {
                    id: format!("sys:kill:pid:{}", num),
                    title: format!("Kill '{}' (PID {})", comm, num),
                    description: Some(format!("Process ID {}", num)),
                    icon: Some("process-stop".to_string()),
                    score: 95,
                    last_used: None,
                });
            }
        }

        // 3. Search processes by name matching the target
        let name_matches = find_processes_by_name(target);
        if !name_matches.is_empty() {
            // Suggest killing all processes matching the name
            results.push(SearchResult {
                id: format!("sys:kill:all:{}", target),
                title: format!("Kill all '{}' processes", target),
                description: Some(format!("Terminate all processes with names containing '{}'", target)),
                icon: Some("process-stop".to_string()),
                score: 92,
                last_used: None,
            });

            // List individual processes
            for (pid, comm) in name_matches.iter().take(10) {
                let already_added = results.iter().any(|r| r.id == format!("sys:kill:pid:{}", pid) || r.id.ends_with(&format!(":{}", pid)));
                if !already_added {
                    results.push(SearchResult {
                        id: format!("sys:kill:name:{}:{}", comm, pid),
                        title: format!("Kill '{}' (PID {})", comm, pid),
                        description: Some(format!("Process ID {}", pid)),
                        icon: Some("process-stop".to_string()),
                        score: 88,
                        last_used: None,
                    });
                }
            }
        }

        results
    }
}

impl LauncherPlugin for SystemPlugin {
    fn id(&self) -> &str {
        "system"
    }

    fn accepts(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }

        if q.starts_with("kill") {
            return true;
        }

        // Accepts gemini prefixes (gem, gemi, gemin, gemini)
        if "gemini".starts_with(&q) && q.len() >= 3 {
            return true;
        }

        // Accepts time/date queries (time, zeit, date, datum)
        if ("time".starts_with(&q) && q.len() >= 3)
            || ("zeit".starts_with(&q) && q.len() >= 3)
            || ("date".starts_with(&q) && q.len() >= 3)
            || ("datum".starts_with(&q) && q.len() >= 3)
        {
            return true;
        }

        // Accepts if it matches or prefixes any system command keywords
        self.commands.iter().any(|cmd| {
            cmd.name.to_lowercase().contains(&q) || cmd.id.to_lowercase().contains(&q)
        })
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        // 1. Handle "gemini" query
        if "gemini".starts_with(&q) && q.len() >= 3 {
            results.push(SearchResult {
                id: "sys:gemini".to_string(),
                title: "Open Gemini".to_string(),
                description: Some("Open https://gemini.google.com/app?hl=de".to_string()),
                icon: Some("google".to_string()),
                score: 1000, // Very high score so it's the first option!
                last_used: None,
            });
        }

        // 2. Handle "kill" queries
        if q == "kill" {
            results.push(SearchResult {
                id: "sys:kill_help".to_string(),
                title: "Kill a process".to_string(),
                description: Some("Type 'kill <name>', 'kill <port>', or 'kill <PID>'".to_string()),
                icon: Some("process-stop".to_string()),
                score: 95,
                last_used: None,
            });
        } else if q.starts_with("kill ") {
            let target = query.chars().skip(5).collect::<String>();
            let target_trimmed = target.trim();
            if !target_trimmed.is_empty() {
                results.extend(self.query_kill_targets(target_trimmed));
            }
        }

        // 3. Handle "time" queries
        let is_time_query = ("time".starts_with(&q) && q.len() >= 3)
            || ("zeit".starts_with(&q) && q.len() >= 3)
            || ("date".starts_with(&q) && q.len() >= 3)
            || ("datum".starts_with(&q) && q.len() >= 3);

        if is_time_query {
            let (time_str, date_str) = get_current_time_and_date();
            results.push(SearchResult {
                id: format!("sys:time:{}", time_str),
                title: time_str,
                description: Some(date_str),
                icon: Some("preferences-system-time".to_string()),
                score: 1000,
                last_used: None,
            });
        }

        // 4. Handle standard system commands
        let standard_commands: Vec<SearchResult> = self.commands
            .iter()
            .filter(|cmd| {
                cmd.name.to_lowercase().contains(&q) || cmd.id.to_lowercase().contains(&q)
            })
            .map(|cmd| SearchResult {
                id: cmd.id.to_string(),
                title: cmd.name.to_string(),
                description: Some(cmd.description.to_string()),
                icon: Some(cmd.icon.to_string()),
                score: 80, // High-ish base score for system commands
                last_used: None,
            })
            .collect();
        results.extend(standard_commands);

        results
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if result_id == "sys:picker" {
            return ExecutionResult::CloseLauncher;
        }

        if result_id == "sys:gemini" {
            match Command::new("xdg-open").arg("https://gemini.google.com/app?hl=de").spawn() {
                Ok(_) => return ExecutionResult::CloseLauncher,
                Err(e) => return ExecutionResult::Error(format!("Failed to open browser: {}", e)),
            }
        }

        if result_id.starts_with("sys:time:") {
            if let Some(time_val) = result_id.strip_prefix("sys:time:") {
                let mut success = false;
                let child = Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn();
                if let Ok(mut child) = child {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        if stdin.write_all(time_val.as_bytes()).is_ok() {
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
                        clipboard.set_text(time_val);
                    }
                }
            }
            return ExecutionResult::CloseLauncher;
        }

        if result_id == "sys:kill_help" {
            return ExecutionResult::KeepOpen;
        }

        if result_id.starts_with("sys:kill:") {
            let parts: Vec<&str> = result_id.split(':').collect();
            if parts.len() >= 4 && parts[2] == "all" {
                // sys:kill:all:<target>
                let target = parts[3..].join(":");
                match kill_all_processes(&target) {
                    Ok(_) => return ExecutionResult::CloseLauncher,
                    Err(e) => return ExecutionResult::Error(format!("Failed to kill processes: {}", e)),
                }
            } else if parts.len() >= 4 {
                // sys:kill:port:<port>:<pid> or sys:kill:name:<comm>:<pid>
                let pid_str = parts.last().unwrap();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    match kill_process(pid) {
                        Ok(_) => return ExecutionResult::CloseLauncher,
                        Err(e) => return ExecutionResult::Error(format!("Failed to kill process {}: {}", pid, e)),
                    }
                }
            } else if parts.len() == 3 {
                // sys:kill:pid:<pid>
                if let Ok(pid) = parts[2].parse::<u32>() {
                    match kill_process(pid) {
                        Ok(_) => return ExecutionResult::CloseLauncher,
                        Err(e) => return ExecutionResult::Error(format!("Failed to kill process {}: {}", pid, e)),
                    }
                }
            }
            return ExecutionResult::Error("Invalid kill command format".to_string());
        }

        if let Some(cmd) = self.commands.iter().find(|c| c.id == result_id) {
            if cmd.exec.is_empty() {
                return ExecutionResult::Error("No execute arguments provided".to_string());
            }

            match Command::new(cmd.exec[0]).args(&cmd.exec[1..]).spawn() {
                Ok(_) => ExecutionResult::CloseLauncher,
                Err(e) => ExecutionResult::Error(format!("Failed to execute command: {}", e)),
            }
        } else {
            ExecutionResult::Error("Invalid system command ID".to_string())
        }
    }
}

fn get_current_time_and_date() -> (String, String) {
    let time_out = Command::new("date").arg("+%H:%M:%S").output();
    let date_out = Command::new("date").arg("+%A, %d. %B %Y").output();

    let time_str = match time_out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "00:00:00".to_string(),
    };

    let date_str = match date_out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "Date unavailable".to_string(),
    };

    (time_str, date_str)
}

// --- Helper Functions for Process Management ---

fn get_pids_for_port(port: u32) -> Option<Vec<u32>> {
    let output = Command::new("lsof")
        .args(&["-t", "-n", "-P", &format!("-i:{}", port)])
        .output()
        .ok()?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();
        for line in stdout.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                pids.push(pid);
            }
        }
        if !pids.is_empty() {
            return Some(pids);
        }
    }
    None
}

fn get_process_name(pid: u32) -> Option<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(&comm_path)
        .map(|s| s.trim().to_string())
        .ok()
}

fn find_processes_by_name(name_query: &str) -> Vec<(u32, String)> {
    let mut results = Vec::new();
    let query = name_query.to_lowercase();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if let Ok(pid) = name_str.parse::<u32>() {
                        if pid == std::process::id() {
                            continue;
                        }
                        if let Some(comm) = get_process_name(pid) {
                            if comm.to_lowercase().contains(&query) {
                                results.push((pid, comm));
                            }
                        }
                    }
                }
            }
        }
    }
    results.sort_by_key(|r| r.0);
    results
}

fn kill_process(pid: u32) -> std::io::Result<()> {
    Command::new("kill")
        .args(&["-9", &pid.to_string()])
        .spawn()?
        .wait()?;
    Ok(())
}

fn kill_all_processes(name: &str) -> std::io::Result<()> {
    Command::new("pkill")
        .args(&["-9", "-f", name])
        .spawn()?
        .wait()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_accepts() {
        let plugin = SystemPlugin::new();
        assert!(plugin.accepts("lock"));
        assert!(plugin.accepts("reboot"));
        assert!(plugin.accepts("shut"));
        assert!(!plugin.accepts("random_word"));
        assert!(plugin.accepts("kill"));
        assert!(plugin.accepts("kill brave"));
        assert!(plugin.accepts("gem"));
        assert!(plugin.accepts("gemini"));
        assert!(plugin.accepts("time"));
        assert!(plugin.accepts("zeit"));
    }

    #[test]
    fn test_system_query() {
        let plugin = SystemPlugin::new();
        let results = plugin.query("reboot");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Reboot");
        assert_eq!(results[0].id, "sys:reboot");

        let kill_help = plugin.query("kill");
        assert_eq!(kill_help.len(), 1);
        assert_eq!(kill_help[0].id, "sys:kill_help");

        let gemini_res = plugin.query("gem");
        assert_eq!(gemini_res.len(), 1);
        assert_eq!(gemini_res[0].id, "sys:gemini");

        let time_res = plugin.query("time");
        assert_eq!(time_res.len(), 1);
        assert!(time_res[0].id.starts_with("sys:time:"));
    }
}
