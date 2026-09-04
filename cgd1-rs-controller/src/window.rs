use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use cgd1_rs::AuthToken;
use cgd1_rs::Backend;
use cgd1_rs::ClockEvent;
use cgd1_rs::ClockManager;
use cgd1_rs::FileTokenStore;
use cgd1_rs::KnownDeviceStore;
use cgd1_rs::MacAddress;
use cgd1_rs::TokenStore;

use tracing::debug;
use tracing::info;
use tracing::warn;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DropDown;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::ProgressBar;
use gtk4::StringList;
use gtk4::Switch;
use gtk4::ToggleButton;
use gtk4::Window;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use crate::device_runtime_state::DeviceRuntimeState;
use crate::dialog::AlarmEditorWidget;
use crate::display::SevenSegmentDisplay;

/// CSS for the main window layout.
const WINDOW_CSS: &str = include_str!("../assets/window.css");

/// The main application window showing the alarm clock display.
#[allow(dead_code)]
pub struct MainWindow {
    window: Window,
    runtime: Arc<tokio::runtime::Runtime>,
    manager: Arc<ClockManager>,
    token_store: Arc<FileTokenStore>,
    /// Address of the device currently displayed in the UI.
    selected_address: Arc<Mutex<Option<MacAddress>>>,
    /// Per-device runtime state for all connected devices.
    device_states: Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>>,
    date_display: SevenSegmentDisplay,
    time_display: SevenSegmentDisplay,
    temp_display: SevenSegmentDisplay,
    humidity_display: SevenSegmentDisplay,
    battery_bar: ProgressBar,
    battery_label: Label,
    bluetooth_label: Label,
    device_dropdown: DropDown,
    scan_button: Button,
    connect_switch: Switch,
    status_label: Label,
    top_section: Box,
    middle_section: Box,
    bottom_section: Box,
    /// Container holding top+middle+bottom sections (hidden when alarm editor is expanded).
    full_display: Box,
    /// Compact single-line summary bar (shown when alarm editor is expanded).
    summary_bar: Box,
    /// Toggle button to expand/collapse the alarm editor panel.
    alarm_toggle: ToggleButton,
    /// Revealer for the collapsible alarm editor panel.
    alarm_revealer: gtk4::Revealer,
    /// Summary bar widgets.
    summary_date: Label,
    summary_time: Label,
    summary_temp: Label,
    summary_humidity: Label,
    summary_battery: ProgressBar,
    /// Number of consecutive scans in which a known device was not seen.
    missed_scans: Arc<Mutex<HashMap<MacAddress, u32>>>,
    /// Known device addresses from the last successful scan.
    known_devices: Arc<Mutex<Vec<MacAddress>>>,
    /// Timestamp of the last scan initiation.
    last_scan_time: Arc<Mutex<Option<Instant>>>,
    /// Persistent store of previously connected device addresses.
    known_device_store: Arc<KnownDeviceStore>,
    /// Last battery level seen from advertising scans, keyed by device address.
    scan_battery_cache: Arc<Mutex<HashMap<MacAddress, u8>>>,
}

impl MainWindow {
    /// Create a new main window.
    pub fn new(app: &gtk4::Application, backend: Backend) -> Self {
        let provider = CssProvider::new();
        provider.load_from_data(WINDOW_CSS);
        let display = gtk4::gdk::Display::default().unwrap_or_else(|| {
            eprintln!("Error: No default display available. Cannot start GUI application.");
            std::process::exit(1);
        });
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let window = Window::builder()
            .title("Alarm Clock CGD1")
            .default_width(480)
            .default_height(600)
            .application(app)
            .build();

        let device_dropdown = DropDown::new(Some(StringList::new(&["No devices"])), None::<&gtk4::Expression>);
        let scan_button = Button::builder().icon_name("view-refresh-symbolic").tooltip_text("Scan for devices").build();

        let connect_switch = Switch::builder().tooltip_text("Connect / Disconnect").build();

        let status_label = Label::builder().label("Disconnected").css_classes(["dim-label"]).build();

        let header = create_header_bar(&scan_button, &device_dropdown, &connect_switch);
        window.set_titlebar(Some(&header));

        let main_box = Box::builder().orientation(Orientation::Vertical).vexpand(true).build();

        let top_section = Box::new(Orientation::Vertical, 0);
        let middle_section = Box::new(Orientation::Vertical, 0);
        let bottom_section = Box::new(Orientation::Vertical, 0);

        // Full display container (top + middle + bottom)
        let full_display = Box::builder().orientation(Orientation::Vertical).vexpand(true).build();
        full_display.append(&top_section);
        full_display.append(&middle_section);
        full_display.append(&bottom_section);

        // Compact summary bar (shown when alarm editor is expanded)
        let summary_bar = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(4)
            .css_classes(["clock-top"])
            .visible(false)
            .build();
        let summary_battery = ProgressBar::builder()
            .fraction(0.0)
            .css_classes(["clock-battery-bar"])
            .width_request(80)
            .build();
        let summary_date = Label::builder().label("--.--.").css_classes(["clock-battery-label"]).build();
        let summary_time = Label::builder().label("--:--").css_classes(["clock-bluetooth-label"]).build();
        let summary_temp = Label::builder().label("--.-°C").css_classes(["clock-battery-label"]).build();
        let summary_humidity = Label::builder().label("--.-%").css_classes(["clock-battery-label"]).build();
        let summary_bt = Label::builder().label("BT").css_classes(["clock-bluetooth-label"]).build();
        summary_bar.append(&summary_battery);
        summary_bar.append(&summary_date);
        summary_bar.append(&summary_time);
        summary_bar.append(&summary_temp);
        summary_bar.append(&summary_humidity);
        summary_bar.append(&summary_bt);

        // Alarm toggle button
        let alarm_toggle = ToggleButton::builder().label("Alarms").tooltip_text("Show / hide alarm editor").build();

        // Alarm editor revealer (collapsed by default)
        let alarm_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        main_box.append(&full_display);
        main_box.append(&summary_bar);
        main_box.append(&alarm_toggle);
        main_box.append(&alarm_revealer);

        window.set_child(Some(&main_box));

        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap_or_else(|e| {
            eprintln!("Error: Failed to create async runtime: {e}.");
            std::process::exit(1);
        }));
        let transport = runtime.block_on(async { backend.create_transport().await }).unwrap_or_else(|e| {
            eprintln!("Error: Failed to create BLE transport: {e}.\n\nIf no Bluetooth adapter is available, run with --backend virtual.");
            std::process::exit(1);
        });
        let manager = Arc::new(ClockManager::new(transport.clone()));
        let token_store = Arc::new(FileTokenStore::default_directory());
        let known_device_store = Arc::new(KnownDeviceStore::default_path());

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
            selected_address: Arc::new(Mutex::new(None)),
            device_states: Arc::new(Mutex::new(HashMap::new())),
            date_display,
            time_display,
            temp_display,
            humidity_display,
            battery_bar,
            battery_label,
            bluetooth_label,
            device_dropdown,
            scan_button,
            connect_switch,
            status_label,
            top_section: top_section.clone(),
            middle_section: middle_section.clone(),
            bottom_section: bottom_section.clone(),
            full_display: full_display.clone(),
            summary_bar: summary_bar.clone(),
            alarm_toggle: alarm_toggle.clone(),
            alarm_revealer: alarm_revealer.clone(),
            summary_date: summary_date.clone(),
            summary_time: summary_time.clone(),
            summary_temp: summary_temp.clone(),
            summary_humidity: summary_humidity.clone(),
            summary_battery: summary_battery.clone(),
            missed_scans: Arc::new(Mutex::new(HashMap::new())),
            known_devices: Arc::new(Mutex::new(known_device_store.load())),
            last_scan_time: Arc::new(Mutex::new(None)),
            scan_battery_cache: Arc::new(Mutex::new(known_device_store.load_battery())),
            known_device_store,
        };

        self_.setup_layout(&main_box, &top_section, &middle_section, &bottom_section);
        self_.setup_dropdown_factory();
        self_.setup_signals();
        self_.setup_alarm_panel();
        self_.start_clock_tick();
        self_.setup_resize_handler();
        self_.populate_known_devices();
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

    /// Get the Arc to the selected address mutex for sharing with closures.
    pub fn selected_address_arc(&self) -> Arc<Mutex<Option<MacAddress>>> {
        self.selected_address.clone()
    }

    /// Get the Arc to the device states mutex for sharing with closures.
    pub fn device_states_arc(&self) -> Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>> {
        self.device_states.clone()
    }

    /// Get the Arc to the known devices mutex for sharing with closures.
    pub fn known_devices_arc(&self) -> Arc<Mutex<Vec<MacAddress>>> {
        self.known_devices.clone()
    }

    /// Get the Arc to the token store for sharing with closures.
    pub fn token_store_arc(&self) -> Arc<FileTokenStore> {
        self.token_store.clone()
    }

    /// Get a clone of the connect switch for sharing with closures.
    pub fn connect_switch_arc(&self) -> Switch {
        self.connect_switch.clone()
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

    /// Set up the collapsible alarm editor panel with compact summary bar.
    fn setup_alarm_panel(&self) {
        let editor = AlarmEditorWidget::new(self.manager.clone(), self.runtime.clone(), self.selected_address.clone());
        self.alarm_revealer.set_child(Some(&editor.container));

        let full_display = self.full_display.clone();
        let summary_bar = self.summary_bar.clone();
        let revealer = self.alarm_revealer.clone();
        let editor = std::rc::Rc::new(editor);

        self.alarm_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
            if expanded {
                editor.read_alarms();
            }
        });
    }

    /// Set up a custom factory for the device dropdown so each entry shows
    /// a Bluetooth icon, the address text, and a connection-status dot.
    fn setup_dropdown_factory(&self) {
        let factory = gtk4::SignalListItemFactory::new();

        let _device_states_setup = self.device_states.clone();
        factory.connect_setup(move |_factory, item| {
            let row = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .margin_start(4)
                .margin_end(4)
                .build();

            let bt_icon = Label::builder().label("BT").css_classes(["clock-bluetooth-label"]).build();
            let addr_label = Label::builder().hexpand(true).halign(Align::Start).build();
            let dot = Box::builder().css_classes(["dropdown-dot"]).build();

            row.append(&bt_icon);
            row.append(&addr_label);
            row.append(&dot);
            item.set_child(Some(&row));
        });

        let device_states_bind = self.device_states.clone();
        factory.connect_bind(move |_factory, item| {
            let row = match item.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
                Some(b) => b,
                None => return,
            };
            // Children: [0] BT icon Label, [1] addr Label, [2] dot Box
            let addr_label = match row.first_child().and_then(|c| c.next_sibling()).and_then(|c| c.downcast::<Label>().ok()) {
                Some(l) => l,
                None => return,
            };
            let dot = match addr_label.next_sibling().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
                Some(d) => d,
                None => return,
            };
            let string_obj = match item.item().and_then(|i| i.downcast::<gtk4::StringObject>().ok()) {
                Some(s) => s,
                None => return,
            };
            let text = string_obj.string().to_string();
            addr_label.set_label(&text);

            // Parse address from the label (may contain RSSI suffix)
            let addr_text = text.split_whitespace().next().unwrap_or(&text);
            let connected = match cgd1_rs::MacAddress::parse(addr_text) {
                Ok(addr) => {
                    let states = device_states_bind.lock().unwrap_or_else(|p| {
                        tracing::warn!("mutex poisoned — recovering");
                        p.into_inner()
                    });
                    states.get(&addr).map_or(false, |s| s.connected)
                }
                Err(_) => false,
            };
            if connected {
                dot.add_css_class("connected");
            } else {
                dot.remove_css_class("connected");
            }
        });

        self.device_dropdown.set_factory(Some(&factory));
    }

    fn setup_signals(&self) {
        let runtime_arc = self.runtime.clone();
        let runtime = runtime_arc.handle().clone();
        let manager = self.manager.clone();
        let dropdown = self.device_dropdown.clone();
        let status = self.status_label.clone();
        let connect_switch = self.connect_switch.clone();
        let scan_btn = std::rc::Rc::new(self.scan_button.clone());
        let scan_btn_for_connect = scan_btn.clone();
        let missed_scans = self.missed_scans.clone();
        let known_devices = self.known_devices.clone();
        let device_states_for_scan = self.device_states.clone();
        let known_device_store_for_scan = self.known_device_store.clone();
        let battery_bar_for_scan = self.battery_bar.clone();
        let battery_label_for_scan = self.battery_label.clone();
        let summary_battery_for_scan = self.summary_battery.clone();
        let selected_address_for_scan = self.selected_address.clone();
        let scan_battery_cache_for_scan = self.scan_battery_cache.clone();

        scan_btn_for_connect.connect_clicked(move |_| {
            (*scan_btn).set_sensitive(false);
            status.set_label("Scanning...");
            debug!("controller: scan triggered");
            let manager = manager.clone();
            let _runtime_keepalive = runtime_arc.clone();
            let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<(String, MacAddress, Option<u8>)>, String>>();
            runtime.spawn(async move {
                let scanner = manager.scanner();
                let result = scanner.scan_active(Duration::from_secs(10)).await;
                let mapped = result
                    .map(|devices| {
                        devices
                            .iter()
                            .map(|d| {
                                let rssi = d.rssi.map(|r| format!(" ({} dBm)", r)).unwrap_or_default();
                                let battery = d.advertisement.as_ref().map(|a| a.battery.value());
                                (format!("{}{}", d.address, rssi), d.address, battery)
                            })
                            .collect::<Vec<(String, MacAddress, Option<u8>)>>()
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(mapped);
            });
            let dropdown = dropdown.clone();
            let status = status.clone();
            let scan_btn = scan_btn.clone();
            let connect_switch = connect_switch.clone();
            let missed_scans = missed_scans.clone();
            let known_devices = known_devices.clone();
            let device_states_for_scan = device_states_for_scan.clone();
            let known_device_store_for_scan = known_device_store_for_scan.clone();
            let battery_bar_for_scan = battery_bar_for_scan.clone();
            let battery_label_for_scan = battery_label_for_scan.clone();
            let summary_battery_for_scan = summary_battery_for_scan.clone();
            let selected_address_for_scan = selected_address_for_scan.clone();
            let scan_battery_cache_for_scan = scan_battery_cache_for_scan.clone();
            let rx = std::cell::RefCell::new(rx);
            glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                Ok(result) => {
                    match result {
                        Ok(found) => {
                            let found_addrs: Vec<MacAddress> = found.iter().map(|(_, a, _)| *a).collect();
                            let connected_addrs: Vec<MacAddress> = {
                                let states = device_states_for_scan.lock().unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                });
                                states.keys().copied().collect()
                            };
                            debug!(found = found_addrs.len(), connected = connected_addrs.len(), "controller: scan complete");
                            {
                                let persisted = known_device_store_for_scan.load();
                                let mut missed = missed_scans.lock().unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                });
                                let mut known = known_devices.lock().unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                });
                                for addr in &found_addrs {
                                    missed.remove(addr);
                                }
                                for addr in known.iter() {
                                    if !found_addrs.contains(addr) {
                                        if connected_addrs.contains(addr) {
                                            continue;
                                        }
                                        if persisted.contains(addr) {
                                            continue;
                                        }
                                        let count = missed.entry(*addr).or_insert(0);
                                        *count += 1;
                                        debug!(address = %addr, missed = *count, "controller: device missed in scan");
                                    }
                                }
                                known.retain(|addr| {
                                    if connected_addrs.contains(addr) {
                                        return true;
                                    }
                                    if persisted.contains(addr) {
                                        return true;
                                    }
                                    *missed.get(addr).unwrap_or(&0) < 3
                                });
                                for addr in &found_addrs {
                                    if !known.contains(addr) {
                                        known.push(*addr);
                                    }
                                }
                            }

                            let labels: Vec<String> = {
                                let known = known_devices.lock().unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                });
                                known
                                    .iter()
                                    .map(|addr| {
                                        if found.iter().any(|(_, a, _)| a == addr) {
                                            found.iter().find(|(_, a, _)| a == addr).unwrap().0.clone()
                                        } else {
                                            format!("{}", addr)
                                        }
                                    })
                                    .collect()
                            };

                            let strs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                            let model = StringList::new(&strs);
                            dropdown.set_model(Some(&model));
                            if found.is_empty() {
                                status.set_label("No devices found");
                            } else {
                                status.set_label(&format!("Found {} device(s)", found.len()));
                                if !connect_switch.is_active() {
                                    if let Some(idx) = {
                                        let known = known_devices.lock().unwrap_or_else(|p| {
                                            tracing::warn!("mutex poisoned — recovering");
                                            p.into_inner()
                                        });
                                        known.iter().position(|a| found_addrs.contains(a))
                                    } {
                                        dropdown.set_selected(idx as u32);
                                        connect_switch.set_active(true);
                                    }
                                }
                            }

                            // Update battery from advertising data for all found devices.
                            let selected_addr = *selected_address_for_scan.lock().unwrap_or_else(|p| {
                                tracing::warn!("mutex poisoned — recovering");
                                p.into_inner()
                            });
                            for (_, addr, battery) in &found {
                                if let Some(level) = battery {
                                    // Cache battery in memory and persist to disk
                                    // so it survives restarts.
                                    scan_battery_cache_for_scan
                                        .lock()
                                        .unwrap_or_else(|p| {
                                            tracing::warn!("mutex poisoned — recovering");
                                            p.into_inner()
                                        })
                                        .insert(*addr, *level);
                                    if let Err(e) = known_device_store_for_scan.save_battery(addr, *level) {
                                        warn!(address = %addr, error = %e, "failed to persist battery level");
                                    }

                                    let mut states = device_states_for_scan.lock().unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(addr) {
                                        state.battery_level = Some(*level);
                                        state.last_data_time = Some(Instant::now());
                                        debug!(address = %addr, battery = level, "scan: updated battery from advertising");
                                        if Some(*addr) == selected_addr {
                                            battery_bar_for_scan.set_fraction(*level as f64 / 100.0);
                                            battery_label_for_scan.set_label(&format!("{:.0}%", level));
                                            summary_battery_for_scan.set_fraction(*level as f64 / 100.0);
                                        }
                                    }
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

        // --- Connect Switch handler ---
        let runtime_arc = self.runtime.clone();
        let runtime = runtime_arc.handle().clone();
        let manager = self.manager.clone();
        let dropdown = self.device_dropdown.clone();
        let connect_switch = self.connect_switch.clone();
        let status = self.status_label.clone();
        let token_store = self.token_store.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let time_display = self.time_display.clone();
        let date_display = self.date_display.clone();
        let battery_bar = self.battery_bar.clone();
        let battery_label = self.battery_label.clone();
        let summary_temp = self.summary_temp.clone();
        let summary_humidity = self.summary_humidity.clone();
        let summary_battery = self.summary_battery.clone();
        let selected_address = self.selected_address.clone();
        let device_states = self.device_states.clone();
        let known_device_store = self.known_device_store.clone();
        let scan_battery_cache_for_connect = self.scan_battery_cache.clone();

        connect_switch.connect_active_notify(move |sw| {
            let _runtime_keepalive = runtime_arc.clone();
            let addr_text = dropdown
                .selected_item()
                .and_then(|item| item.downcast::<gtk4::StringObject>().ok())
                .map(|obj| obj.string().to_string())
                .unwrap_or_default();
            if addr_text.is_empty() || addr_text == "No devices" {
                status.set_label("No device selected");
                debug!("controller: connect clicked but no device selected");
                return;
            }
            let addr = match cgd1_rs::MacAddress::parse(&addr_text) {
                Ok(a) => a,
                Err(e) => {
                    warn!(text = %addr_text, error = %e, "controller: failed to parse address from dropdown");
                    status.set_label(&format!("Invalid address: {e}"));
                    return;
                }
            };
            debug!(address = %addr, active = sw.is_active(), "controller: switch toggled");
            if sw.is_active() {
                status.set_label("Connecting...");
                let manager = manager.clone();
                let token_store = token_store.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
                let (event_tx, event_rx) = std::sync::mpsc::channel::<ClockEvent>();
                let scan_battery_cache = scan_battery_cache_for_connect.clone();
                runtime.spawn(async move {
                    let token_result = token_store.load_or_generate(&addr);
                    let is_new_token = token_result.is_new();
                    let token: AuthToken = (*token_result).clone();
                    if is_new_token {
                        warn!(%addr, "no stored token found, generated new random token — device must be unpaired (factory reset) to accept it");
                    } else {
                        debug!(%addr, "using stored token from file");
                    }

                    let result = manager
                        .connect_authenticate_and_sync(&addr, &token, token_store.clone() as Arc<dyn TokenStore>)
                        .await;
                    match &result {
                        Ok(_) => {
                            let _ = tx.send(Ok(format!("Connected to {addr}")));

                            // Use cached advertising battery if available.
                            // The CGD1 GATT battery characteristic (0x2A19)
                            // returns an unreliable 99%, so we rely solely on
                            // advertising data captured by scans.
                            if let Some(level) = scan_battery_cache
                                .lock()
                                .unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                })
                                .get(&addr)
                                .copied()
                            {
                                debug!(%addr, battery = level, "connect: using cached battery from scan");
                                let _ = event_tx.send(ClockEvent::BatteryLevel {
                                    level: cgd1_rs::BatteryLevel::new(level),
                                });
                            }

                            if let Some(device) = manager.device(&addr).await {
                                debug!(%addr, "starting event forwarding loop");
                                let mut rx_events = device.subscribe();
                                while let Ok(event) = rx_events.recv().await {
                                    debug!(%addr, event = ?event, "forwarding event from broadcast to mpsc");
                                    if event_tx.send(event).is_err() {
                                        warn!(%addr, "event_tx receiver dropped, stopping forwarding loop");
                                        break;
                                    }
                                }
                                warn!(%addr, "broadcast receiver loop ended");
                            }
                        }
                        Err(e) => {
                            let msg = if is_new_token {
                                format!("{e}. Device may need factory reset to accept a new token.")
                            } else {
                                e.to_string()
                            };
                            let _ = tx.send(Err(msg));
                        }
                    }
                });
                let status = status.clone();
                let status_for_events = status.clone();
                let sw = sw.clone();
                let selected_address_for_connect = selected_address.clone();
                let device_states_for_connect = device_states.clone();
                let known_device_store_for_connect = known_device_store.clone();
                let addr_for_connect = addr;
                let rx = std::cell::RefCell::new(rx);
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(msg) => {
                                *selected_address_for_connect.lock().unwrap_or_else(|p| {
                                    tracing::warn!("mutex poisoned — recovering");
                                    p.into_inner()
                                }) = Some(addr_for_connect);
                                device_states_for_connect
                                    .lock()
                                    .unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    })
                                    .insert(addr_for_connect, DeviceRuntimeState::new());
                                if let Err(e) = known_device_store_for_connect.add(&addr_for_connect) {
                                    warn!(%addr_for_connect, error = %e, "controller: failed to persist known device");
                                } else {
                                    debug!(%addr_for_connect, "controller: device saved to known devices store");
                                }
                                info!(%addr_for_connect, "controller: connected");
                                status.set_label(&msg);
                            }
                            Err(e) => {
                                warn!(%addr_for_connect, error = %e, "controller: connect failed");
                                status.set_label(&format!("Connect failed: {e}"));
                                sw.set_active(false);
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        warn!(%addr_for_connect, "controller: connect task channel disconnected");
                        status.set_label("Connect task failed");
                        sw.set_active(false);
                        glib::ControlFlow::Break
                    }
                });
                let temp_display = temp_display.clone();
                let humidity_display = humidity_display.clone();
                let battery_bar = battery_bar.clone();
                let battery_label = battery_label.clone();
                let summary_temp = summary_temp.clone();
                let summary_humidity = summary_humidity.clone();
                let summary_battery = summary_battery.clone();
                let device_states_for_events = device_states.clone();
                let selected_address_for_events = selected_address.clone();
                let event_rx = std::cell::RefCell::new(event_rx);
                glib::source::idle_add_local(move || match event_rx.borrow_mut().try_recv() {
                    Ok(event) => {
                        let is_selected = *selected_address_for_events.lock().unwrap_or_else(|p| {
                            tracing::warn!("mutex poisoned — recovering");
                            p.into_inner()
                        }) == Some(addr_for_connect);
                        match event {
                            ClockEvent::SensorUpdate { temperature, humidity } => {
                                debug!(%addr_for_connect, temp = %temperature.value(), hum = %humidity.value(), "SensorUpdate event received");
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                        state.temperature = Some(temperature.value().into());
                                        state.humidity = Some(humidity.value().into());
                                    } else {
                                        warn!(%addr_for_connect, "SensorUpdate: device state not found in device_states map");
                                    }
                                }
                                if is_selected {
                                    temp_display.set_display_text(&format!("{:.1}°C", temperature.value()));
                                    humidity_display.set_display_text(&format!("{:.0}%", humidity.value()));
                                    summary_temp.set_label(&format!("{:.1}°C", temperature.value()));
                                    summary_humidity.set_label(&format!("{:.0}%", humidity.value()));
                                }
                            }
                            ClockEvent::BatteryLevel { level } => {
                                let pct = level.value();
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                        state.battery_level = Some(pct);
                                    }
                                }
                                if is_selected {
                                    battery_bar.set_fraction(pct as f64 / 100.0);
                                    battery_label.set_label(&format!("{:.0}%", pct));
                                    summary_battery.set_fraction(pct as f64 / 100.0);
                                }
                            }
                            ClockEvent::Disconnected => {
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = None;
                                        state.connected = false;
                                    }
                                }
                                if is_selected {
                                    warn!("controller: selected device disconnected event received");
                                    status_for_events.set_label("Device disconnected");
                                }
                            }
                            ClockEvent::Reconnected => {
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        tracing::warn!("mutex poisoned — recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                        state.connected = true;
                                    }
                                }
                                if is_selected {
                                    info!("controller: selected device reconnected");
                                    status_for_events.set_label("Device reconnected");
                                }
                            }
                            _ => {}
                        }
                        glib::ControlFlow::Continue
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
                });
            } else {
                info!(%addr, "controller: manual disconnect");
                *selected_address.lock().unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                }) = None;
                device_states
                    .lock()
                    .unwrap_or_else(|p| {
                        tracing::warn!("mutex poisoned — recovering");
                        p.into_inner()
                    })
                    .remove(&addr);
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

        // --- Dropdown selection handler (device switching) ---
        let dropdown = self.device_dropdown.clone();
        let connect_switch = self.connect_switch.clone();
        let status = self.status_label.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let battery_bar = self.battery_bar.clone();
        let battery_label = self.battery_label.clone();
        let selected_address = self.selected_address.clone();
        let device_states = self.device_states.clone();

        let dropdown_for_handler = dropdown.clone();
        dropdown.connect_selected_notify(move |_| {
            let addr_text = dropdown_for_handler
                .selected_item()
                .and_then(|item| item.downcast::<gtk4::StringObject>().ok())
                .map(|obj| obj.string().to_string())
                .unwrap_or_default();
            if addr_text.is_empty() || addr_text == "No devices" {
                return;
            }
            let addr = match cgd1_rs::MacAddress::parse(&addr_text) {
                Ok(a) => a,
                Err(_) => return,
            };
            debug!(address = %addr, "controller: dropdown selection changed");
            let previous = *selected_address.lock().unwrap_or_else(|p| {
                tracing::warn!("mutex poisoned — recovering");
                p.into_inner()
            });
            if previous == Some(addr) {
                return;
            }
            *selected_address.lock().unwrap_or_else(|p| {
                tracing::warn!("mutex poisoned — recovering");
                p.into_inner()
            }) = Some(addr);

            let state = device_states
                .lock()
                .unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                })
                .get(&addr)
                .cloned();
            match state {
                Some(state) => {
                    if !connect_switch.is_active() {
                        connect_switch.set_active(true);
                    }
                    if let Some(t) = state.temperature {
                        temp_display.set_display_text(&format!("{:.1}°C", t));
                    } else {
                        temp_display.set_display_text("--.-°C");
                    }
                    if let Some(h) = state.humidity {
                        humidity_display.set_display_text(&format!("{:.0}%", h));
                    } else {
                        humidity_display.set_display_text("--.-%");
                    }
                    if let Some(b) = state.battery_level {
                        battery_bar.set_fraction(b as f64 / 100.0);
                        battery_label.set_label(&format!("{:.0}%", b));
                    } else {
                        battery_bar.set_fraction(0.0);
                        battery_label.set_label("--%");
                    }
                    status.set_label(&format!("Selected: {addr}"));
                }
                None => {
                    temp_display.set_display_text("");
                    humidity_display.set_display_text("");
                    battery_bar.set_fraction(0.0);
                    battery_label.set_label("--%");
                    status.set_label(&format!("Available: {addr}"));
                }
            }
        });
    }
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
        let summary_date = self.summary_date.clone();
        let summary_time = self.summary_time.clone();

        glib::timeout_add_local(Duration::from_secs(1), move || {
            let now = match glib::DateTime::now_local()
                .or_else(|_| glib::DateTime::now_utc())
                .or_else(|_| glib::DateTime::from_unix_utc(0))
            {
                Ok(dt) => dt,
                Err(e) => {
                    tracing::error!(error = ?e, "all glib DateTime constructors failed, skipping clock tick");
                    return glib::ControlFlow::Continue;
                }
            };
            let date_str = format!("{:02}.{:02}.", now.day_of_month(), now.month());
            date_display.set_display_text(&date_str);
            summary_date.set_label(&date_str);

            let time_str = format!("{:02}:{:02}", now.hour(), now.minute());
            time_display.set_display_text(&time_str);
            summary_time.set_label(&time_str);

            if temp_display.imp().text.borrow().is_empty() {
                temp_display.set_display_text("--.-°C");
            }
            if humidity_display.imp().text.borrow().is_empty() {
                humidity_display.set_display_text("--.-%");
            }

            glib::ControlFlow::Continue
        });
    }

    /// Populate the dropdown with persisted known devices on startup.
    fn populate_known_devices(&self) {
        let known = self.known_devices.lock().unwrap_or_else(|p| {
            tracing::warn!("mutex poisoned — recovering");
            p.into_inner()
        });
        if known.is_empty() {
            return;
        }
        debug!(count = known.len(), "controller: populating dropdown with persisted known devices");
        let labels: Vec<String> = known.iter().map(|addr| format!("{}", addr)).collect();
        let strs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let model = StringList::new(&strs);
        self.device_dropdown.set_model(Some(&model));
    }

    /// Start periodic auto-scan with dynamic interval.
    ///
    /// - Disconnected: scan every 60 seconds
    /// - Connected: scan every 10 minutes
    /// Also checks for stale data (no sensor/battery events for 120 seconds)
    /// and disconnects the device if it appears unresponsive.
    fn start_auto_scan(&self) {
        let scan_btn = self.scan_button.clone();
        let selected_address = self.selected_address.clone();
        let device_states = self.device_states.clone();
        let last_scan_time = self.last_scan_time.clone();
        let connect_switch = self.connect_switch.clone();
        let status = self.status_label.clone();
        let dropdown = self.device_dropdown.clone();
        let known_devices = self.known_devices.clone();
        let known_device_store = self.known_device_store.clone();

        // Try connecting to the first persisted known device immediately,
        // in parallel with the initial scan. The device may not be advertising
        // but BlueZ might still know it from a previous session.
        {
            let persisted = known_device_store.load();
            if let Some(first) = persisted.first() {
                debug!(address = %first, "controller: attempting startup connect to persisted known device");
                let known = known_devices.lock().unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                });
                if let Some(idx) = known.iter().position(|a| a == first) {
                    dropdown.set_selected(idx as u32);
                    if !connect_switch.is_active() {
                        connect_switch.set_active(true);
                    }
                }
            }
        }

        // Trigger initial scan immediately
        scan_btn.emit_clicked();
        *last_scan_time.lock().unwrap_or_else(|p| {
            tracing::warn!("mutex poisoned — recovering");
            p.into_inner()
        }) = Some(Instant::now());

        // Periodic check: scan + staleness detection
        glib::timeout_add_local(Duration::from_secs(10), move || {
            let has_connected = !device_states
                .lock()
                .unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                })
                .is_empty();
            if has_connected {
                // Check all connected devices for staleness
                let stale_addrs: Vec<MacAddress> = {
                    let states = device_states.lock().unwrap_or_else(|p| {
                        tracing::warn!("mutex poisoned — recovering");
                        p.into_inner()
                    });
                    states
                        .iter()
                        .filter(|(_, state)| state.last_data_time.map_or(false, |t| t.elapsed() > Duration::from_secs(120)))
                        .map(|(addr, _)| *addr)
                        .collect()
                };
                if !stale_addrs.is_empty() {
                    for stale_addr in &stale_addrs {
                        warn!(address = %stale_addr, "controller: no data from device for 120s, disconnecting");
                    }
                    let first_stale = stale_addrs[0];
                    status.set_label("Device unresponsive, disconnecting...");
                    if connect_switch.is_active() {
                        connect_switch.set_active(false);
                    }
                    // Remove stale devices from state
                    {
                        let mut states = device_states.lock().unwrap_or_else(|p| {
                            tracing::warn!("mutex poisoned — recovering");
                            p.into_inner()
                        });
                        for addr in &stale_addrs {
                            states.remove(addr);
                        }
                    }
                    *selected_address.lock().unwrap_or_else(|p| {
                        tracing::warn!("mutex poisoned — recovering");
                        p.into_inner()
                    }) = None;

                    // Schedule a reconnect attempt after 5 seconds
                    let dropdown = dropdown.clone();
                    let connect_switch = connect_switch.clone();
                    let _selected_address = selected_address.clone();
                    let device_states = device_states.clone();
                    let known_devices = known_devices.clone();
                    let status = status.clone();
                    glib::timeout_add_local(Duration::from_secs(5), move || {
                        // Only reconnect if nothing is connected (e.g. by auto-scan)
                        if !device_states
                            .lock()
                            .unwrap_or_else(|p| {
                                tracing::warn!("mutex poisoned — recovering");
                                p.into_inner()
                            })
                            .is_empty()
                        {
                            debug!("controller: reconnect skipped, a device is connected");
                            return glib::ControlFlow::Break;
                        }
                        // Find the device in the dropdown by address
                        let known = known_devices.lock().unwrap_or_else(|p| {
                            tracing::warn!("mutex poisoned — recovering");
                            p.into_inner()
                        });
                        if let Some(idx) = known.iter().position(|a| *a == first_stale) {
                            debug!(address = %first_stale, "controller: attempting reconnect to known device");
                            dropdown.set_selected(idx as u32);
                            status.set_label("Reconnecting...");
                            if !connect_switch.is_active() {
                                connect_switch.set_active(true);
                            }
                        } else {
                            warn!(address = %first_stale, "controller: reconnect failed, device no longer in known list");
                        }
                        glib::ControlFlow::Break
                    });
                }
            }

            let interval = if has_connected { Duration::from_secs(600) } else { Duration::from_secs(60) };

            let should_scan = {
                let last = last_scan_time.lock().unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                });
                match *last {
                    Some(t) => t.elapsed() >= interval,
                    None => true,
                }
            };

            if should_scan {
                debug!(connected = has_connected, interval_secs = interval.as_secs(), "controller: triggering periodic scan");
                *last_scan_time.lock().unwrap_or_else(|p| {
                    tracing::warn!("mutex poisoned — recovering");
                    p.into_inner()
                }) = Some(Instant::now());
                scan_btn.emit_clicked();
            }

            glib::ControlFlow::Continue
        });
    }
}

/// Create the header bar with scan button, split-button (device selector + connect), and menu.
fn create_header_bar(scan_button: &Button, device_dropdown: &DropDown, connect_switch: &Switch) -> gtk4::HeaderBar {
    let header = gtk4::HeaderBar::builder().show_title_buttons(true).build();

    // Scan button on the left
    header.pack_start(scan_button);

    // Split-button: [DropDown] [Switch] linked together
    let split_button = Box::builder().orientation(Orientation::Horizontal).spacing(0).css_classes(["linked"]).build();
    split_button.append(device_dropdown);
    split_button.append(connect_switch);
    header.pack_start(&split_button);

    let menu = gio::Menu::new();
    menu.append(Some("Alarms"), Some("app.alarms"));
    menu.append(Some("Settings"), Some("app.settings"));
    menu.append(Some("Audio"), Some("app.audio"));
    menu.append(Some("Sensor Overview"), Some("app.sensor_overview"));
    menu.append(Some("Reset Token"), Some("app.reset_token"));
    menu.append(Some("Info"), Some("app.info"));

    let menu_button = gtk4::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu).build();
    header.pack_end(&menu_button);

    header
}
