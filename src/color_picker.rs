use gtk::prelude::*;
use gtk::{gdk, glib, gio, Box as GtkBox, Label, Button, Window, Orientation, Image};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub fn pick_color_and_show_ui(launcher_window: gtk::ApplicationWindow) {
    let connection = match gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to get session D-Bus connection: {}", e);
            launcher_window.close();
            return;
        }
    };
    let options = std::collections::HashMap::<String, glib::Variant>::new();
    let parameters = (
        "".to_string(),
        options,
    ).to_variant();


    let hold_guard_cell = std::rc::Rc::new(std::cell::RefCell::new(None));
    if let Some(ref a) = launcher_window.application() {
        let guard = <gtk::Application as gio::prelude::ApplicationExtManual>::hold(a);
        *hold_guard_cell.borrow_mut() = Some(guard);
    }
    let hold_guard_cell_for_call = hold_guard_cell.clone();

    let launcher_window_clone = launcher_window.clone();
    let connection_clone = connection.clone();
    connection.call(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
        "PickColor",
        Some(&parameters),
        Some(glib::VariantTy::new("(o)").unwrap()),
        gio::DBusCallFlags::NONE,
        -1,
        None::<&gio::Cancellable>,
        move |result| {
            let request_path = match result {
                Ok(val) => {
                    let tuple = val;
                    tuple.child_value(0).get::<String>().unwrap_or_default()
                }
                Err(e) => {
                    eprintln!("D-Bus call PickColor failed: {}", e);
                    let _guard = hold_guard_cell_for_call.borrow_mut().take();
                    std::process::exit(1);
                }
            };

            if request_path.is_empty() {
                let _guard = hold_guard_cell_for_call.borrow_mut().take();
                std::process::exit(1);
            }
            let launcher_window_clone2 = launcher_window_clone.clone();
            let hold_guard_cell_for_signal = hold_guard_cell_for_call.clone();
            let subscription_cell = std::rc::Rc::new(std::cell::RefCell::new(None));
            let subscription_cell_clone = subscription_cell.clone();

            let sub = connection_clone.subscribe_to_signal(
                Some("org.freedesktop.portal.Desktop"),
                Some("org.freedesktop.portal.Request"),
                Some("Response"),
                Some(&request_path),
                None,
                gio::DBusSignalFlags::NONE,
                move |signal| {
                    let _sub = subscription_cell_clone.borrow_mut().take();
                    let _guard = hold_guard_cell_for_signal.borrow_mut().take();

                    let tuple = signal.parameters;
                    let response_code: u32 = tuple.child_value(0).get().unwrap_or(2);
                    if response_code == 0 {
                        let results = tuple.child_value(1);
                        // Dict is represented as an array of dict entries: a{sv}
                        let mut found_color = None;
                        let n_children = results.n_children();
                        for i in 0..n_children {
                            let entry = results.child_value(i);
                            let key = entry.child_value(0).get::<String>().unwrap_or_default();
                            if key == "color" {
                                found_color = Some(entry.child_value(1));
                                break;
                            }
                        }

                        if let Some(color_var) = found_color {
                            // Extract color tuple (ddd)
                            let inner_val = color_var.child_value(0);
                            let r_val = inner_val.child_value(0).get::<f64>().unwrap_or(0.0);
                            let g_val = inner_val.child_value(1).get::<f64>().unwrap_or(0.0);
                            let b_val = inner_val.child_value(2).get::<f64>().unwrap_or(0.0);

                            let r = (r_val * 255.0).round() as u8;
                            let g = (g_val * 255.0).round() as u8;
                            let b = (b_val * 255.0).round() as u8;

                            println!("Color picked: r={}, g={}, b={} (HEX: #{:02X}{:02X}{:02X})", r, g, b, r, g, b);
                            show_color_picker_result_ui(r, g, b, launcher_window_clone2.clone());
                            return;
                        }
                    }

                    // Exit immediately on cancel/failure
                    std::process::exit(0);
                }
            );
            *subscription_cell.borrow_mut() = Some(sub);
        }
    );
}

fn show_color_picker_result_ui(r: u8, g: u8, b: u8, launcher_window: gtk::ApplicationWindow) {
    let hex_str = format!("#{:02X}{:02X}{:02X}", r, g, b);
    let rgb_str = format!("rgb({}, {}, {})", r, g, b);

    // Auto-copy HEX value on pick
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(&hex_str);
    }

    let window = Window::builder()
        .title("Color Picker Result")
        .build();
    window.add_css_class("launcher-window");

    window.init_layer_shell();
    window.set_namespace(Some("launcher"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_decorated(false);

    // Center on screen with more airy default sizing
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_default_size(420, 310);

    let main_box = GtkBox::new(Orientation::Vertical, 16);
    main_box.add_css_class("launcher-box");
    main_box.set_size_request(340, 230);

    // CSS Provider for local elements
    let preview_provider = gtk::CssProvider::new();
    preview_provider.load_from_data(&format!("
        .launcher-box {{
            padding: 20px;
        }}
        .color-preview-box {{
            min-width: 90px;
            min-height: 90px;
            background-color: rgb({}, {}, {});
            border-radius: 20px;
            border: 1px solid rgba(255, 255, 255, 0.12);
        }}
        .color-value-lbl {{
            font-family: monospace;
            font-size: 15px;
            font-weight: 500;
            color: #ffffff;
        }}
        button.flat-icon-btn {{
            background-color: transparent;
            background-image: none;
            border-style: none;
            box-shadow: none;
            color: rgba(255, 255, 255, 0.6);
            padding: 6px;
            border-radius: 9999px;
            transition: all 120ms ease;
        }}
        button.flat-icon-btn:hover {{
            background-color: rgba(255, 255, 255, 0.08);
            background-image: none;
            color: #ffffff;
        }}
        button.flat-icon-btn:active {{
            background-color: rgba(255, 255, 255, 0.15);
            background-image: none;
        }}
        button.copy-icon-btn {{
            background-color: rgba(255, 255, 255, 0.04);
            background-image: none;
            border: 1px solid rgba(255, 255, 255, 0.08);
            box-shadow: none;
            color: rgba(255, 255, 255, 0.7);
            padding: 8px;
            border-radius: 10px;
            transition: all 120ms ease;
        }}
        button.copy-icon-btn:hover {{
            background-color: rgba(255, 255, 255, 0.08);
            background-image: none;
            border-color: rgba(255, 255, 255, 0.15);
            color: #ffffff;
        }}
        button.copy-icon-btn:active {{
            background-color: rgba(255, 255, 255, 0.15);
            background-image: none;
        }}
        button.picker-action-btn {{
            background-color: rgba(255, 255, 255, 0.06);
            background-image: none;
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 12px;
            color: #ffffff;
            font-size: 13px;
            font-weight: 600;
            padding-top: 10px;
            padding-bottom: 10px;
            padding-left: 20px;
            padding-right: 20px;
            transition: all 120ms ease;
        }}
        button.picker-action-btn:hover {{
            background-color: rgba(255, 255, 255, 0.1);
            background-image: none;
            border-color: rgba(255, 255, 255, 0.15);
        }}
        button.picker-action-btn:active {{
            background-color: rgba(255, 255, 255, 0.15);
            background-image: none;
        }}
    ", r, g, b));
    let display = gdk::Display::default().unwrap();
    gtk::style_context_add_provider_for_display(&display, &preview_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);


    // Header Row
    let header_row = GtkBox::new(Orientation::Horizontal, 0);
    header_row.set_valign(gtk::Align::Center);

    // Spacer to push close button to the right
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header_row.append(&spacer);

    let exiting = std::rc::Rc::new(std::cell::Cell::new(true));

    // Close button (Top-Right)
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

    // Details Row
    let details_row = GtkBox::new(Orientation::Horizontal, 20);
    details_row.set_halign(gtk::Align::Center);

    // Color Preview Block
    let preview_box = GtkBox::builder()
        .width_request(90)
        .height_request(90)
        .build();
    preview_box.add_css_class("color-preview-box");
    details_row.append(&preview_box);

    // Values Column
    let texts_col = GtkBox::new(Orientation::Vertical, 12);
    texts_col.set_valign(gtk::Align::Center);
    texts_col.set_width_request(190);

    // HEX field
    let hex_box = GtkBox::new(Orientation::Horizontal, 12);
    let hex_val_lbl = Label::builder()
        .label(&hex_str)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    hex_val_lbl.add_css_class("color-value-lbl");
    hex_box.append(&hex_val_lbl);
    
    let hex_copy_btn = Button::builder()
        .icon_name("edit-copy-symbolic")
        .build();
    hex_copy_btn.add_css_class("copy-icon-btn");
    let hex_str_clone = hex_str.clone();
    let hex_copy_btn_clone = hex_copy_btn.clone();
    hex_copy_btn.connect_clicked(move |_| {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&hex_str_clone);
        }
        hex_copy_btn_clone.set_icon_name("object-select-symbolic");
        let btn = hex_copy_btn_clone.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
            btn.set_icon_name("edit-copy-symbolic");
            glib::ControlFlow::Break
        });
    });
    hex_box.append(&hex_copy_btn);
    texts_col.append(&hex_box);

    // RGB field
    let rgb_box = GtkBox::new(Orientation::Horizontal, 12);
    let rgb_val_lbl = Label::builder()
        .label(&rgb_str)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    rgb_val_lbl.add_css_class("color-value-lbl");
    rgb_box.append(&rgb_val_lbl);

    let rgb_copy_btn = Button::builder()
        .icon_name("edit-copy-symbolic")
        .build();
    rgb_copy_btn.add_css_class("copy-icon-btn");
    let rgb_str_clone = rgb_str.clone();
    let rgb_copy_btn_clone = rgb_copy_btn.clone();
    rgb_copy_btn.connect_clicked(move |_| {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&rgb_str_clone);
        }
        rgb_copy_btn_clone.set_icon_name("object-select-symbolic");
        let btn = rgb_copy_btn_clone.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
            btn.set_icon_name("edit-copy-symbolic");
            glib::ControlFlow::Break
        });
    });
    rgb_box.append(&rgb_copy_btn);
    texts_col.append(&rgb_box);

    details_row.append(&texts_col);
    main_box.append(&details_row);

    // Actions Row
    let actions_row = GtkBox::new(Orientation::Horizontal, 0);
    actions_row.set_halign(gtk::Align::Center);

    let pick_again_btn = Button::new();
    pick_again_btn.add_css_class("picker-action-btn");
    
    let btn_content = GtkBox::new(Orientation::Horizontal, 6);
    btn_content.set_halign(gtk::Align::Center);
    let pick_icon = Image::from_icon_name("color-select-symbolic");
    let pick_lbl = Label::new(Some("Pick Again"));
    btn_content.append(&pick_icon);
    btn_content.append(&pick_lbl);
    pick_again_btn.set_child(Some(&btn_content));

    let launcher_window_clone = launcher_window.clone();
    let window_clone2 = window.clone();
    let exiting_clone = exiting.clone();
    pick_again_btn.connect_clicked(move |_| {
        exiting_clone.set(false);
        window_clone2.close();
        pick_color_and_show_ui(launcher_window_clone.clone());
    });
    actions_row.append(&pick_again_btn);
    main_box.append(&actions_row);

    window.set_child(Some(&main_box));

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
    let preview_provider_clone = preview_provider.clone();
    let exiting_destroy_clone = exiting.clone();
    window.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display_clone, &preview_provider_clone);
        if exiting_destroy_clone.get() {
            std::process::exit(0);
        }
    });

    let exiting_close_request_clone = exiting.clone();
    window.connect_close_request(move |_| {
        if exiting_close_request_clone.get() {
            std::process::exit(0);
        }
        glib::Propagation::Proceed
    });

    window.present();
}
