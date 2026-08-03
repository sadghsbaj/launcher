use crate::plugin::{ExecutionResult, SearchResult};
use crate::router::SearchRouter;
use gtk::prelude::*;
use gtk::{gdk, glib, Application, ApplicationWindow, Box as GtkBox, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Image};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

pub fn run_ui(router: Arc<SearchRouter>, clipboard_history: Arc<Mutex<Vec<String>>>, config: crate::config::Config) {
    let app = Application::builder()
        .application_id("com.colin.wayland-launcher")
        .build();

    let config_clone = config.clone();
    app.connect_activate(move |app| {
        build_ui(app, router.clone(), clipboard_history.clone(), config_clone.clone());
    });

    app.run();
}

fn build_ui(app: &Application, router: Arc<SearchRouter>, clipboard_history: Arc<Mutex<Vec<String>>>, config: crate::config::Config) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Wayland Launcher")
        .build();
    window.add_css_class("launcher-window");

    // Initialize Wayland Layer Shell
    window.init_layer_shell();
    window.set_namespace(Some("launcher"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_decorated(false); // Removes titlebar (toolbar) and window borders

    // Center on screen (setting all anchors to true makes the window fullscreen, allowing GTK to center the main box dynamically)
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, true);

    // Set width and height (larger to account for 40px left/right shadow margins and 30px/50px top/bottom margins)
    window.set_default_size(680, 460);

    // Load custom CSS style sheet
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&crate::config::generate_css(&config));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    // Load persistent history on startup and update with current system clipboard
    let mut initial_history = load_clipboard_history();
    if let Some(current) = get_current_clipboard() {
        if initial_history.last() != Some(&current) {
            initial_history.retain(|x| x != &current);
            initial_history.push(current);
            if initial_history.len() > 100 {
                initial_history.remove(0);
            }
            save_clipboard_history(&initial_history);
        }
    }
    {
        let mut hist = clipboard_history.lock().unwrap();
        *hist = initial_history;
    }

    // Set up GDK clipboard monitoring on main thread
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();
        let history_clone = clipboard_history.clone();
        
        clipboard.connect_changed(move |cb| {
            let hist = history_clone.clone();
            cb.read_text_async(None::<&gtk::gio::Cancellable>, move |result| {
                if let Ok(Some(text)) = result {
                    let text_str = text.to_string();
                    if text_str.trim().is_empty() {
                        return;
                    }
                    let mut h = hist.lock().unwrap();
                    // Avoid duplicating the absolute latest entry
                    if h.last() != Some(&text_str) {
                        h.retain(|x| x != &text_str);
                        h.push(text_str);
                        if h.len() > 100 {
                            h.remove(0);
                        }
                        save_clipboard_history(&h);
                    }
                }
            });
        });
    }

    // Main layout container with fixed size request so it never shrinks or grows
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.add_css_class("launcher-box");
    main_box.set_size_request(600, 380);
    main_box.set_halign(gtk::Align::Center);
    main_box.set_valign(gtk::Align::Center);

    // Search input field
    let search_entry = Entry::builder()
        .placeholder_text("Search...")
        .has_frame(false)
        .build();
    search_entry.add_css_class("search-entry");
    main_box.append(&search_entry);

    // Search results list
    let list_box = ListBox::new();
    list_box.add_css_class("results-list");

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_height(300)
        .max_content_height(300)
        .propagate_natural_height(false)
        .vexpand(true)
        .build();
    scrolled_window.add_css_class("results-scroll");
    scrolled_window.set_child(Some(&list_box));
    main_box.append(&scrolled_window);

    window.set_child(Some(&main_box));

    // Shared state for the active query results
    let current_results = Arc::new(Mutex::new(Vec::<SearchResult>::new()));

    // Key event controller for custom navigation
    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let window_clone = window.clone();
    let list_box_clone = list_box.clone();
    let current_results_clone = current_results.clone();
    let router_clone = router.clone();
    let scrolled_window_clone = scrolled_window.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, state| {
        let shift_pressed = state.contains(gdk::ModifierType::SHIFT_MASK);
        match keyval {
            gdk::Key::Escape => {
                window_clone.close();
                glib::Propagation::Stop
            }
            gdk::Key::Down => {
                let row_count = list_box_clone.row_at_index(0).is_some();
                if row_count {
                    let current_selected = list_box_clone.selected_row();
                    let next_index = match &current_selected {
                        Some(row) => row.index() + 1,
                        None => 0,
                    };
                    if let Some(next_row) = list_box_clone.row_at_index(next_index) {
                        list_box_clone.select_row(Some(&next_row));
                        scroll_to_row(&list_box_clone, &scrolled_window_clone, &next_row);
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Up => {
                if let Some(row) = list_box_clone.selected_row() {
                    let prev_index = row.index() - 1;
                    if prev_index >= 0 {
                        if let Some(prev_row) = list_box_clone.row_at_index(prev_index) {
                            list_box_clone.select_row(Some(&prev_row));
                            scroll_to_row(&list_box_clone, &scrolled_window_clone, &prev_row);
                        }
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Return => {
                if let Some(row) = list_box_clone.selected_row() {
                    let idx = row.index() as usize;
                    let results = current_results_clone.lock().unwrap();
                    if idx < results.len() {
                        let result_id = &results[idx].id;
                        if result_id == "sys:picker" {
                            window_clone.hide();
                            crate::color_picker::pick_color_and_show_ui(window_clone.clone());
                        } else if result_id.starts_with("tr:") {
                            window_clone.hide();
                            let _ = router_clone.execute(result_id, shift_pressed);
                        } else {
                            match router_clone.execute(result_id, shift_pressed) {
                                ExecutionResult::CloseLauncher => {
                                    window_clone.close();
                                }
                                ExecutionResult::KeepOpen => {}
                                ExecutionResult::Error(err_msg) => {
                                    eprintln!("Error executing launcher action: {}", err_msg);
                                }
                            }
                        }
                    }
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    window.add_controller(key_controller);

    // Click outside main_box to close launcher
    let click_controller = gtk::GestureClick::new();
    let window_click_clone = window.clone();
    let main_box_click_clone = main_box.clone();
    click_controller.connect_pressed(move |_, _, x, y| {
        let alloc = main_box_click_clone.allocation();
        let x_i = x as i32;
        let y_i = y as i32;
        if x_i < alloc.x() || x_i > alloc.x() + alloc.width() || y_i < alloc.y() || y_i > alloc.y() + alloc.height() {
            window_click_clone.close();
        }
    });
    window.add_controller(click_controller);

    // Handle mouse click / row activation on items
    let current_results_row_act = current_results.clone();
    let router_row_act = router.clone();
    let window_row_act = window.clone();
    list_box.connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        let results = current_results_row_act.lock().unwrap();
        if idx < results.len() {
            let result_id = &results[idx].id;
            if result_id == "sys:picker" {
                window_row_act.hide();
                crate::color_picker::pick_color_and_show_ui(window_row_act.clone());
            } else if result_id.starts_with("tr:") {
                window_row_act.hide();
                let _ = router_row_act.execute(result_id, false);
            } else {
                match router_row_act.execute(result_id, false) {
                    ExecutionResult::CloseLauncher => {
                        window_row_act.close();
                    }
                    ExecutionResult::KeepOpen => {}
                    ExecutionResult::Error(err_msg) => {
                        eprintln!("Error executing: {}", err_msg);
                    }
                }
            }
        }
    });

    // Initial search query (updates results when empty initially)
    let router_init = router.clone();
    let list_box_init = list_box.clone();
    let results_init = current_results.clone();
    update_results("", &router_init, &list_box_init, &results_init);

    // Search query update handler (fires on every key stroke)
    let router_update = router.clone();
    let list_box_update = list_box.clone();
    let results_update = current_results.clone();
    search_entry.connect_changed(move |entry| {
        let text = entry.text().to_string();
        update_results(&text, &router_update, &list_box_update, &results_update);
    });

    window.present();
}

/// Helper function to perform matching and refresh listbox elements
fn update_results(
    query: &str,
    router: &SearchRouter,
    list_box: &ListBox,
    current_results: &Arc<Mutex<Vec<SearchResult>>>,
) {
    // Clear listbox
    while let Some(child) = list_box.row_at_index(0) {
        list_box.remove(&child);
    }

    // Query router
    let results = router.query(query);
    
    // Store results
    let mut curr = current_results.lock().unwrap();
    *curr = results.clone();

    // Populate rows
    for res in &results {
        let row = create_row_widget(res);
        list_box.append(&row);
    }

    // Select the first row by default for quick keyboard execution
    if let Some(first_row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first_row));
    }
}

/// Constructs the row widget containing details and icons
fn create_row_widget(result: &SearchResult) -> ListBoxRow {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(6);
    hbox.set_margin_bottom(6);

    // Dynamic icon lookup
    let icon_name = result.icon.as_deref().unwrap_or("application-x-executable");
    let image = Image::from_icon_name(icon_name);
    image.set_pixel_size(32);
    hbox.append(&image);

    // Labels vertical layout
    let vbox = GtkBox::new(Orientation::Vertical, 2);
    vbox.set_valign(gtk::Align::Center); // Centered vertically relative to the icon!

    let title_lbl = Label::builder()
        .label(&result.title)
        .halign(gtk::Align::Start)
        .build();

    if result.description.is_none() {
        title_lbl.add_css_class("single-line-title");
    } else {
        title_lbl.add_css_class("result-title");
    }
    vbox.append(&title_lbl);

    if let Some(desc) = &result.description {
        let desc_lbl = Label::builder()
            .label(desc)
            .halign(gtk::Align::Start)
            .build();
        desc_lbl.add_css_class("result-description");
        vbox.append(&desc_lbl);
    }

    hbox.append(&vbox);
    row.set_child(Some(&hbox));
    row
}

/// Automatically scrolls the viewport so that the selected row is always visible
fn scroll_to_row(list_box: &ListBox, scrolled_window: &ScrolledWindow, row: &ListBoxRow) {
    let list_box_clone = list_box.clone();
    let row_clone = row.clone();
    let adj = scrolled_window.vadjustment();

    glib::idle_add_local(move || {
        if let Some((_, y)) = row_clone.translate_coordinates(&list_box_clone, 0.0, 0.0) {
            let row_height = row_clone.height() as f64;
            let page_size = adj.page_size();
            let current_value = adj.value();

            let y_f = y;
            if y_f < current_value {
                adj.set_value(y_f);
            } else if y_f + row_height > current_value + page_size {
                adj.set_value(y_f + row_height - page_size);
            }
        }
        glib::ControlFlow::Break
    });
}

fn get_history_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let mut path = std::path::PathBuf::from(home);
    path.push(".cache");
    path.push("wayland-launcher-clipboard.json");
    path
}

fn load_clipboard_history() -> Vec<String> {
    let path = get_history_path();
    if let Ok(mut file) = File::open(&path) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Ok(history) = serde_json::from_str::<Vec<String>>(&contents) {
                return history;
            }
        }
    }
    Vec::new()
}

fn save_clipboard_history(history: &[String]) {
    let path = get_history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string(history) {
        if let Ok(mut file) = File::create(&path) {
            let _ = file.write_all(serialized.as_bytes());
        }
    }
}

fn get_current_clipboard() -> Option<String> {
    let output = Command::new("wl-paste")
        .arg("--no-newline")
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}
