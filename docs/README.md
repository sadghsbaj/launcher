# Wayland Launcher Documentation Index

Welcome to the documentation for the Wayland Launcher. This directory contains detailed specifications, design decisions, implementation details, and pitfalls encountered during the development of this monotone, glassmorphic keyboard-driven launcher.

## Documentation Map

1. **[System Architecture](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/docs/architecture.md)**
   * Learn about the main execution loop (GTK4 + Layer Shell), asynchronous channel communication, fuzzy routing, and lifecycles.
2. **[Plugin Guide](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/docs/plugins.md)**
   * Technical guide on the standard plugins including Applications, Calculator, Clipboard, System power commands, Web Search, Google Translate, and Window Switcher.
3. **[Theming & CSS System](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/docs/theming.md)**
   * Explanation of the config parsing system, self-healing defaults, dynamic GTK CSS compilation, and Wayland compositor blur matching.
4. **[Pitfalls & Lessons Learned (Fallen)](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/docs/pitfalls.md)**
   * Critical review of technical challenges solved (process leaks, D-Bus signal subscription lifetimes, missing GTK theme icons, and raw string escapes).

---

## Directory Overview

* **`src/main.rs`**: Application entry point initializing shared data stores (Frecency database, Clipboard cache), loading configurations, registering plugins, and launching the UI.
* **`src/ui.rs`**: GTK4 window initialization, event controller bindings (e.g., Escape to close, arrow keys to navigate), search results rendering, and dynamic CSS styling.
* **`src/config.rs`**: Holds the strongly-typed `Config` struct, default values, paths resolution, self-healing creation, and dynamic CSS template compiler.
* **`src/router.rs`**: Implements the matching logic using `fuzzy-matcher` and decay-based Frecency database scores to rank search results.
* **`src/plugin.rs`**: Declares the `LauncherPlugin` trait defining standard methods each extension must implement.
* **`src/plugins/`**: Directory containing individual modular plugins.
* **`deploy.sh`**: Installs release build to `~/.local/bin/launcher`, manages configuration migrations, and gracefully restarts active instances.
