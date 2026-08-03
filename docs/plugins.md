# Plugin System & Extensions

The launcher uses a modular plugin architecture. Every plugin implements the `LauncherPlugin` trait defined in [src/plugin.rs](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/src/plugin.rs).

---

## The `LauncherPlugin` Trait

```rust
pub trait LauncherPlugin: Send + Sync {
    /// Unique identifier for the plugin (e.g., "calculator")
    fn id(&self) -> &str;

    /// Checks if the plugin should handle or intercept the current input query
    fn accepts(&self, query: &str) -> bool;

    /// Performs search/matching and returns ranked items
    fn query(&self, query: &str) -> Vec<SearchResult>;

    /// Executes the selected item action
    fn execute(&self, result_id: &str, shift_pressed: bool) -> ExecutionResult;
}
```

---

## Built-in Plugins

### 1. Applications (`src/plugins/applications.rs`)
Parses `.desktop` files in `/usr/share/applications` and `~/.local/share/applications`.
* **Features**:
  * Extracts display name, executable command, and icon path.
  * Filters out items flagged with `NoDisplay=true`.
  * Runs launch commands in detached child processes (`std::process::Command`).

### 2. Calculator (`src/plugins/calculator.rs`)
In-memory math evaluator.
* **Features**:
  * **Float Division**: Pre-processes queries (e.g., `3/2` -> `3.0/2.0`) to force floating-point division instead of integer division.
  * **Dynamic Formatting**: Formats results to look clean:
    * Whole numbers (e.g. `2.0`) -> `2`
    * 1 decimal place (e.g. `1.5`) -> `1.5`
    * 2+ decimal places (e.g. `1.3333`) -> `1.33` (rounded).
  * **German Keyword Translation**: Automatically translates natural-language inputs (e.g., `"mal"` to `*`, `"wurzel"` to `sqrt`, `"geteilt durch"` to `/`).
  * **Equation Solvers**: Detects coefficients for the quadratic formula (`abc`) or `pq`-formula and prints complex or real roots.

### 3. Clipboard (`src/plugins/clipboard.rs`)
Synchronizes clipboard state and logs history.
* **Features**:
  * Hooks into the GDK clipboard monitoring thread on startup.
  * Saves clipboard history locally to `~/.cache/launcher/clipboard_history.txt`.
  * Instantly copies the selected item back to the system clipboard on Enter.

### 4. System (`src/plugins/system.rs`)
Controls power management commands, process execution, and system utility lookups.
* **Features**:
  * Power commands: `lock`, `suspend`, `reboot`, `shutdown`.
  * **Process Killer**: Typing `kill <name>` (e.g., `kill firefox`) or `kill <port>` (e.g., `kill 5173`) finds the process using `killall` or `fuser -k` and terminates it.
  * **Gemini Shortcut**: Typing `gem` or `gemini` opens the default system browser to `https://gemini.google.com/app?hl=de`.
  * **Time & Date Utility**: Typing `time`, `zeit`, `date`, or `datum` returns the current local time (e.g. `13:25:44`) as the title, and the full date (e.g. `Sonntag, 14. Juni 2026`) as the description. Pressing Enter copies the formatted time to the system clipboard.

### 5. Web Search (`src/plugins/web.rs`)
Quick query redirection to search engines.
* **Features**:
  * Triggered by `g <query>` or `google <query>`.
  * Opens the default web browser with URL-encoded parameters.

### 6. Translator (`src/plugins/translator.rs`)
Translates text between German and English.
* **Features**:
  * Triggered by `t: <text>`, `tr <text>`, or `translate <text>`.
  * Hides the main launcher and displays a spinner state during lookup.
  * Uses a background worker thread calling Google Translate's free API endpoint.
  * Presents the result in a clean card displaying `Deutsch` and `Englisch` with an automated clipboard-copy feature.
  * Features a copy button that morphs into a checkmark icon (`object-select-symbolic`) for 1s.

### 7. Window Switcher (`src/plugins/window_switcher.rs`)
Switches between active windows under Hyprland.
* **Features**:
  * Reads the list of active window clients from Hyprland IPC sockets.
  * Focuses the selected window on activation.
