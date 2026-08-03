# Pitfalls & Technical Challenges (Fallen)

This document catalogs the major technical hurdles, traps, and bugs encountered during development, along with their solutions.

---

## 1. GtkApplication Lifetime & Hidden Windows

### The Trap
When the user selects a search query to translate, we want to hide the main launcher window, perform the translation, and show the translation result window. Initially, we returned `ExecutionResult::CloseLauncher` which called `window.close()` on the main window. However, because the main window was the only window registered on the `GtkApplication`, destroying it caused the GTK main loop to exit immediately.

### The Fix
In [src/ui.rs](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/src/ui.rs), we intercept the translator plugin request and call `.hide()` on the main window instead of closing/destroying it. This keeps the GTK event loop alive in the background. The translator window then handles the keyboard focus and calls `std::process::exit(0)` upon being destroyed (via WM close, Close button, or Escape key) to clean up the background processes immediately.

---

## 2. "Text File Busy" Deployment Error

### The Trap
When deploying updates, running `cp target/release/launcher ~/.local/bin/launcher` failed with:
`cp: cannot create regular file '/home/colin/.local/bin/launcher': Text file busy`
This happened because the launcher binary was running in the background (active session) and Linux locks running binaries from being overwritten on disk.

### The Fix
In [deploy.sh](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/deploy.sh), we check if `launcher` is active using `pgrep`. If active, we issue a `killall` command and sleep for `0.5` seconds to release the file lock *before* copying the binary. We save the active state in a variable and launch the new binary in the background at the end of the script if it was running previously.

---

## 3. D-Bus Signal Subscription Garbage Collection

### The Trap
The Color Picker requests color picking from the XDG Desktop Portal via D-Bus and subscribes to the signal response. Initially, the subscription was dropped immediately at the end of the function. In Rust's GIO D-Bus bindings, dropping the subscription object automatically unsubscribes the callback. As a result, the launcher never received the picked color signal.

### The Fix
We stored the subscription in an `Rc<RefCell<Option<SignalSubscription>>>` that is cloned and captured by the callback closure. Inside the callback closure, the subscription is extracted from the cell and dropped. This keeps the subscription alive *precisely* until the D-Bus portal responds, at which point the subscription is cleaned up, avoiding both premature unsubscription and memory leaks.

---

## 4. Theme Icon Fallbacks & Monotone Icons

### The Trap
For the "Copied!" feedback state, we set the button icon to `"emblem-ok-symbolic"`. However, on standard Fedora GNOME systems running the default Adwaita icon theme, `emblem-ok-symbolic` is missing. GTK's fallback rules caused the icon to fall back to the button's default icon `"edit-copy-symbolic"` (which looks like a filled copy sheet icon), giving the illusion that the click handler didn't change the icon at all.

### The Fix
We scanned the standard Adwaita icon path `/usr/share/icons/Adwaita/` and discovered `"object-select-symbolic"`, which is the standard checkmark (Haken) icon in the GTK/Gnome ecosystem. Changing `"emblem-ok-symbolic"` to `"object-select-symbolic"` resolved the issue, and the copy buttons now morph into a clear checkmark.

---

## 5. Raw String Escapes (`r#"`)

### The Trap
In `src/config.rs`, writing a raw string literal to hold the default TOML template:
```rust
const DEFAULT_CONFIG_TEMPLATE: &str = r#"accent_color = "#ffffff" ... "#;
```
failed to compile. The sequence `"#` inside `"#ffffff"` was matched by the Rust parser as the termination sequence of the raw string, leaving the rest of the string as syntax errors.

### The Fix
We upgraded the raw string to use double-hash markers:
```rust
const DEFAULT_CONFIG_TEMPLATE: &str = r##"accent_color = "#ffffff" ... "##;
```
which only terminates at `"##`, safely ignoring `"#` within the string.

---

## 6. Integer Division in Math Evaluators

### The Trap
Typing `3/2` yielded `= 1` because `evalexpr` treats integer-on-integer division as integer division (truncating decimals).

### The Fix
We added a parser `convert_integers_to_floats` in [src/plugins/calculator.rs](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/src/plugins/calculator.rs) that scans the expression and appends `.0` to any integer sequence (e.g. `3` -> `3.0`), converting the query to float division before passing it to `evalexpr`.

---

## 7. Deep Learning Local C++ Dependencies

### The Trap
The user requested local offline translation. We initially integrated `trad` which depends on `ctranslate2` and `oneDNN`. Building these native C++ deep learning libraries failed on Fedora due to compilation and linker errors.

### The Fix
We fell back to Google Translate's free web API using a lightweight blocking client (`reqwest`). To keep the application responsive, we offloaded the network lookup to a background worker thread and used GLib local timeouts to periodically poll for the result. This delivers fast translations without bloating the compilation.
