use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use std::sync::Mutex;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use gtk::prelude::*;
use gtk::{gdk, glib, Box as GtkBox, Label, Button, Window, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub enum TranslationMsg {
    Translate {
        text: String,
        to_english: bool,
        response_tx: Sender<Result<String, String>>,
    },
}

pub struct TranslatorPlugin {
    tx: Mutex<Sender<TranslationMsg>>,
}

impl TranslatorPlugin {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            run_worker(rx);
        });
        Self {
            tx: Mutex::new(tx),
        }
    }

    fn query_translate_targets(&self, text: &str) -> Vec<SearchResult> {
        vec![
            SearchResult {
                id: format!("tr:en:{}", text),
                title: format!("Translate '{}' to English", text),
                description: Some("Translate German text to English".to_string()),
                icon: Some("accessories-dictionary".to_string()),
                score: 95,
                last_used: None,
            },
            SearchResult {
                id: format!("tr:de:{}", text),
                title: format!("Translate '{}' to German", text),
                description: Some("Translate English text to German".to_string()),
                icon: Some("accessories-dictionary".to_string()),
                score: 90,
                last_used: None,
            },
        ]
    }
}

impl LauncherPlugin for TranslatorPlugin {
    fn id(&self) -> &str {
        "translator"
    }

    fn accepts(&self, query: &str) -> bool {
        let q = query.trim();
        q.starts_with("t:") || q.starts_with("tr ") || q.starts_with("translate ")
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let trimmed = query.trim();
        let text = if trimmed.starts_with("t:") {
            &trimmed[2..]
        } else if trimmed.starts_with("tr ") {
            &trimmed[3..]
        } else if trimmed.starts_with("translate ") {
            &trimmed[10..]
        } else {
            return Vec::new();
        };

        let text = text.trim();
        if text.is_empty() {
            return vec![SearchResult {
                id: "tr:help".to_string(),
                title: "Translate text".to_string(),
                description: Some("Type 't:<text>' to translate between German and English".to_string()),
                icon: Some("accessories-dictionary".to_string()),
                score: 95,
                last_used: None,
            }];
        }

        self.query_translate_targets(text)
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if result_id == "tr:help" {
            return ExecutionResult::KeepOpen;
        }

        if result_id.starts_with("tr:en:") || result_id.starts_with("tr:de:") {
            let to_english = result_id.starts_with("tr:en:");
            let text = &result_id[6..]; // Both "tr:en:" and "tr:de:" are 6 chars prefix

            let tx = {
                let guard = self.tx.lock().unwrap();
                guard.clone()
            };

            show_translation_ui(text, to_english, tx);
            return ExecutionResult::KeepOpen;
        }

        ExecutionResult::Error("Invalid translation command".to_string())
    }
}

fn run_worker(rx: Receiver<TranslationMsg>) {
    for msg in rx {
        match msg {
            TranslationMsg::Translate { text, to_english, response_tx } => {
                let res = translate_via_api(&text, to_english);
                let _ = response_tx.send(res);
            }
        }
    }
}

fn translate_via_api(text: &str, to_english: bool) -> Result<String, String> {
    let from_lang = if to_english { "de" } else { "en" };
    let to_lang = if to_english { "en" } else { "de" };

    // Percent encode the text
    let encoded: String = text
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

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        from_lang, to_lang, encoded
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let json: serde_json::Value = response.json()
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut translated = String::new();
    if let Some(sentences) = json.get(0).and_then(|v| v.as_array()) {
        for sentence in sentences {
            if let Some(trans) = sentence.get(0).and_then(|v| v.as_str()) {
                translated.push_str(trans);
            }
        }
    }

    if translated.is_empty() {
        return Err("Translation empty or parse error".to_string());
    }

    Ok(translated)
}

fn show_translation_ui(text: &str, to_english: bool, tx: Sender<TranslationMsg>) {
    let window = Window::builder()
        .title("Translation Result")
        .build();
    window.add_css_class("launcher-window");

    window.init_layer_shell();
    window.set_namespace(Some("launcher"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_decorated(false);

    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_default_size(420, 310);

    let main_box = GtkBox::new(Orientation::Vertical, 12);
    main_box.add_css_class("launcher-box");
    main_box.set_size_request(340, 230);

    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data("
        .launcher-box {
            padding: 20px;
        }
        .translation-section-title {
            font-size: 11px;
            font-weight: 700;
            color: rgba(255, 255, 255, 0.4);
            text-transform: uppercase;
            letter-spacing: 0.8px;
        }
        .translation-text-lbl {
            font-size: 15px;
            color: #ffffff;
            line-height: 1.4;
        }
        .loading-lbl {
            font-size: 14px;
            color: rgba(255, 255, 255, 0.7);
        }
        button.flat-icon-btn {
            background-color: transparent;
            background-image: none;
            border-style: none;
            box-shadow: none;
            color: rgba(255, 255, 255, 0.6);
            padding: 6px;
            border-radius: 9999px;
            transition: all 120ms ease;
        }
        button.flat-icon-btn:hover {
            background-color: rgba(255, 255, 255, 0.08);
            background-image: none;
            color: #ffffff;
        }
        button.flat-icon-btn:active {
            background-color: rgba(255, 255, 255, 0.15);
            background-image: none;
        }
        button.copy-icon-btn {
            background-color: rgba(255, 255, 255, 0.04);
            background-image: none;
            border: 1px solid rgba(255, 255, 255, 0.08);
            box-shadow: none;
            color: rgba(255, 255, 255, 0.7);
            padding: 6px;
            border-radius: 8px;
            transition: all 120ms ease;
        }
        button.copy-icon-btn:hover {
            background-color: rgba(255, 255, 255, 0.08);
            background-image: none;
            border-color: rgba(255, 255, 255, 0.15);
            color: #ffffff;
        }
        button.copy-icon-btn:active {
            background-color: rgba(255, 255, 255, 0.15);
            background-image: none;
        }
    ");
    let display = gdk::Display::default().unwrap();
    gtk::style_context_add_provider_for_display(&display, &css_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);

    // Header row
    let header_row = GtkBox::new(Orientation::Horizontal, 0);
    let title_lbl = Label::builder()
        .label("Translate")
        .halign(gtk::Align::Start)
        .build();
    title_lbl.add_css_class("translation-section-title");
    header_row.append(&title_lbl);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header_row.append(&spacer);

    let close_btn = Button::builder()
        .icon_name("window-close-symbolic")
        .build();
    close_btn.add_css_class("flat-icon-btn");
    let window_clone = window.clone();
    close_btn.connect_clicked(move |_| {
        window_clone.close();
    });
    header_row.append(&close_btn);
    main_box.append(&header_row);

    // Stack container for Loading vs Result views
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(200);

    // 1. Loading View
    let loading_box = GtkBox::new(Orientation::Vertical, 16);
    loading_box.set_valign(gtk::Align::Center);
    loading_box.set_halign(gtk::Align::Center);
    loading_box.set_vexpand(true);

    let spinner = gtk::Spinner::new();
    spinner.set_size_request(40, 40);
    spinner.start();
    loading_box.append(&spinner);

    let loading_lbl = Label::new(Some("Translating..."));
    loading_lbl.add_css_class("loading-lbl");
    loading_box.append(&loading_lbl);
    stack.add_named(&loading_box, Some("loading"));

    // 2. Result View
    let result_box = GtkBox::new(Orientation::Vertical, 12);
    result_box.set_valign(gtk::Align::Fill);
    result_box.set_halign(gtk::Align::Fill);
    result_box.set_vexpand(true);

    let orig_title = Label::builder()
        .label(if to_english { "Deutsch" } else { "Englisch" })
        .halign(gtk::Align::Start)
        .build();
    orig_title.add_css_class("translation-section-title");
    result_box.append(&orig_title);

    let orig_lbl = Label::builder()
        .label(text)
        .wrap(true)
        .halign(gtk::Align::Start)
        .build();
    orig_lbl.add_css_class("translation-text-lbl");
    result_box.append(&orig_lbl);

    let separator = gtk::Separator::new(Orientation::Horizontal);
    result_box.append(&separator);

    let trans_title_row = GtkBox::new(Orientation::Horizontal, 8);
    let trans_title = Label::builder()
        .label(if to_english { "Englisch" } else { "Deutsch" })
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    trans_title.add_css_class("translation-section-title");
    trans_title_row.append(&trans_title);

    let copy_btn = Button::builder()
        .icon_name("edit-copy-symbolic")
        .build();
    copy_btn.add_css_class("copy-icon-btn");
    trans_title_row.append(&copy_btn);
    result_box.append(&trans_title_row);

    let trans_lbl = Label::builder()
        .wrap(true)
        .halign(gtk::Align::Start)
        .build();
    trans_lbl.add_css_class("translation-text-lbl");
    result_box.append(&trans_lbl);

    stack.add_named(&result_box, Some("result"));

    main_box.append(&stack);
    window.set_child(Some(&main_box));

    // Bind copy button immediately
    let translated_text_cell = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let translated_text_cell_clone = translated_text_cell.clone();
    let copy_btn_clone = copy_btn.clone();
    copy_btn.connect_clicked(move |_| {
        let text_to_copy = translated_text_cell_clone.borrow().clone();
        if !text_to_copy.is_empty() {
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text_to_copy);
            }
            copy_btn_clone.set_icon_name("object-select-symbolic");
            let btn = copy_btn_clone.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
                btn.set_icon_name("edit-copy-symbolic");
                glib::ControlFlow::Break
            });
        }
    });

    // Close on Escape key press
    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let window_clone3 = window.clone();
    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Escape {
            window_clone3.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    let display_clone = display.clone();
    let css_provider_clone = css_provider.clone();
    window.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display_clone, &css_provider_clone);
        std::process::exit(0);
    });

    window.connect_close_request(move |_| {
        std::process::exit(0);
    });

    window.present();

    // Spawn translation request
    let (response_tx, response_rx) = channel();
    let _ = tx.send(TranslationMsg::Translate {
        text: text.to_string(),
        to_english,
        response_tx,
    });

    // Check for response in the GTK main loop
    let stack_clone = stack.clone();
    let trans_lbl_clone = trans_lbl.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match response_rx.try_recv() {
            Ok(Ok(translated_text_val)) => {
                // Auto-copy to clipboard
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&translated_text_val);
                }
                
                *translated_text_cell.borrow_mut() = translated_text_val.clone();
                trans_lbl_clone.set_text(&translated_text_val);
                stack_clone.set_visible_child_name("result");
                glib::ControlFlow::Break
            }
            Ok(Err(err_msg)) => {
                trans_lbl_clone.set_text(&format!("Error: {}", err_msg));
                stack_clone.set_visible_child_name("result");
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                glib::ControlFlow::Continue
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                trans_lbl_clone.set_text("Error: Translation worker disconnected");
                stack_clone.set_visible_child_name("result");
                glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translator_accepts() {
        let plugin = TranslatorPlugin::new();
        assert!(plugin.accepts("t: hello"));
        assert!(plugin.accepts("tr Hello"));
        assert!(plugin.accepts("translate Wie geht es dir"));
        assert!(!plugin.accepts("hello"));
    }

    #[test]
    fn test_translator_query() {
        let plugin = TranslatorPlugin::new();
        let help = plugin.query("t:");
        assert_eq!(help.len(), 1);
        assert_eq!(help[0].id, "tr:help");

        let targets = plugin.query("t: hello");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "tr:en:hello");
        assert_eq!(targets[1].id, "tr:de:hello");
    }
}
