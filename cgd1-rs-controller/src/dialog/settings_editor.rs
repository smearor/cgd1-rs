use std::sync::Arc;
use std::sync::mpsc::TryRecvError;

use cgd1_rs::Brightness;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::DeviceSettings;
use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::ScreenLightDuration;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;
use cgd1_rs::Timezone;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::DropDown;
use gtk4::Frame;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Scale;
use gtk4::SpinButton;
use gtk4::StringList;
use gtk4::ToggleButton;
use gtk4::glib;
use gtk4::prelude::*;
use tracing::warn;

/// Reusable settings editor widget - can be embedded in the main window or a dialog.
#[allow(dead_code)]
pub struct SettingsEditorWidget {
    /// The container box holding all settings editor content.
    pub container: Box,
    /// Status label for operation feedback.
    pub status_label: Label,
    /// Refresh button.
    pub refresh_button: Button,
    /// Brightness scale control.
    brightness_scale: Scale,
    /// Night brightness scale control.
    night_brightness_scale: Scale,
    /// 24h format toggle.
    time_format_24h: ToggleButton,
    /// 12h format toggle.
    time_format_12h: ToggleButton,
    /// Celsius toggle.
    temp_c: ToggleButton,
    /// Fahrenheit toggle.
    temp_f: ToggleButton,
    /// English language toggle.
    lang_en: ToggleButton,
    /// Chinese language toggle.
    lang_zh: ToggleButton,
    /// Night mode toggle.
    night_mode_toggle: ToggleButton,
    /// Screen light duration spin button.
    screen_duration_spin: SpinButton,
    /// Timezone dropdown.
    timezone_dropdown: DropDown,
    /// Clock manager for device communication.
    manager: Arc<ClockManager>,
    /// Async runtime.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Connected device address.
    connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
}

/// Create a labeled settings row with a fixed-width label.
fn settings_row(label_text: &str) -> Box {
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let label = Label::builder()
        .label(label_text)
        .xalign(1.0f32)
        .width_chars(18)
        .css_classes(["settings-label"])
        .build();
    row.append(&label);
    row
}

/// Create a frame with an icon + title header.
fn settings_frame(icon_name: &str, title: &str) -> Frame {
    let header = Box::builder().orientation(Orientation::Horizontal).spacing(6).build();
    let icon = Image::builder().icon_name(icon_name).css_classes(["frame-icon"]).build();
    let label = Label::builder().label(title).css_classes(["frame-title"]).build();
    header.append(&icon);
    header.append(&label);

    Frame::builder()
        .label_widget(&header)
        .margin_top(4)
        .margin_bottom(4)
        .css_classes(["settings-frame"])
        .build()
}

impl SettingsEditorWidget {
    /// Build the settings editor content (without window chrome).
    pub fn new(manager: Arc<ClockManager>, runtime: Arc<tokio::runtime::Runtime>, connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>) -> Self {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .max_content_height(300)
            .propagate_natural_height(true)
            .build();

        let content = Box::builder().orientation(Orientation::Vertical).spacing(8).build();

        // --- Display frame ---
        let display_frame = settings_frame("nf-cod-screen-full-symbolic", "Display");
        let display_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let brightness_row = settings_row("Brightness");
        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 10.0);
        brightness_scale.set_value(80.0);
        brightness_scale.set_digits(0);
        brightness_scale.set_hexpand(true);
        brightness_scale.set_draw_value(true);
        brightness_scale.set_value_pos(gtk4::PositionType::Right);
        brightness_row.append(&brightness_scale);
        display_box.append(&brightness_row);

        let night_brightness_row = settings_row("Night Brightness");
        let night_brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 10.0);
        night_brightness_scale.set_value(30.0);
        night_brightness_scale.set_digits(0);
        night_brightness_scale.set_hexpand(true);
        night_brightness_scale.set_draw_value(true);
        night_brightness_scale.set_value_pos(gtk4::PositionType::Right);
        night_brightness_row.append(&night_brightness_scale);
        display_box.append(&night_brightness_row);

        let screen_duration_row = settings_row("Screen Timeout");
        let screen_duration_inner = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();
        let screen_duration_spin = SpinButton::with_range(0.0, 255.0, 1.0);
        screen_duration_spin.set_value(10.0);
        let screen_duration_suffix = Label::builder().label("s").css_classes(["dim-label"]).build();
        screen_duration_inner.append(&screen_duration_spin);
        screen_duration_inner.append(&screen_duration_suffix);
        screen_duration_row.append(&screen_duration_inner);
        display_box.append(&screen_duration_row);

        let night_mode_row = settings_row("Night Mode");
        let night_mode_toggle = ToggleButton::builder().label("Enabled").build();
        night_mode_toggle.set_active(true);
        night_mode_row.append(&night_mode_toggle);
        night_mode_row.append(&Box::builder().hexpand(true).build());
        display_box.append(&night_mode_row);

        display_frame.set_child(Some(&display_box));
        content.append(&display_frame);

        // --- Regional frame ---
        let regional_frame = settings_frame("nf-cod-globe-symbolic", "Regional");
        let regional_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let time_format_row = settings_row("Time Format");
        let time_format_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let time_format_24h = ToggleButton::builder().label("24h").css_classes(["segmented-btn"]).build();
        let time_format_12h = ToggleButton::builder().label("12h").css_classes(["segmented-btn"]).build();
        time_format_24h.set_active(true);
        time_format_24h.set_group(Some(&time_format_12h));
        time_format_box.append(&time_format_24h);
        time_format_box.append(&time_format_12h);
        time_format_row.append(&time_format_box);
        time_format_row.append(&Box::builder().hexpand(true).build());
        regional_box.append(&time_format_row);

        let temp_unit_row = settings_row("Temperature");
        let temp_unit_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let temp_c = ToggleButton::builder().label("°C").css_classes(["segmented-btn"]).build();
        let temp_f = ToggleButton::builder().label("°F").css_classes(["segmented-btn"]).build();
        temp_c.set_active(true);
        temp_c.set_group(Some(&temp_f));
        temp_unit_box.append(&temp_c);
        temp_unit_box.append(&temp_f);
        temp_unit_row.append(&temp_unit_box);
        temp_unit_row.append(&Box::builder().hexpand(true).build());
        regional_box.append(&temp_unit_row);

        let language_row = settings_row("Language");
        let language_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let lang_en = ToggleButton::builder().label("English").css_classes(["segmented-btn"]).build();
        let lang_zh = ToggleButton::builder().label("中文").css_classes(["segmented-btn"]).build();
        lang_en.set_active(true);
        lang_en.set_group(Some(&lang_zh));
        language_box.append(&lang_en);
        language_box.append(&lang_zh);
        language_row.append(&language_box);
        language_row.append(&Box::builder().hexpand(true).build());
        regional_box.append(&language_row);

        let timezone_entries = timezone_list();
        let timezone_labels: Vec<&str> = timezone_entries.iter().map(|(label, _)| *label).collect();
        let timezone_model = StringList::new(&timezone_labels);

        let timezone_row = settings_row("Timezone");
        let timezone_dropdown = DropDown::new(Some(timezone_model), None::<&gtk4::Expression>);
        timezone_dropdown.set_selected(0);
        timezone_dropdown.set_hexpand(true);
        timezone_row.append(&timezone_dropdown);
        regional_box.append(&timezone_row);

        let sync_tz_button = Button::builder()
            .icon_name("nf-cod-sync-symbolic")
            .label("Sync from System")
            .tooltip_text("Set timezone from the computer's local clock")
            .css_classes(["suggested-action"])
            .build();
        regional_box.append(&sync_tz_button);

        let tz_info_label = Label::builder()
            .label("The device has no DST logic. Timezone is auto-synced on connect. Reconnect after a DST change to update.")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        regional_box.append(&tz_info_label);

        regional_frame.set_child(Some(&regional_box));
        content.append(&regional_frame);

        scrolled.set_child(Some(&content));
        container.append(&scrolled);

        container.append(&gtk4::Separator::new(Orientation::Horizontal));

        let status_label = Label::builder().label("").css_classes(["dim-label"]).halign(Align::Start).hexpand(true).build();

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::Fill).build();
        let refresh_button = Button::builder()
            .icon_name("nf-cod-sync-symbolic")
            .label("Read")
            .tooltip_text("Read settings from device")
            .build();
        let apply_button = Button::builder()
            .icon_name("nf-cod-check-symbolic")
            .label("Write")
            .tooltip_text("Write settings to device")
            .css_classes(["suggested-action"])
            .build();
        button_box.append(&status_label);
        button_box.append(&refresh_button);
        button_box.append(&apply_button);
        container.append(&button_box);

        // Sync timezone from system
        {
            let timezone_dropdown = timezone_dropdown.clone();
            let status_label = status_label.clone();

            sync_tz_button.connect_clicked(move |_| {
                let offset_seconds = chrono::Local::now().offset().local_minus_utc();
                let offset_minutes = (offset_seconds / 60) as i16;
                match find_timezone_index(offset_minutes) {
                    Some(idx) => {
                        timezone_dropdown.set_selected(idx);
                        status_label.set_label(&format!("Timezone set to match system (UTC{offset_minutes:+})"));
                    }
                    None => {
                        status_label.set_label(&format!("System timezone UTC{offset_minutes:+} not in list"));
                    }
                }
            });
        }

        // Read from Device
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let brightness_scale = brightness_scale.clone();
            let night_brightness_scale = night_brightness_scale.clone();
            let time_format_24h = time_format_24h.clone();
            let time_format_12h = time_format_12h.clone();
            let temp_c = temp_c.clone();
            let temp_f = temp_f.clone();
            let lang_en = lang_en.clone();
            let lang_zh = lang_zh.clone();
            let night_mode_toggle = night_mode_toggle.clone();
            let screen_duration_spin = screen_duration_spin.clone();
            let timezone_dropdown = timezone_dropdown.clone();
            let status_label = status_label.clone();

            refresh_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };
                status_label.set_label("Reading settings...");
                let manager = manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<DeviceSettings, String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.read_settings().await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let brightness_scale = brightness_scale.clone();
                let night_brightness_scale = night_brightness_scale.clone();
                let time_format_24h = time_format_24h.clone();
                let time_format_12h = time_format_12h.clone();
                let temp_c = temp_c.clone();
                let temp_f = temp_f.clone();
                let lang_en = lang_en.clone();
                let lang_zh = lang_zh.clone();
                let night_mode_toggle = night_mode_toggle.clone();
                let screen_duration_spin = screen_duration_spin.clone();
                let timezone_dropdown = timezone_dropdown.clone();
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(settings) => {
                                brightness_scale.set_value(settings.brightness().value() as f64);
                                night_brightness_scale.set_value(settings.night_brightness().value() as f64);
                                match settings.time_format() {
                                    TimeFormat::TwentyFourHour => time_format_24h.set_active(true),
                                    TimeFormat::TwelveHour => time_format_12h.set_active(true),
                                }
                                match settings.temperature_unit() {
                                    TemperatureUnit::Celsius => temp_c.set_active(true),
                                    TemperatureUnit::Fahrenheit => temp_f.set_active(true),
                                }
                                match settings.language() {
                                    Language::English => lang_en.set_active(true),
                                    Language::Chinese => lang_zh.set_active(true),
                                }
                                night_mode_toggle.set_active(settings.night_mode_enabled());
                                screen_duration_spin.set_value(settings.screen_light_duration().seconds() as f64);
                                if let Some(idx) = find_timezone_index(settings.timezone().minutes()) {
                                    timezone_dropdown.set_selected(idx);
                                }
                                status_label.set_label("Settings loaded");
                            }
                            Err(e) => {
                                status_label.set_label(&format!("Read failed: {e}"));
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label.set_label("Read task failed");
                        glib::ControlFlow::Break
                    }
                });
            });
        }

        // Write to Device
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let brightness_scale = brightness_scale.clone();
            let night_brightness_scale = night_brightness_scale.clone();
            let time_format_12h = time_format_12h.clone();
            let temp_f = temp_f.clone();
            let lang_en = lang_en.clone();
            let night_mode_toggle = night_mode_toggle.clone();
            let screen_duration_spin = screen_duration_spin.clone();
            let timezone_dropdown = timezone_dropdown.clone();
            let status_label = status_label.clone();

            apply_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };

                let brightness = match Brightness::new(brightness_scale.value() as u8) {
                    Ok(b) => b,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid brightness: {e}"));
                        return;
                    }
                };
                let night_brightness = match Brightness::new(night_brightness_scale.value() as u8) {
                    Ok(b) => b,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid night brightness: {e}"));
                        return;
                    }
                };
                let time_format = if time_format_12h.is_active() {
                    TimeFormat::TwelveHour
                } else {
                    TimeFormat::TwentyFourHour
                };
                let temp_unit = if temp_f.is_active() {
                    TemperatureUnit::Fahrenheit
                } else {
                    TemperatureUnit::Celsius
                };
                let language = if lang_en.is_active() { Language::English } else { Language::Chinese };
                let night_mode_enabled = night_mode_toggle.is_active();
                let screen_light_duration = match ScreenLightDuration::new(screen_duration_spin.value() as u8) {
                    Ok(d) => d,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid screen duration: {e}"));
                        return;
                    }
                };
                let tz_idx = timezone_dropdown.selected() as usize;
                let timezone_entries = timezone_list();
                let timezone_minutes = timezone_entries[tz_idx].1;
                let timezone = match Timezone::from_minutes(timezone_minutes) {
                    Ok(t) => t,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid timezone: {e}"));
                        return;
                    }
                };

                status_label.set_label("Writing settings...");
                let manager = manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        let current = device.read_settings().await?;
                        let updated = DeviceSettings::new(
                            current.volume(),
                            time_format,
                            temp_unit,
                            language,
                            timezone,
                            screen_light_duration,
                            brightness,
                            night_brightness,
                            current.night_start(),
                            current.night_end(),
                            night_mode_enabled,
                            current.master_alarm_disabled(),
                            current.ringtone_signature(),
                        )?;
                        device.write_settings(&updated).await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => status_label.set_label("Settings written"),
                            Err(e) => status_label.set_label(&format!("Write failed: {e}")),
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label.set_label("Write task failed");
                        glib::ControlFlow::Break
                    }
                });
            });
        }

        let widget = Self {
            container,
            status_label: status_label.clone(),
            refresh_button: refresh_button.clone(),
            brightness_scale: brightness_scale.clone(),
            night_brightness_scale: night_brightness_scale.clone(),
            time_format_24h: time_format_24h.clone(),
            time_format_12h: time_format_12h.clone(),
            temp_c: temp_c.clone(),
            temp_f: temp_f.clone(),
            lang_en: lang_en.clone(),
            lang_zh: lang_zh.clone(),
            night_mode_toggle: night_mode_toggle.clone(),
            screen_duration_spin: screen_duration_spin.clone(),
            timezone_dropdown: timezone_dropdown.clone(),
            manager: manager.clone(),
            runtime: runtime.clone(),
            connected_address: connected_address.clone(),
        };

        widget
    }

    /// Read settings from the connected device and update the UI.
    pub fn read_settings(&self) {
        let addr = *self.connected_address.lock().unwrap_or_else(|p| {
            warn!("mutex poisoned - recovering");
            p.into_inner()
        });
        let Some(addr) = addr else {
            self.status_label.set_label("No device connected");
            return;
        };
        self.status_label.set_label("Reading settings...");
        let manager = self.manager.clone();
        let (tx, rx) = std::sync::mpsc::channel::<Result<DeviceSettings, String>>();
        self.runtime.spawn(async move {
            let result = async {
                let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                device.read_settings().await
            }
            .await;
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
        let rx = std::cell::RefCell::new(rx);
        let brightness_scale = self.brightness_scale.clone();
        let night_brightness_scale = self.night_brightness_scale.clone();
        let time_format_24h = self.time_format_24h.clone();
        let time_format_12h = self.time_format_12h.clone();
        let temp_c = self.temp_c.clone();
        let temp_f = self.temp_f.clone();
        let lang_en = self.lang_en.clone();
        let lang_zh = self.lang_zh.clone();
        let night_mode_toggle = self.night_mode_toggle.clone();
        let screen_duration_spin = self.screen_duration_spin.clone();
        let timezone_dropdown = self.timezone_dropdown.clone();
        let status_label = self.status_label.clone();
        glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
            Ok(result) => {
                match result {
                    Ok(settings) => {
                        brightness_scale.set_value(settings.brightness().value() as f64);
                        night_brightness_scale.set_value(settings.night_brightness().value() as f64);
                        match settings.time_format() {
                            TimeFormat::TwentyFourHour => time_format_24h.set_active(true),
                            TimeFormat::TwelveHour => time_format_12h.set_active(true),
                        }
                        match settings.temperature_unit() {
                            TemperatureUnit::Celsius => temp_c.set_active(true),
                            TemperatureUnit::Fahrenheit => temp_f.set_active(true),
                        }
                        match settings.language() {
                            Language::English => lang_en.set_active(true),
                            Language::Chinese => lang_zh.set_active(true),
                        }
                        night_mode_toggle.set_active(settings.night_mode_enabled());
                        screen_duration_spin.set_value(settings.screen_light_duration().seconds() as f64);
                        if let Some(idx) = find_timezone_index(settings.timezone().minutes()) {
                            timezone_dropdown.set_selected(idx);
                        }
                        status_label.set_label("Settings loaded");
                    }
                    Err(e) => {
                        status_label.set_label(&format!("Read failed: {e}"));
                    }
                }
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status_label.set_label("Read task failed");
                glib::ControlFlow::Break
            }
        });
    }
}

/// Return a list of (display_label, offset_minutes) for common timezones.
///
/// The device stores timezone in 6-minute units, so fractional offsets
/// like +5:30 (India) or +5:45 (Nepal) are supported.
fn timezone_list() -> Vec<(&'static str, i16)> {
    vec![
        ("UTC-12:00 - Baker Island", -720),
        ("UTC-11:00 - American Samoa", -660),
        ("UTC-10:00 - Honolulu", -600),
        ("UTC-09:30 - Marquesas Islands", -570),
        ("UTC-09:00 - Anchorage", -540),
        ("UTC-08:00 - Los Angeles", -480),
        ("UTC-07:00 - Denver", -420),
        ("UTC-06:00 - Chicago, Mexico City", -360),
        ("UTC-05:00 - New York, Lima", -300),
        ("UTC-04:00 - Halifax, Caracas", -240),
        ("UTC-03:30 - St. John's", -210),
        ("UTC-03:00 - Buenos Aires, São Paulo", -180),
        ("UTC-02:00 - South Georgia", -120),
        ("UTC-01:00 - Azores", -60),
        ("UTC+00:00 - London, Dublin, Lisbon", 0),
        ("UTC+01:00 - Berlin, Paris, Rome", 60),
        ("UTC+02:00 - Cairo, Athens, Helsinki", 120),
        ("UTC+03:00 - Moscow, Istanbul, Nairobi", 180),
        ("UTC+03:30 - Tehran", 210),
        ("UTC+04:00 - Dubai, Baku", 240),
        ("UTC+04:30 - Kabul", 270),
        ("UTC+05:00 - Karachi, Tashkent", 300),
        ("UTC+05:30 - Delhi, Mumbai", 330),
        ("UTC+05:45 - Kathmandu", 345),
        ("UTC+06:00 - Dhaka, Almaty", 360),
        ("UTC+06:30 - Yangon", 390),
        ("UTC+07:00 - Bangkok, Jakarta", 420),
        ("UTC+08:00 - Beijing, Singapore, Perth", 480),
        ("UTC+09:00 - Tokyo, Seoul", 540),
        ("UTC+09:30 - Adelaide, Darwin", 570),
        ("UTC+10:00 - Sydney, Melbourne", 600),
        ("UTC+10:30 - Lord Howe Island", 630),
        ("UTC+11:00 - Nouméa, Solomon Islands", 660),
        ("UTC+12:00 - Auckland, Fiji", 720),
        ("UTC+13:00 - Samoa, Tonga", 780),
        ("UTC+14:00 - Kiritimati", 840),
    ]
}

/// Find the dropdown index for a given offset in minutes.
fn find_timezone_index(offset_minutes: i16) -> Option<u32> {
    timezone_list().iter().position(|(_, mins)| *mins == offset_minutes).map(|idx| idx as u32)
}
