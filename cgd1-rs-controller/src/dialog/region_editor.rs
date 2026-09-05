use std::sync::Arc;
use std::sync::mpsc::TryRecvError;

use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::DeviceSettings;
use cgd1_rs::Language;
use cgd1_rs::MacAddress;
use cgd1_rs::TemperatureUnit;
use cgd1_rs::TimeFormat;
use cgd1_rs::Timezone;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::DropDown;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::StringList;
use gtk4::ToggleButton;
use gtk4::glib;
use gtk4::prelude::*;
use tracing::warn;

use super::common::find_timezone_index;
use super::common::settings_frame;
use super::common::settings_row;
use super::common::timezone_list;
use crate::fl;

/// Region editor widget for time format, temperature unit, language, and timezone.
#[allow(dead_code)]
pub struct RegionEditorWidget {
    /// The container box holding all region editor content.
    pub container: Box,
    /// Status label for operation feedback.
    pub status_label: Label,
    /// Refresh button.
    pub refresh_button: Button,
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
    /// Timezone dropdown.
    timezone_dropdown: DropDown,
    /// Clock manager for device communication.
    manager: Arc<ClockManager>,
    /// Async runtime.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Connected device address.
    connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
}

impl RegionEditorWidget {
    /// Build the region editor content (without window chrome).
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

        // --- Regional frame ---
        let regional_frame = settings_frame("nf-cod-globe-symbolic", &fl!("frame-regional"));
        let regional_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let time_format_row = settings_row(&fl!("label-time-format"));
        let time_format_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let time_format_24h = ToggleButton::builder().label(&fl!("toggle-24h")).css_classes(["segmented-btn"]).build();
        let time_format_12h = ToggleButton::builder().label(&fl!("toggle-12h")).css_classes(["segmented-btn"]).build();
        time_format_24h.set_active(true);
        time_format_24h.set_group(Some(&time_format_12h));
        time_format_box.append(&time_format_24h);
        time_format_box.append(&time_format_12h);
        time_format_row.append(&time_format_box);
        time_format_row.append(&Box::builder().hexpand(true).build());
        regional_box.append(&time_format_row);

        let temp_unit_row = settings_row(&fl!("label-temperature"));
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

        let language_row = settings_row(&fl!("label-language"));
        let language_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let lang_en = ToggleButton::builder().label(&fl!("toggle-english")).css_classes(["segmented-btn"]).build();
        let lang_zh = ToggleButton::builder().label(&fl!("toggle-chinese")).css_classes(["segmented-btn"]).build();
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

        let timezone_row = settings_row(&fl!("label-timezone"));
        let timezone_dropdown = DropDown::new(Some(timezone_model), None::<&gtk4::Expression>);
        timezone_dropdown.set_selected(0);
        timezone_dropdown.set_hexpand(true);
        timezone_row.append(&timezone_dropdown);
        regional_box.append(&timezone_row);

        let sync_tz_button = Button::builder()
            .icon_name("nf-cod-sync-symbolic")
            .label(&fl!("button-sync-from-system"))
            .tooltip_text(&fl!("tooltip-sync-from-system"))
            .css_classes(["suggested-action"])
            .build();
        regional_box.append(&sync_tz_button);

        let tz_info_label = Label::builder()
            .label(&fl!("info-tz-dst"))
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
            .label(&fl!("button-read"))
            .tooltip_text(&fl!("tooltip-read-region"))
            .build();
        let apply_button = Button::builder()
            .icon_name("nf-cod-check-symbolic")
            .label(&fl!("button-write"))
            .tooltip_text(&fl!("tooltip-write-region"))
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
                        status_label.set_label(&fl!("status-tz-synced", offset = offset_minutes.to_string()));
                    }
                    None => {
                        status_label.set_label(&fl!("status-tz-not-in-list", offset = offset_minutes.to_string()));
                    }
                }
            });
        }

        // Read from Device
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let time_format_24h = time_format_24h.clone();
            let time_format_12h = time_format_12h.clone();
            let temp_c = temp_c.clone();
            let temp_f = temp_f.clone();
            let lang_en = lang_en.clone();
            let lang_zh = lang_zh.clone();
            let timezone_dropdown = timezone_dropdown.clone();
            let status_label = status_label.clone();

            refresh_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label(&fl!("status-no-device-connected"));
                    return;
                };
                status_label.set_label(&fl!("status-reading-settings"));
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
                let time_format_24h = time_format_24h.clone();
                let time_format_12h = time_format_12h.clone();
                let temp_c = temp_c.clone();
                let temp_f = temp_f.clone();
                let lang_en = lang_en.clone();
                let lang_zh = lang_zh.clone();
                let timezone_dropdown = timezone_dropdown.clone();
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(settings) => {
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
                                if let Some(idx) = find_timezone_index(settings.timezone().minutes()) {
                                    timezone_dropdown.set_selected(idx);
                                }
                                status_label.set_label(&fl!("status-region-settings-loaded"));
                            }
                            Err(e) => {
                                status_label.set_label(&fl!("status-read-failed", error = e.to_string()));
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label.set_label(&fl!("status-read-task-failed"));
                        glib::ControlFlow::Break
                    }
                });
            });
        }

        // Write to Device (read-modify-write: only region fields are modified)
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let time_format_12h = time_format_12h.clone();
            let temp_f = temp_f.clone();
            let lang_en = lang_en.clone();
            let timezone_dropdown = timezone_dropdown.clone();
            let status_label = status_label.clone();

            apply_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label(&fl!("status-no-device-connected"));
                    return;
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
                let tz_idx = timezone_dropdown.selected() as usize;
                let timezone_entries = timezone_list();
                let timezone_minutes = timezone_entries[tz_idx].1;
                let timezone = match Timezone::from_minutes(timezone_minutes) {
                    Ok(t) => t,
                    Err(e) => {
                        status_label.set_label(&fl!("status-invalid-timezone", error = e.to_string()));
                        return;
                    }
                };

                status_label.set_label(&fl!("status-writing-settings"));
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
                            current.screen_light_duration(),
                            current.brightness(),
                            current.night_brightness(),
                            current.night_start(),
                            current.night_end(),
                            current.night_mode_enabled(),
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
                            Ok(()) => status_label.set_label(&fl!("status-region-settings-written")),
                            Err(e) => status_label.set_label(&fl!("status-write-failed", error = e.to_string())),
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label.set_label(&fl!("status-write-task-failed"));
                        glib::ControlFlow::Break
                    }
                });
            });
        }

        Self {
            container,
            status_label: status_label.clone(),
            refresh_button: refresh_button.clone(),
            time_format_24h: time_format_24h.clone(),
            time_format_12h: time_format_12h.clone(),
            temp_c: temp_c.clone(),
            temp_f: temp_f.clone(),
            lang_en: lang_en.clone(),
            lang_zh: lang_zh.clone(),
            timezone_dropdown: timezone_dropdown.clone(),
            manager: manager.clone(),
            runtime: runtime.clone(),
            connected_address: connected_address.clone(),
        }
    }

    /// Read region settings from the connected device and update the UI.
    pub fn read_settings(&self) {
        let addr = *self.connected_address.lock().unwrap_or_else(|p| {
            warn!("mutex poisoned - recovering");
            p.into_inner()
        });
        let Some(addr) = addr else {
            self.status_label.set_label(&fl!("status-no-device-connected"));
            return;
        };
        self.status_label.set_label(&fl!("status-reading-settings"));
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
        let time_format_24h = self.time_format_24h.clone();
        let time_format_12h = self.time_format_12h.clone();
        let temp_c = self.temp_c.clone();
        let temp_f = self.temp_f.clone();
        let lang_en = self.lang_en.clone();
        let lang_zh = self.lang_zh.clone();
        let timezone_dropdown = self.timezone_dropdown.clone();
        let status_label = self.status_label.clone();
        glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
            Ok(result) => {
                match result {
                    Ok(settings) => {
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
                        if let Some(idx) = find_timezone_index(settings.timezone().minutes()) {
                            timezone_dropdown.set_selected(idx);
                        }
                        status_label.set_label(&fl!("status-region-settings-loaded"));
                    }
                    Err(e) => {
                        status_label.set_label(&fl!("status-read-failed", error = e.to_string()));
                    }
                }
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status_label.set_label(&fl!("status-read-task-failed"));
                glib::ControlFlow::Break
            }
        });
    }
}
