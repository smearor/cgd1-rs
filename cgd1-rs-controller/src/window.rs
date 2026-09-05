use cgd1_rs::AuthToken;
use cgd1_rs::Backend;
use cgd1_rs::Brightness;
use cgd1_rs::ClockEvent;
use cgd1_rs::ClockManager;
use cgd1_rs::DiscoveredDevice;
use cgd1_rs::FileTokenStore;
use cgd1_rs::KnownDeviceStore;
use cgd1_rs::MacAddress;
use cgd1_rs::TokenStore;
use glib::ControlFlow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;
use std::time::Instant;

use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DropDown;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::StringList;
use gtk4::Switch;
use gtk4::ToggleButton;
use gtk4::Window;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use crate::config::ConfigStore;
use crate::device_runtime_state::DeviceRuntimeState;
use crate::dialog::AlarmEditorWidget;
use crate::dialog::AudioEditorWidget;
use crate::dialog::DisplayEditorWidget;
use crate::dialog::RegionEditorWidget;
use crate::dialog::SensorOverviewWidget;
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
    battery_icon: Image,
    bluetooth_icon: Image,
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
    /// Toggle button to expand/collapse the display editor panel.
    display_toggle: ToggleButton,
    /// Revealer for the collapsible display editor panel.
    display_revealer: gtk4::Revealer,
    /// Toggle button to expand/collapse the region editor panel.
    region_toggle: ToggleButton,
    /// Revealer for the collapsible region editor panel.
    region_revealer: gtk4::Revealer,
    /// Toggle button to expand/collapse the sensor overview panel.
    sensor_toggle: ToggleButton,
    /// Revealer for the collapsible sensor overview panel.
    sensor_revealer: gtk4::Revealer,
    /// Toggle button to expand/collapse the audio editor panel.
    audio_toggle: ToggleButton,
    /// Revealer for the collapsible audio editor panel.
    audio_revealer: gtk4::Revealer,
    /// Summary bar widgets.
    summary_date: Label,
    summary_time: Label,
    summary_temp: Label,
    summary_humidity: Label,
    summary_battery_icon: Image,
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
    /// Application config store.
    config_store: ConfigStore,
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
        let summary_battery_icon = Image::builder()
            .icon_name(battery_icon_name(None))
            .tooltip_text("-- %")
            .css_classes(["summary-icon"])
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let summary_date = Label::builder()
            .label("----.--.--")
            .css_classes(["summary-label"])
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let summary_time = Label::builder()
            .label("--:--")
            .css_classes(["summary-label"])
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let summary_temp_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let summary_temp_icon = Image::builder().icon_name("nf-fae-thermometer-symbolic").css_classes(["summary-icon"]).build();
        let summary_temp = Label::builder().label("--.-°C").css_classes(["summary-label"]).build();
        summary_temp_box.append(&summary_temp_icon);
        summary_temp_box.append(&summary_temp);
        let summary_humidity_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let summary_humidity_icon = Image::builder().icon_name("nf-weather-humidity-symbolic").css_classes(["summary-icon"]).build();
        let summary_humidity = Label::builder().label("--.-%").css_classes(["summary-label"]).build();
        summary_humidity_box.append(&summary_humidity_icon);
        summary_humidity_box.append(&summary_humidity);
        let summary_bt = Image::builder()
            .icon_name("nf-fa-bluetooth-symbolic")
            .css_classes(["summary-icon"])
            .hexpand(true)
            .halign(Align::Center)
            .build();
        summary_bar.append(&summary_battery_icon);
        summary_bar.append(&summary_date);
        summary_bar.append(&summary_time);
        summary_bar.append(&summary_temp_box);
        summary_bar.append(&summary_humidity_box);
        summary_bar.append(&summary_bt);

        // Alarm toggle button
        let alarm_toggle = ToggleButton::builder().label("Alarms").tooltip_text("Show / hide alarm editor").build();

        // Display editor toggle button
        let display_toggle = ToggleButton::builder().label("Display").tooltip_text("Show / hide display editor").build();

        // Region editor toggle button
        let region_toggle = ToggleButton::builder().label("Region").tooltip_text("Show / hide region editor").build();

        // Sensor overview toggle button
        let sensor_toggle = ToggleButton::builder().label("Sensors").tooltip_text("Show / hide sensor overview").build();

        // Audio editor toggle button
        let audio_toggle = ToggleButton::builder().label("Audio").tooltip_text("Show / hide audio editor").build();

        // Alarm editor revealer (collapsed by default)
        let alarm_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        // Display editor revealer (collapsed by default)
        let display_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        // Region editor revealer (collapsed by default)
        let region_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        // Sensor overview revealer (collapsed by default)
        let sensor_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        // Audio editor revealer (collapsed by default)
        let audio_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(300)
            .reveal_child(false)
            .vexpand(false)
            .build();

        // Toggle bar holding all handles
        let toggle_bar = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .halign(Align::Fill)
            .css_classes(["toggle-bar"])
            .build();
        toggle_bar.append(&alarm_toggle);
        toggle_bar.append(&display_toggle);
        toggle_bar.append(&region_toggle);
        toggle_bar.append(&sensor_toggle);
        toggle_bar.append(&audio_toggle);

        main_box.append(&full_display);
        main_box.append(&summary_bar);
        main_box.append(&toggle_bar);
        main_box.append(&sensor_revealer);
        main_box.append(&display_revealer);
        main_box.append(&region_revealer);
        main_box.append(&alarm_revealer);
        main_box.append(&audio_revealer);

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
        let config_store = ConfigStore::load_default();

        let date_display = SevenSegmentDisplay::new();
        let time_display = SevenSegmentDisplay::new();
        let temp_display = SevenSegmentDisplay::new();
        let humidity_display = SevenSegmentDisplay::new();

        let battery_icon = Image::builder()
            .icon_name(battery_icon_name(None))
            .tooltip_text("-- %")
            .css_classes(["battery-icon"])
            .build();
        let bluetooth_icon = Image::builder()
            .icon_name("nf-fa-bluetooth-symbolic")
            .css_classes(["battery-icon", "bluetooth-off"])
            .build();

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
            battery_icon,
            bluetooth_icon,
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
            display_toggle: display_toggle.clone(),
            display_revealer: display_revealer.clone(),
            region_toggle: region_toggle.clone(),
            region_revealer: region_revealer.clone(),
            sensor_toggle: sensor_toggle.clone(),
            sensor_revealer: sensor_revealer.clone(),
            audio_toggle: audio_toggle.clone(),
            audio_revealer: audio_revealer.clone(),
            summary_date: summary_date.clone(),
            summary_time: summary_time.clone(),
            summary_temp: summary_temp.clone(),
            summary_humidity: summary_humidity.clone(),
            summary_battery_icon: summary_battery_icon.clone(),
            missed_scans: Arc::new(Mutex::new(HashMap::new())),
            known_devices: Arc::new(Mutex::new(known_device_store.load())),
            last_scan_time: Arc::new(Mutex::new(None)),
            scan_battery_cache: Arc::new(Mutex::new(known_device_store.load_battery())),
            known_device_store,
            config_store,
        };

        self_.setup_layout(&main_box, &top_section, &middle_section, &bottom_section);
        self_.setup_dropdown_factory();
        self_.setup_signals();
        self_.setup_alarm_panel();
        self_.setup_display_panel();
        self_.setup_region_panel();
        self_.setup_sensor_panel();
        self_.setup_audio_panel();
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
    #[allow(dead_code)]
    pub fn runtime(&self) -> &Arc<tokio::runtime::Runtime> {
        &self.runtime
    }

    /// Get the Arc to the selected address mutex for sharing with closures.
    pub fn selected_address_arc(&self) -> Arc<Mutex<Option<MacAddress>>> {
        self.selected_address.clone()
    }

    /// Get the Arc to the device states mutex for sharing with closures.
    #[allow(dead_code)]
    pub fn device_states_arc(&self) -> Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>> {
        self.device_states.clone()
    }

    /// Get the Arc to the known devices mutex for sharing with closures.
    #[allow(dead_code)]
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
        battery_box.append(&self.battery_icon);
        top_content.append(&battery_box);

        let date_container = Box::builder().orientation(Orientation::Vertical).halign(Align::Center).hexpand(true).build();
        date_container.append(&self.date_display);
        top_content.append(&date_container);

        let bt_box = Box::builder().orientation(Orientation::Vertical).halign(Align::End).build();
        bt_box.append(&self.bluetooth_icon);
        top_content.append(&bt_box);

        top.append(&top_content);

        // Middle section: ~60% of window height - time display
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

        // Bottom section: ~25% of window height - sensors
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
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .halign(Align::Center)
            .hexpand(true)
            .build();
        let temp_icon = Image::builder()
            .icon_name("nf-fae-thermometer-symbolic")
            .css_classes(["sensor-icon"])
            .hexpand(false)
            .build();
        temp_box.append(&temp_icon);
        temp_box.append(&self.temp_display);
        bottom_content.append(&temp_box);

        let humidity_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .halign(Align::Center)
            .hexpand(true)
            .build();
        let humidity_icon = Image::builder()
            .icon_name("nf-weather-humidity-symbolic")
            .css_classes(["sensor-icon"])
            .hexpand(false)
            .build();
        humidity_box.append(&humidity_icon);
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
        let display_toggle = self.display_toggle.clone();
        let region_toggle = self.region_toggle.clone();
        let sensor_toggle = self.sensor_toggle.clone();
        let audio_toggle = self.audio_toggle.clone();
        let editor = std::rc::Rc::new(editor);

        self.alarm_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            if expanded {
                display_toggle.set_active(false);
                region_toggle.set_active(false);
                sensor_toggle.set_active(false);
                audio_toggle.set_active(false);
            }
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
            if expanded {
                editor.read_alarms();
            }
        });
    }

    /// Set up the collapsible display editor panel with compact summary bar.
    fn setup_display_panel(&self) {
        let editor = DisplayEditorWidget::new(self.manager.clone(), self.runtime.clone(), self.selected_address.clone(), self.config_store.clone());
        self.display_revealer.set_child(Some(&editor.container));

        let full_display = self.full_display.clone();
        let summary_bar = self.summary_bar.clone();
        let revealer = self.display_revealer.clone();
        let alarm_toggle = self.alarm_toggle.clone();
        let region_toggle = self.region_toggle.clone();
        let sensor_toggle = self.sensor_toggle.clone();
        let audio_toggle = self.audio_toggle.clone();
        let editor = std::rc::Rc::new(editor);

        self.display_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            if expanded {
                alarm_toggle.set_active(false);
                region_toggle.set_active(false);
                sensor_toggle.set_active(false);
                audio_toggle.set_active(false);
            }
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
            if expanded {
                editor.read_settings();
            }
        });
    }

    /// Set up the collapsible region editor panel with compact summary bar.
    fn setup_region_panel(&self) {
        let editor = RegionEditorWidget::new(self.manager.clone(), self.runtime.clone(), self.selected_address.clone());
        self.region_revealer.set_child(Some(&editor.container));

        let full_display = self.full_display.clone();
        let summary_bar = self.summary_bar.clone();
        let revealer = self.region_revealer.clone();
        let alarm_toggle = self.alarm_toggle.clone();
        let display_toggle = self.display_toggle.clone();
        let sensor_toggle = self.sensor_toggle.clone();
        let audio_toggle = self.audio_toggle.clone();
        let editor = std::rc::Rc::new(editor);

        self.region_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            if expanded {
                alarm_toggle.set_active(false);
                display_toggle.set_active(false);
                sensor_toggle.set_active(false);
                audio_toggle.set_active(false);
            }
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
            if expanded {
                editor.read_settings();
            }
        });
    }

    /// Set up the collapsible sensor overview panel with compact summary bar.
    fn setup_sensor_panel(&self) {
        let editor = SensorOverviewWidget::new(self.device_states.clone(), self.known_devices.clone());
        self.sensor_revealer.set_child(Some(&editor.container));

        let full_display = self.full_display.clone();
        let summary_bar = self.summary_bar.clone();
        let revealer = self.sensor_revealer.clone();
        let alarm_toggle = self.alarm_toggle.clone();
        let display_toggle = self.display_toggle.clone();
        let region_toggle = self.region_toggle.clone();
        let audio_toggle = self.audio_toggle.clone();
        let editor = std::rc::Rc::new(editor);
        let device_states = self.device_states.clone();
        let known_devices = self.known_devices.clone();

        self.sensor_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            if expanded {
                alarm_toggle.set_active(false);
                display_toggle.set_active(false);
                region_toggle.set_active(false);
                audio_toggle.set_active(false);
                editor.populate(&device_states, &known_devices);
            }
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
        });
    }

    /// Set up the collapsible audio editor panel with compact summary bar.
    fn setup_audio_panel(&self) {
        let editor = AudioEditorWidget::new(self.manager.clone(), self.runtime.clone(), self.selected_address.clone());
        self.audio_revealer.set_child(Some(&editor.container));

        let full_display = self.full_display.clone();
        let summary_bar = self.summary_bar.clone();
        let revealer = self.audio_revealer.clone();
        let alarm_toggle = self.alarm_toggle.clone();
        let display_toggle = self.display_toggle.clone();
        let region_toggle = self.region_toggle.clone();
        let sensor_toggle = self.sensor_toggle.clone();

        self.audio_toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            if expanded {
                alarm_toggle.set_active(false);
                display_toggle.set_active(false);
                region_toggle.set_active(false);
                sensor_toggle.set_active(false);
            }
            full_display.set_visible(!expanded);
            summary_bar.set_visible(expanded);
            revealer.set_vexpand(expanded);
            revealer.set_reveal_child(expanded);
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

            let bt_icon = Image::builder()
                .icon_name("nf-fa-bluetooth-symbolic")
                .css_classes(["clock-bluetooth-label"])
                .build();
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
                        warn!("mutex poisoned - recovering");
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
        let battery_icon_for_scan = self.battery_icon.clone();
        let summary_battery_icon_for_scan = self.summary_battery_icon.clone();
        let selected_address_for_scan = self.selected_address.clone();
        let scan_battery_cache_for_scan = self.scan_battery_cache.clone();

        scan_btn_for_connect.connect_clicked(move |_| {
            (*scan_btn).set_sensitive(false);
            status.set_label("Scanning...");
            debug!("controller: scan triggered");
            let manager = manager.clone();
            let _runtime_keepalive = runtime_arc.clone();
            let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<DiscoveredDevice>, String>>();
            runtime.spawn(async move {
                let scanner = manager.scanner();
                let result = scanner.scan_active(Duration::from_secs(10)).await;
                let mapped = result.map_err(|e| e.to_string());
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
            let battery_icon_for_scan = battery_icon_for_scan.clone();
            let summary_battery_icon_for_scan = summary_battery_icon_for_scan.clone();
            let selected_address_for_scan = selected_address_for_scan.clone();
            let scan_battery_cache_for_scan = scan_battery_cache_for_scan.clone();
            let rx = std::cell::RefCell::new(rx);
            glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                Ok(result) => {
                    match result {
                        Ok(found) => {
                            let found_addrs: Vec<MacAddress> = found.iter().map(|d| d.address).collect();
                            let connected_addrs: Vec<MacAddress> = {
                                let states = device_states_for_scan.lock().unwrap_or_else(|p| {
                                    warn!("mutex poisoned - recovering");
                                    p.into_inner()
                                });
                                states.iter().filter(|(_, s)| s.connected).map(|(a, _)| *a).collect()
                            };
                            debug!(found = found_addrs.len(), connected = connected_addrs.len(), "controller: scan complete");
                            {
                                let persisted = known_device_store_for_scan.load();
                                let mut missed = missed_scans.lock().unwrap_or_else(|p| {
                                    warn!("mutex poisoned - recovering");
                                    p.into_inner()
                                });
                                let mut known = known_devices.lock().unwrap_or_else(|p| {
                                    warn!("mutex poisoned - recovering");
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
                                    warn!("mutex poisoned - recovering");
                                    p.into_inner()
                                });
                                known
                                    .iter()
                                    .map(|addr| {
                                        if let Some(d) = found.iter().find(|d| d.address == *addr) {
                                            let rssi = d.rssi.map(|r| format!(" ({} dBm)", r)).unwrap_or_default();
                                            format!("{}{}", d.address, rssi)
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
                                            warn!("mutex poisoned - recovering");
                                            p.into_inner()
                                        });
                                        known.iter().position(|a| found_addrs.contains(a))
                                    } {
                                        dropdown.set_selected(idx as u32);
                                        connect_switch.set_active(true);
                                    }
                                }
                            }

                            // Update sensor data from advertising data for all found devices.
                            let selected_addr = *selected_address_for_scan.lock().unwrap_or_else(|p| {
                                warn!("mutex poisoned - recovering");
                                p.into_inner()
                            });
                            for d in &found {
                                let addr = d.address;
                                let (temp, hum, battery) = d
                                    .advertisement
                                    .as_ref()
                                    .map_or((None, None, None), |a| (Some(a.temperature), Some(a.humidity), Some(a.battery)));

                                // Cache battery in memory and persist to disk.
                                if let Some(level) = battery {
                                    scan_battery_cache_for_scan
                                        .lock()
                                        .unwrap_or_else(|p| {
                                            warn!("mutex poisoned - recovering");
                                            p.into_inner()
                                        })
                                        .insert(addr, level.value());
                                    if let Err(e) = known_device_store_for_scan.save_battery(&addr, level.value()) {
                                        warn!(address = %addr, error = %e, "failed to persist battery level");
                                    }
                                }

                                // Update device_states with scan data for all devices
                                // (connected or not) so sensor overview shows values.
                                let mut states = device_states_for_scan.lock().unwrap_or_else(|p| {
                                    warn!("mutex poisoned - recovering");
                                    p.into_inner()
                                });
                                let state = states.entry(addr).or_insert_with(|| DeviceRuntimeState {
                                    connected: false,
                                    last_data_time: Some(Instant::now()),
                                    temperature: None,
                                    humidity: None,
                                    battery_level: None,
                                });
                                if let Some(t) = temp {
                                    state.temperature = Some(t);
                                }
                                if let Some(h) = hum {
                                    state.humidity = Some(h);
                                }
                                if let Some(level) = battery {
                                    state.battery_level = Some(level);
                                    state.last_data_time = Some(Instant::now());
                                    debug!(address = %addr, battery = %level.value(), "scan: updated battery from advertising");
                                    if Some(addr) == selected_addr {
                                        set_battery_icon(&battery_icon_for_scan, Some(level.value()));
                                        set_battery_icon(&summary_battery_icon_for_scan, Some(level.value()));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            status.set_label(&format!("Scan failed: {e}"));
                        }
                    }
                    (*scan_btn).set_sensitive(true);
                    ControlFlow::Break
                }
                Err(TryRecvError::Empty) => ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    (*scan_btn).set_sensitive(true);
                    status.set_label("Scan task failed");
                    ControlFlow::Break
                }
            });
        });

        // --- Connect Switch handler ---
        let runtime_arc = self.runtime.clone();
        let runtime = runtime_arc.handle().clone();
        let manager = self.manager.clone();
        let dropdown = self.device_dropdown.clone();
        let connect_switch = self.connect_switch.clone();
        let full_display = self.full_display.clone();
        let status = self.status_label.clone();
        let token_store = self.token_store.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let time_display = self.time_display.clone();
        let date_display = self.date_display.clone();
        let battery_icon = self.battery_icon.clone();
        let bluetooth_icon = self.bluetooth_icon.clone();
        let summary_temp = self.summary_temp.clone();
        let summary_humidity = self.summary_humidity.clone();
        let summary_battery_icon = self.summary_battery_icon.clone();
        let selected_address = self.selected_address.clone();
        let device_states = self.device_states.clone();
        let known_device_store = self.known_device_store.clone();
        let scan_battery_cache_for_connect = self.scan_battery_cache.clone();
        let config_store_for_connect = self.config_store.clone();

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
                bluetooth_icon.remove_css_class("bluetooth-off");
                bluetooth_icon.add_css_class("bluetooth-blinking");
                let manager = manager.clone();
                let token_store = token_store.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
                let (event_tx, event_rx) = std::sync::mpsc::channel::<ClockEvent>();
                let scan_battery_cache = scan_battery_cache_for_connect.clone();
                let config_store = config_store_for_connect.clone();
                runtime.spawn(async move {
                    let token_result = token_store.load_or_generate(&addr);
                    let is_new_token = token_result.is_new();
                    let token: AuthToken = (*token_result).clone();
                    if is_new_token {
                        warn!(%addr, "no stored token found, generated new random token - device must be unpaired (factory reset) to accept it");
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
                                    warn!("mutex poisoned - recovering");
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
                                // Visual feedback: triple blink with decreasing brightness.
                                // Spawned as a separate task so the event forwarding loop
                                // is not delayed by the ~8s blink sequence.
                                if config_store.blink_on_connect() {
                                    let blink_device = device.clone();
                                    tokio::spawn(async move {
                                        tokio::time::sleep(Duration::from_millis(2000)).await;
                                        for &level in &[20u8, 50u8, 80u8] {
                                            if let Err(e) = blink_device.set_brightness(Brightness::new(level).unwrap_or(Brightness::MAX)).await {
                                                warn!(%addr, error = %e, "connect: failed to set brightness for visual feedback");
                                                break;
                                            }
                                            tokio::time::sleep(Duration::from_millis(2000)).await;
                                        }
                                    });
                                }
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
                let bluetooth_icon_for_connect = bluetooth_icon.clone();
                let selected_address_for_connect = selected_address.clone();
                let device_states_for_connect = device_states.clone();
                let known_device_store_for_connect = known_device_store.clone();
                let addr_for_connect = addr;
                let dropdown_for_result = dropdown.clone();
                let rx = std::cell::RefCell::new(rx);
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(msg) => {
                                *selected_address_for_connect.lock().unwrap_or_else(|p| {
                                    warn!("mutex poisoned - recovering");
                                    p.into_inner()
                                }) = Some(addr_for_connect);
                                device_states_for_connect
                                    .lock()
                                    .unwrap_or_else(|p| {
                                        warn!("mutex poisoned - recovering");
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
                                bluetooth_icon_for_connect.remove_css_class("bluetooth-blinking");
                                bluetooth_icon_for_connect.remove_css_class("bluetooth-off");
                                // Refresh dropdown to update connection dot
                                if let Some(model) = dropdown_for_result.model() {
                                    dropdown_for_result.set_model(Some(&model));
                                }
                            }
                            Err(e) => {
                                warn!(%addr_for_connect, error = %e, "controller: connect failed");
                                status.set_label(&format!("Connect failed: {e}"));
                                bluetooth_icon_for_connect.remove_css_class("bluetooth-blinking");
                                bluetooth_icon_for_connect.add_css_class("bluetooth-off");
                                sw.set_active(false);
                            }
                        }
                        ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        warn!(%addr_for_connect, "controller: connect task channel disconnected");
                        status.set_label("Connect task failed");
                        bluetooth_icon_for_connect.remove_css_class("bluetooth-blinking");
                        bluetooth_icon_for_connect.add_css_class("bluetooth-off");
                        sw.set_active(false);
                        ControlFlow::Break
                    }
                });
                let temp_display = temp_display.clone();
                let humidity_display = humidity_display.clone();
                let battery_icon = battery_icon.clone();
                let summary_temp = summary_temp.clone();
                let summary_humidity = summary_humidity.clone();
                let summary_battery_icon = summary_battery_icon.clone();
                let bluetooth_icon_for_events = bluetooth_icon.clone();
                let device_states_for_events = device_states.clone();
                let selected_address_for_events = selected_address.clone();
                let dropdown_for_events = dropdown.clone();
                let full_display_for_events = full_display.clone();
                let event_rx = std::cell::RefCell::new(event_rx);
                glib::source::idle_add_local(move || match event_rx.borrow_mut().try_recv() {
                    Ok(event) => {
                        let is_selected = *selected_address_for_events.lock().unwrap_or_else(|p| {
                            warn!("mutex poisoned - recovering");
                            p.into_inner()
                        }) == Some(addr_for_connect);
                        match event {
                            ClockEvent::SensorUpdate { temperature, humidity } => {
                                debug!(%addr_for_connect, temp = %temperature.value(), hum = %humidity.value(), "SensorUpdate event received");
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        warn!("mutex poisoned - recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                        state.temperature = Some(temperature);
                                        state.humidity = Some(humidity);
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
                                        warn!("mutex poisoned - recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                        state.battery_level = Some(level);
                                    }
                                }
                                if is_selected {
                                    set_battery_icon(&battery_icon, Some(pct));
                                    set_battery_icon(&summary_battery_icon, Some(pct));
                                }
                            }
                            ClockEvent::AlarmTriggered { slot } => {
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        warn!("mutex poisoned - recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = Some(Instant::now());
                                    }
                                }
                                if is_selected {
                                    let msg = format!("Alarm {} triggered", slot.value());
                                    info!(%addr_for_connect, slot = slot.value(), "alarm triggered on device");
                                    status_for_events.set_label(&msg);
                                    full_display_for_events.add_css_class("alarm-flash");
                                    let display = full_display_for_events.clone();
                                    glib::source::timeout_add_local_once(Duration::from_millis(2000), move || {
                                        display.remove_css_class("alarm-flash");
                                    });
                                }
                            }
                            ClockEvent::Disconnected => {
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        warn!("mutex poisoned - recovering");
                                        p.into_inner()
                                    });
                                    if let Some(state) = states.get_mut(&addr_for_connect) {
                                        state.last_data_time = None;
                                        state.connected = false;
                                    }
                                }
                                if is_selected {
                                    warn!("controller: selected device disconnected event received");
                                    status_for_events.set_label("Reconnecting...");
                                    bluetooth_icon_for_events.remove_css_class("bluetooth-off");
                                    bluetooth_icon_for_events.add_css_class("bluetooth-blinking");
                                }
                                if let Some(model) = dropdown_for_events.model() {
                                    dropdown_for_events.set_model(Some(&model));
                                }
                            }
                            ClockEvent::Reconnected => {
                                {
                                    let mut states = device_states_for_events.lock().unwrap_or_else(|p| {
                                        warn!("mutex poisoned - recovering");
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
                                    bluetooth_icon_for_events.remove_css_class("bluetooth-blinking");
                                    bluetooth_icon_for_events.remove_css_class("bluetooth-off");
                                }
                                if let Some(model) = dropdown_for_events.model() {
                                    dropdown_for_events.set_model(Some(&model));
                                }
                            }
                            _ => {}
                        }
                        ControlFlow::Continue
                    }
                    Err(TryRecvError::Empty) => ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => ControlFlow::Break,
                });
            } else {
                info!(%addr, "controller: manual disconnect");
                *selected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                }) = None;
                device_states
                    .lock()
                    .unwrap_or_else(|p| {
                        warn!("mutex poisoned - recovering");
                        p.into_inner()
                    })
                    .get_mut(&addr)
                    .map(|s| s.connected = false);
                let manager = manager.clone();
                runtime.spawn(async move {
                    let _ = manager.disconnect(&addr).await;
                });
                temp_display.set_display_text("");
                humidity_display.set_display_text("");
                time_display.set_display_text("");
                date_display.set_display_text("");
                set_battery_icon(&battery_icon, None);
                bluetooth_icon.remove_css_class("bluetooth-blinking");
                bluetooth_icon.add_css_class("bluetooth-off");
                status.set_label("Disconnected");
                // Refresh dropdown to update connection dot
                if let Some(model) = dropdown.model() {
                    dropdown.set_model(Some(&model));
                }
            }
        });

        // --- Dropdown selection handler (device switching) ---
        let dropdown = self.device_dropdown.clone();
        let connect_switch = self.connect_switch.clone();
        let status = self.status_label.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let battery_icon = self.battery_icon.clone();
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
                warn!("mutex poisoned - recovering");
                p.into_inner()
            });
            if previous == Some(addr) {
                return;
            }
            *selected_address.lock().unwrap_or_else(|p| {
                warn!("mutex poisoned - recovering");
                p.into_inner()
            }) = Some(addr);

            let state = device_states
                .lock()
                .unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
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
                        temp_display.set_display_text(&format!("{:.1}°C", t.value()));
                    } else {
                        temp_display.set_display_text("--.-°C");
                    }
                    if let Some(h) = state.humidity {
                        humidity_display.set_display_text(&format!("{:.0}%", h.value()));
                    } else {
                        humidity_display.set_display_text("--.-%");
                    }
                    if let Some(b) = state.battery_level {
                        set_battery_icon(&battery_icon, Some(b.value()));
                    } else {
                        set_battery_icon(&battery_icon, None);
                    }
                    status.set_label(&format!("Selected: {addr}"));
                }
                None => {
                    temp_display.set_display_text("");
                    humidity_display.set_display_text("");
                    set_battery_icon(&battery_icon, None);
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
                return ControlFlow::Continue;
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
            ControlFlow::Continue
        });
    }

    fn start_clock_tick(&self) {
        let date_display = self.date_display.clone();
        let time_display = self.time_display.clone();
        let temp_display = self.temp_display.clone();
        let humidity_display = self.humidity_display.clone();
        let summary_date = self.summary_date.clone();
        let summary_time = self.summary_time.clone();

        let mut colon_visible = true;

        glib::timeout_add_local(Duration::from_secs(1), move || {
            let now = match glib::DateTime::now_local()
                .or_else(|_| glib::DateTime::now_utc())
                .or_else(|_| glib::DateTime::from_unix_utc(0))
            {
                Ok(dt) => dt,
                Err(e) => {
                    error!(error = ?e, "all glib DateTime constructors failed, skipping clock tick");
                    return ControlFlow::Continue;
                }
            };
            let date_str = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day_of_month());
            date_display.set_display_text(&date_str);
            summary_date.set_label(&date_str);

            let sep = if colon_visible { ":" } else { " " };
            let time_str = format!("{:02}{sep}{:02}", now.hour(), now.minute());
            time_display.set_display_text(&time_str);
            summary_time.set_label(&time_str);
            colon_visible = !colon_visible;

            if temp_display.imp().text.borrow().is_empty() {
                temp_display.set_display_text("--.-°C");
            }
            if humidity_display.imp().text.borrow().is_empty() {
                humidity_display.set_display_text("--.-%");
            }

            ControlFlow::Continue
        });
    }

    /// Populate the dropdown with persisted known devices on startup.
    fn populate_known_devices(&self) {
        let known = self.known_devices.lock().unwrap_or_else(|p| {
            warn!("mutex poisoned - recovering");
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
                    warn!("mutex poisoned - recovering");
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
            warn!("mutex poisoned - recovering");
            p.into_inner()
        }) = Some(Instant::now());

        // Periodic check: scan + staleness detection
        glib::timeout_add_local(Duration::from_secs(10), move || {
            let has_connected = device_states
                .lock()
                .unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                })
                .values()
                .any(|s| s.connected);
            if has_connected {
                // Check all connected devices for staleness
                let stale_addrs: Vec<MacAddress> = {
                    let states = device_states.lock().unwrap_or_else(|p| {
                        warn!("mutex poisoned - recovering");
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
                    // Mark stale devices as disconnected
                    {
                        let mut states = device_states.lock().unwrap_or_else(|p| {
                            warn!("mutex poisoned - recovering");
                            p.into_inner()
                        });
                        for addr in &stale_addrs {
                            if let Some(state) = states.get_mut(addr) {
                                state.connected = false;
                            }
                        }
                    }
                    *selected_address.lock().unwrap_or_else(|p| {
                        warn!("mutex poisoned - recovering");
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
                        if device_states
                            .lock()
                            .unwrap_or_else(|p| {
                                warn!("mutex poisoned - recovering");
                                p.into_inner()
                            })
                            .values()
                            .any(|s| s.connected)
                        {
                            debug!("controller: reconnect skipped, a device is connected");
                            return ControlFlow::Break;
                        }
                        // Find the device in the dropdown by address
                        let known = known_devices.lock().unwrap_or_else(|p| {
                            warn!("mutex poisoned - recovering");
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
                        ControlFlow::Break
                    });
                }
            }

            let interval = if has_connected { Duration::from_secs(600) } else { Duration::from_secs(60) };

            let should_scan = {
                let last = last_scan_time.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
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
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                }) = Some(Instant::now());
                scan_btn.emit_clicked();
            }

            ControlFlow::Continue
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
    menu.append(Some("Reset Token"), Some("app.reset_token"));
    menu.append(Some("Info"), Some("app.info"));

    let menu_button = gtk4::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu).build();
    header.pack_end(&menu_button);

    header
}

/// Maps a battery level (0–100) to the appropriate Nerd Font battery icon name.
fn battery_icon_name(level: Option<u8>) -> &'static str {
    match level {
        Some(l) if l >= 87 => "nf-fa-battery-full-symbolic",
        Some(l) if l >= 62 => "nf-fa-battery-three-quarters-symbolic",
        Some(l) if l >= 37 => "nf-fa-battery-half-symbolic",
        Some(l) if l >= 12 => "nf-fa-battery-quarter-symbolic",
        Some(_) => "nf-fa-battery-empty-symbolic",
        None => "nf-fa-battery-empty-symbolic",
    }
}

/// Update a battery icon's icon name and tooltip text from a battery level.
fn set_battery_icon(icon: &Image, level: Option<u8>) {
    icon.set_icon_name(Some(battery_icon_name(level)));
    icon.set_tooltip_text(level.map(|l| format!("{} %", l)).as_deref());
}
