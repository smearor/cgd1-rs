use std::sync::Arc;
use std::time::Duration;

use cgd1_rs::AuthToken;
use cgd1_rs::Backend;
use cgd1_rs::ClockEvent;
use cgd1_rs::ClockManager;
use cgd1_rs::FileTokenStore;
use cgd1_rs::TokenStore;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DropDown;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::ProgressBar;
use gtk4::StringList;
use gtk4::ToggleButton;
use gtk4::Window;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use crate::display::SevenSegmentDisplay;

/// CSS for the main window layout.
const WINDOW_CSS: &str = "
.clock-top {
    background-color: #1a1a2e;
}
.clock-middle {
    background-color: #000000;
}
.clock-bottom {
    background-color: #1a1a2e;
}
.clock-battery-bar {
    min-height: 6px;
}
.clock-bluetooth-label {
    color: #5588ff;
    font-size: 24px;
}
.clock-battery-label {
    color: #00ff41;
    font-size: 14px;
}
.clock-section {
    padding: 8px;
}
.connect-dot {
    min-width: 10px;
    min-height: 10px;
    padding: 0;
    margin: 0;
    border: none;
    border-radius: 9999px;
    background: #ff4444;
}
.connect-dot.connected {
    background: #00ff41;
}
";

/// The main application window showing the alarm clock display.
#[allow(dead_code)]
pub struct MainWindow {
    window: Window,
    runtime: Arc<tokio::runtime::Runtime>,
    manager: Arc<ClockManager>,
    token_store: Arc<FileTokenStore>,
    connected_address: Arc<std::sync::Mutex<Option<cgd1_rs::MacAddress>>>,
    date_display: SevenSegmentDisplay,
    time_display: SevenSegmentDisplay,
    temp_display: SevenSegmentDisplay,
    humidity_display: SevenSegmentDisplay,
    battery_bar: ProgressBar,
    battery_label: Label,
    bluetooth_label: Label,
    device_dropdown: DropDown,
    scan_button: Button,
    connect_button: ToggleButton,
    connect_dot: gtk4::Box,
    connect_label: Label,
    status_label: Label,
    top_section: Box,
    middle_section: Box,
    bottom_section: Box,
}

impl MainWindow {
    /// Create a new main window.
    pub fn new(app: &gtk4::Application, backend: Backend) -> Self {
        let provider = CssProvider::new();
        provider.load_from_data(WINDOW_CSS);
        let display = gtk4::gdk::Display::default().expect("no default display");
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let window = Window::builder()
            .title("Alarm Clock CGD1")
            .default_width(480)
            .default_height(600)
            .application(app)
            .build();

        let device_dropdown = DropDown::new(Some(StringList::new(&["No devices"])), None::<&gtk4::Expression>);
        let scan_button = Button::builder().icon_name("view-refresh-symbolic").tooltip_text("Scan for devices").build();

        let connect_dot = gtk4::Box::builder().css_classes(["connect-dot"]).build();
        let connect_label = Label::builder().label("Connect").build();
        let connect_btn_child = Box::builder().orientation(Orientation::Horizontal).spacing(6).build();
        connect_btn_child.append(&connect_dot);
        connect_btn_child.append(&connect_label);
        let connect_button = ToggleButton::builder().child(&connect_btn_child).build();

        let status_label = Label::builder().label("Disconnected").css_classes(["dim-label"]).build();

        let header = create_header_bar(&scan_button, &device_dropdown, &connect_button);
        window.set_titlebar(Some(&header));

        let main_box = Box::builder().orientation(Orientation::Vertical).vexpand(true).build();

        let top_section = Box::new(Orientation::Vertical, 0);
        let middle_section = Box::new(Orientation::Vertical, 0);
        let bottom_section = Box::new(Orientation::Vertical, 0);

        main_box.append(&top_section);
        main_box.append(&middle_section);
        main_box.append(&bottom_section);

        window.set_child(Some(&main_box));

        let runtime = Arc::new(tokio::runtime::Runtime::new().expect("failed to create tokio runtime"));
        let transport = runtime.block_on(async { backend.create_transport().await.expect("failed to create transport") });
        let manager = Arc::new(ClockManager::new(transport.clone()));
        let token_store = Arc::new(FileTokenStore::default_directory());

        let date_display = SevenSegmentDisplay::new();
        let time_display = SevenSegmentDisplay::new();
        let temp_display = SevenSegmentDisplay::new();
        let humidity_display = SevenSegmentDisplay::new();

        let battery_bar = ProgressBar::builder().fraction(0.0).css_classes(["clock-battery-bar"]).build();
        let battery_label = Label::builder().label("--%").css_classes(["clock-battery-label"]).build();
        let bluetooth_label = Label::builder().label("BT").css_classes(["clock-bluetooth-label"]).build();

        let self_ = Self {
            window,
            runtime,
            manager,
            token_store,
            connected_address: Arc::new(std::sync::Mutex::new(None)),
            date_display,
            time_display,
            temp_display,
            humidity_display,
            battery_bar,
            battery_label,
            bluetooth_label,
            device_dropdown,
            scan_button,
            connect_button,
            connect_dot,
            connect_label,
            status_label,
            top_section: top_section.clone(),
            middle_section: middle_section.clone(),
            bottom_section: bottom_section.clone(),
        };

        self_.setup_layout(&main_box, &top_section, &middle_section, &bottom_section);
        self_.setup_signals();
        self_.start_clock_tick();
        self_.setup_resize_handler();
        self_.start_auto_scan();

        self_
    }

    /// Present the window.
    pub fn present(&self) {
        self.window.present();
    }

    /// Get the inner GTK window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get the clock manager.
    pub fn manager(&self) -> &Arc<ClockManager> {
        &self.manager
    }

    /// Get the tokio runtime.
    pub fn runtime(&self) -> &Arc<tokio::runtime::Runtime> {
        &self.runtime
    }

    /// Get the Arc to the connected address mutex for sharing with closures.
    pub fn connected_address_arc(&self) -> Arc<std::sync::Mutex<Option<cgd1_rs::MacAddress>>> {
        self.connected_address.clone()
    }

    fn setup_layout(&self, _main_box: &Box, top: &Box, middle: &Box, bottom: &Box) {
        // Top section: 20% of window height
        top.add_css_class("clock-top");
        let top_content = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .halign(Align::Fill)
            .valign(Align::Center)
            .hexpand(true)
            .build();

        let battery_box = Box::builder().orientation(Orientation::Vertical).spacing(4).halign(Align::Start).build();
        battery_box.append(&self.battery_label);
        battery_box.append(&self.battery_bar);
        top_content.append(&battery_box);

        let date_container = Box::builder().orientation(Orientation::Vertical).halign(Align::Center).hexpand(true).build();
        date_container.append(&self.date_display);
        top_content.append(&date_container);

        let bt_box = Box::builder().orientation(Orientation::Vertical).halign(Align::End).build();
        bt_box.append(&self.bluetooth_label);
        top_content.append(&bt_box);

        top.append(&top_content);

        // Middle section: ~60% of window height — time display
        middle.set_vexpand(true);
        middle.add_css_class("clock-middle");
        let middle_content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .hexpand(true)
            .build();
        middle_content.append(&self.time_display);
        middle.append(&middle_content);

        // Bottom section: ~25% of window height — sensors
        bottom.set_vexpand(false);
        bottom.add_css_class("clock-bottom");
        let bottom_content = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(24)
            .halign(Align::Fill)
            .valign(Align::Center)
            .hexpand(true)
            .build();

        let temp_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .halign(Align::Center)
            .hexpand(true)
            .build();
        let temp_label = Label::builder().label("Temperature").css_classes(["dim-label"]).build();
        temp_box.append(&temp_label);
        temp_box.append(&self.temp_display);
        bottom_content.append(&temp_box);

        let humidity_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .halign(Align::Center)
            .hexpand(true)
            .build();
        let humidity_label = Label::builder().label("Humidity").css_classes(["dim-label"]).build();
        humidity_box.append(&humidity_label);
        humidity_box.append(&self.humidity_display);
        bottom_content.append(&humidity_box);

        bottom.append(&bottom_content);
    }

    fn setup_signals(&self) {
        let runtime_arc = self.runtime.clone();
        let runtime = runtime_arc.handle().clone();
        let manager = self.manager.clone();
        let dropdown = self.device_dropdown.clone();
        let status = self.status_label.clone();
        let connect_btn = self.connect_button.clone();
        let scan_btn = std::rc::Rc::new(self.scan_button.clone());
        let scan_btn_for_connect = scan_btn.clone();

        scan_btn_for_connect.connect_clicked(move |_| {
            (*scan_btn).set_sensitive(false);
            status.set_label("Scanning...");
            let manager = manager.clone();
            let _runtime_keepalive = runtime_arc.clone();
            let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<String>, String>>();
            runtime.spawn(async move {
                let scanner = manager.scanner();
                let result = scanner.scan_active(Duration::from_secs(10)).await;
                let mapped = result
                    .map(|devices| {
                        devices
                            .iter()
                            .map(|d| {
                                let rssi = d.rssi.map(|r| format!(" ({} dBm)", r)).unwrap_or_default();
                                format!("{}{}", d.address, rssi)
                            })
                            .collect::<Vec<String>>()
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(mapped);
            });
            let dropdown = dropdown.clone();
            let status = status.clone();
            let scan_btn = scan_btn.clone();
            let connect_btn = connect_btn.clone();
            let rx = std::cell::RefCell::new(rx);
            glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                Ok(result) => {
                    match result {
                        Ok(labels) => {
                            let strs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                            let model = StringList::new(&strs);
                            dropdown.set_model(Some(&model));
                            if labels.is_empty() {
                                status.set_label("No devices found");
                            } else {
                                status.set_label(&format!("Found {} device(s)", labels.len()));
                                // Auto-connect to the first device
                                if !connect_btn.is_active() {
                                    connect_btn.emit_clicked();
                                }
                            }
                        }
                        Err(e) => {
                            status.set_label(&format!("Scan failed: {e}"));
                        }
                    }
                    (*scan_btn).set_sensitive(true);
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    (*scan_btn).set_sensitive(true);
                    status.set_label("Scan task failed");
                    glib::ControlFlow::Break
                }
            });
        });

        let runtime_arc = self.runtime.clone();
        let runtime = runtime_arc.handle().clone();
        let manager = self.manager.clone();
        let dropdown = self.device_dropdown.clone();
        let connect_btn = self.connect_button.clone();
        let connect_dot = self.connect_dot.clone();
        let connect_label = self.connect_label.clone();
        let status = self.status_label.clone();
        let token_store = self.token_store.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let time_display = self.time_display.clone();
        let date_display = self.date_display.clone();
        let battery_bar = self.battery_bar.clone();
        let battery_label = self.battery_label.clone();
        let connected_address = self.connected_address.clone();

        connect_btn.connect_clicked(move |btn| {
            let _runtime_keepalive = runtime_arc.clone();
            let addr_text = dropdown
                .selected_item()
                .and_then(|item| item.downcast::<gtk4::StringObject>().ok())
                .map(|obj| obj.string().to_string())
                .unwrap_or_default();
            if addr_text.is_empty() || addr_text == "No devices" {
                status.set_label("No device selected");
                return;
            }
            let addr = match cgd1_rs::MacAddress::parse(&addr_text) {
                Ok(a) => a,
                Err(e) => {
                    status.set_label(&format!("Invalid address: {e}"));
                    return;
                }
            };
            if btn.is_active() {
                connect_label.set_label("Disconnect");
                status.set_label("Connecting...");
                let manager = manager.clone();
                let token_store = token_store.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
                let (event_tx, event_rx) = std::sync::mpsc::channel::<ClockEvent>();
                runtime.spawn(async move {
                    let token_result = token_store.load_or_generate(&addr);
                    let token: AuthToken = (*token_result).clone();
                    let result = manager
                        .connect_authenticate_and_sync(&addr, &token, token_store.clone() as Arc<dyn TokenStore>)
                        .await;
                    match &result {
                        Ok(_) => {
                            let _ = tx.send(Ok(format!("Connected to {addr}")));
                            if let Some(device) = manager.device(&addr).await {
                                if let Ok(level) = device.read_battery().await {
                                    let _ = event_tx.send(ClockEvent::BatteryLevel { level });
                                }
                                let mut rx_events = device.subscribe();
                                while let Ok(event) = rx_events.recv().await {
                                    if event_tx.send(event).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string()));
                        }
                    }
                });
                let status = status.clone();
                let status_for_events = status.clone();
                let btn = btn.clone();
                let connect_dot = connect_dot.clone();
                let connect_label = connect_label.clone();
                let connected_address_for_connect = connected_address.clone();
                let addr_for_connect = addr;
                let rx = std::cell::RefCell::new(rx);
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(msg) => {
                                *connected_address_for_connect.lock().expect("mutex poisoned") = Some(addr_for_connect);
                                connect_dot.add_css_class("connected");
                                status.set_label(&msg);
                            }
                            Err(e) => {
                                status.set_label(&format!("Connect failed: {e}"));
                                btn.set_active(false);
                                connect_label.set_label("Connect");
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status.set_label("Connect task failed");
                        btn.set_active(false);
                        connect_label.set_label("Connect");
                        glib::ControlFlow::Break
                    }
                });
                let temp_display = temp_display.clone();
                let humidity_display = humidity_display.clone();
                let battery_bar = battery_bar.clone();
                let battery_label = battery_label.clone();
                let event_rx = std::cell::RefCell::new(event_rx);
                glib::source::idle_add_local(move || match event_rx.borrow_mut().try_recv() {
                    Ok(event) => {
                        match event {
                            ClockEvent::SensorUpdate { temperature, humidity } => {
                                temp_display.set_display_text(&format!("{:.1}°C", temperature.value()));
                                humidity_display.set_display_text(&format!("{:.0}%", humidity.value()));
                            }
                            ClockEvent::BatteryLevel { level } => {
                                let pct = level.value() as f64;
                                battery_bar.set_fraction(pct / 100.0);
                                battery_label.set_label(&format!("{:.0}%", pct));
                            }
                            ClockEvent::Disconnected => {
                                status_for_events.set_label("Device disconnected");
                            }
                            ClockEvent::Reconnected => {
                                status_for_events.set_label("Device reconnected");
                            }
                            _ => {}
                        }
                        glib::ControlFlow::Continue
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
                });
            } else {
                connect_label.set_label("Connect");
                connect_dot.remove_css_class("connected");
                *connected_address.lock().expect("mutex poisoned") = None;
                let manager = manager.clone();
                runtime.spawn(async move {
                    let _ = manager.disconnect(&addr).await;
                });
                temp_display.set_display_text("");
                humidity_display.set_display_text("");
                time_display.set_display_text("");
                date_display.set_display_text("");
                battery_bar.set_fraction(0.0);
                battery_label.set_label("--%");
                status.set_label("Disconnected");
            }
        });
    }

    /// Set up a handler to update font sizes and section proportions when the window is resized.
    fn setup_resize_handler(&self) {
        let time_display = self.time_display.clone();
        let date_display = self.date_display.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let win = self.window.clone();

        // Poll window height periodically and update font sizes when it changes.
        let last_height = std::rc::Rc::new(std::cell::Cell::new(0i32));
        let last_height_clone = last_height.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let height = win.height();
            if height <= 0 || height == last_height_clone.get() {
                return glib::ControlFlow::Continue;
            }
            last_height_clone.set(height);
            let h = height as f64;
            // Font sizes: time 20%, date 6%, sensors 5%
            let time_scale = (h * 0.20) / 48.0;
            let date_scale = (h * 0.06) / 48.0;
            let sensor_scale = (h * 0.05) / 48.0;
            time_display.set_font_scale(time_scale);
            date_display.set_font_scale(date_scale);
            temp_display.set_font_scale(sensor_scale);
            humidity_display.set_font_scale(sensor_scale);
            glib::ControlFlow::Continue
        });
    }

    fn start_clock_tick(&self) {
        let date_display = self.date_display.clone();
        let time_display = self.time_display.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();

        glib::timeout_add_local(Duration::from_secs(1), move || {
            let now = glib::DateTime::now_local().unwrap_or_else(|_| glib::DateTime::now_utc().unwrap());
            let date_str = format!("{:02}.{:02}.", now.day_of_month(), now.month());
            date_display.set_display_text(&date_str);

            let time_str = format!("{:02}:{:02}", now.hour(), now.minute());
            time_display.set_display_text(&time_str);

            if temp_display.imp().text.borrow().is_empty() {
                temp_display.set_display_text("--.-°C");
            }
            if humidity_display.imp().text.borrow().is_empty() {
                humidity_display.set_display_text("--.-%");
            }

            glib::ControlFlow::Continue
        });
    }

    /// Trigger a scan automatically on startup.
    fn start_auto_scan(&self) {
        self.scan_button.emit_clicked();
    }
}

/// Create the header bar with scan button, split-button (device selector + connect), and menu.
fn create_header_bar(scan_button: &Button, device_dropdown: &DropDown, connect_button: &ToggleButton) -> gtk4::HeaderBar {
    let header = gtk4::HeaderBar::builder().show_title_buttons(true).build();

    // Scan button on the left
    header.pack_start(scan_button);

    // Split-button: [DropDown] [Connect] linked together
    let split_button = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .css_classes(["linked"])
        .build();
    split_button.append(device_dropdown);
    split_button.append(connect_button);
    header.pack_start(&split_button);

    let menu = gio::Menu::new();
    menu.append(Some("Alarms"), Some("app.alarms"));
    menu.append(Some("Settings"), Some("app.settings"));
    menu.append(Some("Audio"), Some("app.audio"));
    menu.append(Some("Info"), Some("app.info"));

    let menu_button = gtk4::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu).build();
    header.pack_end(&menu_button);

    header
}
