use std::sync::Arc;
use std::sync::mpsc::TryRecvError;

use cgd1_rs::Brightness;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::ClockTime;
use cgd1_rs::DeviceSettings;
use cgd1_rs::MacAddress;
use cgd1_rs::ScreenLightDuration;

use gtk4::Box;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Scale;
use gtk4::SpinButton;
use gtk4::Switch;
use gtk4::glib;
use gtk4::prelude::*;
use tracing::warn;

use super::TimeEntry;
use super::common::settings_frame;
use super::common::settings_row;
use crate::config::ConfigStore;
use crate::config::TimeBasedBlink;
use crate::fl;

/// Display editor widget for brightness, night brightness, screen timeout, and night mode.
#[allow(dead_code)]
pub struct DisplayEditorWidget {
    /// The container box holding all display editor content.
    pub container: Box,
    /// Status label for operation feedback.
    pub status_label: Label,
    /// Refresh button.
    pub refresh_button: Button,
    /// Brightness scale control.
    brightness_scale: Scale,
    /// Night brightness scale control.
    night_brightness_scale: Scale,
    /// Night mode switch.
    night_mode_switch: Switch,
    /// Screen light duration spin button.
    screen_duration_spin: SpinButton,
    /// Night mode start time entry.
    night_start_entry: TimeEntry,
    /// Night mode end time entry.
    night_end_entry: TimeEntry,
    /// Clock manager for device communication.
    manager: Arc<ClockManager>,
    /// Async runtime.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Connected device address.
    connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
    /// Application config store.
    config_store: ConfigStore,
}

impl DisplayEditorWidget {
    /// Build the display editor content (without window chrome).
    pub fn new(
        manager: Arc<ClockManager>,
        runtime: Arc<tokio::runtime::Runtime>,
        connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
        config_store: ConfigStore,
    ) -> Self {
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
        let display_frame = settings_frame("nf-cod-screen-full-symbolic", &fl!("frame-display"));
        let display_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let brightness_row = settings_row(&fl!("label-brightness"));
        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 15.0, 1.0);
        brightness_scale.set_value(8.0);
        brightness_scale.set_digits(0);
        brightness_scale.set_hexpand(true);
        brightness_scale.set_draw_value(true);
        brightness_scale.set_value_pos(gtk4::PositionType::Right);
        for i in 0..=15 {
            brightness_scale.add_mark(i as f64, gtk4::PositionType::Bottom, None);
        }
        brightness_row.append(&brightness_scale);
        display_box.append(&brightness_row);

        let screen_duration_row = settings_row(&fl!("label-screen-timeout"));
        let screen_duration_inner = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();
        let screen_duration_spin = SpinButton::with_range(0.0, 255.0, 1.0);
        screen_duration_spin.set_value(10.0);
        let screen_duration_suffix = Label::builder().label("s").css_classes(["dim-label"]).build();
        screen_duration_inner.append(&screen_duration_spin);
        screen_duration_inner.append(&screen_duration_suffix);
        screen_duration_row.append(&screen_duration_inner);
        display_box.append(&screen_duration_row);

        let blink_row = settings_row(&fl!("label-blink-on-connect"));
        let blink_switch = Switch::builder().tooltip_text(&fl!("tooltip-blink-on-connect")).build();
        blink_switch.set_active(config_store.blink_on_connect());
        let blink_config = config_store.clone();
        blink_switch.connect_state_set(move |_, state| {
            blink_config.set_blink_on_connect(state);
            glib::Propagation::Proceed
        });
        blink_row.append(&blink_switch);
        blink_row.append(&Box::builder().hexpand(true).build());
        display_box.append(&blink_row);

        let time_blink_row = settings_row(&fl!("label-time-based-blink"));
        let time_blink_scale = Scale::with_range(Orientation::Horizontal, 0.0, 4.0, 1.0);
        time_blink_scale.set_value(config_store.time_based_blink().index() as f64);
        time_blink_scale.set_digits(0);
        time_blink_scale.set_round_digits(0);
        time_blink_scale.set_hexpand(true);
        time_blink_scale.set_draw_value(false);
        for variant in TimeBasedBlink::ALL {
            time_blink_scale.add_mark(variant.index() as f64, gtk4::PositionType::Bottom, Some(&variant.label()));
        }
        let time_blink_config = config_store.clone();
        time_blink_scale.connect_value_changed(move |scale| {
            if let Some(variant) = TimeBasedBlink::from_index(scale.value() as usize) {
                time_blink_config.set_time_based_blink(variant);
            }
        });
        time_blink_row.append(&time_blink_scale);
        display_box.append(&time_blink_row);

        display_frame.set_child(Some(&display_box));
        content.append(&display_frame);

        // --- Night Mode frame ---
        let night_frame = settings_frame("nf-oct-moon-symbolic", &fl!("frame-night-mode"));
        let night_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let night_mode_row = settings_row(&fl!("label-night-mode"));
        let night_mode_switch = Switch::builder().build();
        night_mode_switch.set_active(true);
        night_mode_row.append(&night_mode_switch);
        night_mode_row.append(&Box::builder().hexpand(true).build());
        night_box.append(&night_mode_row);

        let night_brightness_row = settings_row(&fl!("label-night-brightness"));
        let night_brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 15.0, 1.0);
        night_brightness_scale.set_value(3.0);
        night_brightness_scale.set_digits(0);
        night_brightness_scale.set_hexpand(true);
        night_brightness_scale.set_draw_value(true);
        night_brightness_scale.set_value_pos(gtk4::PositionType::Right);
        for i in 0..=15 {
            night_brightness_scale.add_mark(i as f64, gtk4::PositionType::Bottom, None);
        }
        night_brightness_row.append(&night_brightness_scale);
        night_box.append(&night_brightness_row);

        // Night Start and Night End side by side in a horizontal row.
        let night_time_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        let night_start_label = Label::builder()
            .label(&fl!("label-night-start"))
            .xalign(1.0f32)
            .width_chars(18)
            .css_classes(["settings-label"])
            .build();
        let night_start_entry = TimeEntry::new();
        night_start_entry.set_time(22, 0);
        let night_end_label = Label::builder()
            .label(&fl!("label-night-end"))
            .xalign(1.0f32)
            .width_chars(18)
            .css_classes(["settings-label"])
            .build();
        let night_end_entry = TimeEntry::new();
        night_end_entry.set_time(7, 0);
        night_time_row.append(&night_start_label);
        night_time_row.append(night_start_entry.widget());
        night_time_row.append(&night_end_label);
        night_time_row.append(night_end_entry.widget());
        night_time_row.append(&Box::builder().hexpand(true).build());
        night_box.append(&night_time_row);

        // Grey out night mode controls when the switch is off.
        night_mode_switch
            .bind_property("active", &night_brightness_scale, "sensitive")
            .sync_create()
            .build();
        night_mode_switch
            .bind_property("active", night_start_entry.widget(), "sensitive")
            .sync_create()
            .build();
        night_mode_switch
            .bind_property("active", night_end_entry.widget(), "sensitive")
            .sync_create()
            .build();

        night_frame.set_child(Some(&night_box));
        content.append(&night_frame);

        scrolled.set_child(Some(&content));
        container.append(&scrolled);

        container.append(&gtk4::Separator::new(Orientation::Horizontal));

        let status_label = Label::builder()
            .label("")
            .css_classes(["dim-label"])
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(gtk4::Align::Fill).build();
        let refresh_button = Button::builder()
            .icon_name("nf-cod-sync-symbolic")
            .label(&fl!("button-read"))
            .tooltip_text(&fl!("tooltip-read-display"))
            .build();
        let apply_button = Button::builder()
            .icon_name("nf-cod-check-symbolic")
            .label(&fl!("button-write"))
            .tooltip_text(&fl!("tooltip-write-display"))
            .css_classes(["suggested-action"])
            .build();
        button_box.append(&status_label);
        button_box.append(&refresh_button);
        button_box.append(&apply_button);
        container.append(&button_box);

        // Read from Device
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let brightness_scale = brightness_scale.clone();
            let night_brightness_scale = night_brightness_scale.clone();
            let night_mode_switch = night_mode_switch.clone();
            let screen_duration_spin = screen_duration_spin.clone();
            let night_start_entry = night_start_entry.clone();
            let night_end_entry = night_end_entry.clone();
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
                let brightness_scale = brightness_scale.clone();
                let night_brightness_scale = night_brightness_scale.clone();
                let night_mode_switch = night_mode_switch.clone();
                let screen_duration_spin = screen_duration_spin.clone();
                let night_start_entry = night_start_entry.clone();
                let night_end_entry = night_end_entry.clone();
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(settings) => {
                                brightness_scale.set_value(settings.brightness().value() as f64 / 10.0);
                                night_brightness_scale.set_value(settings.night_brightness().value() as f64 / 10.0);
                                night_mode_switch.set_active(settings.night_mode_enabled());
                                screen_duration_spin.set_value(settings.screen_light_duration().seconds() as f64);
                                night_start_entry.set_time(settings.night_start().hour(), settings.night_start().minute());
                                night_end_entry.set_time(settings.night_end().hour(), settings.night_end().minute());
                                status_label.set_label(&fl!("status-display-settings-loaded"));
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

        // Write to Device (read-modify-write: only display fields are modified)
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let brightness_scale = brightness_scale.clone();
            let night_brightness_scale = night_brightness_scale.clone();
            let night_mode_switch = night_mode_switch.clone();
            let screen_duration_spin = screen_duration_spin.clone();
            let night_start_entry = night_start_entry.clone();
            let night_end_entry = night_end_entry.clone();
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

                let brightness = match Brightness::new((brightness_scale.value() as u8) * 10) {
                    Ok(b) => b,
                    Err(e) => {
                        status_label.set_label(&fl!("status-invalid-brightness", error = e.to_string()));
                        return;
                    }
                };
                let night_brightness = match Brightness::new((night_brightness_scale.value() as u8) * 10) {
                    Ok(b) => b,
                    Err(e) => {
                        status_label.set_label(&fl!("status-invalid-night-brightness", error = e.to_string()));
                        return;
                    }
                };
                let night_mode_enabled = night_mode_switch.is_active();
                let night_start = match night_start_entry.get_time() {
                    Some((h, m)) => match ClockTime::new(h, m) {
                        Ok(t) => t,
                        Err(e) => {
                            status_label.set_label(&fl!("status-invalid-night-start", error = e.to_string()));
                            return;
                        }
                    },
                    None => {
                        status_label.set_label(&fl!("status-invalid-night-start-time"));
                        return;
                    }
                };
                let night_end = match night_end_entry.get_time() {
                    Some((h, m)) => match ClockTime::new(h, m) {
                        Ok(t) => t,
                        Err(e) => {
                            status_label.set_label(&fl!("status-invalid-night-end", error = e.to_string()));
                            return;
                        }
                    },
                    None => {
                        status_label.set_label(&fl!("status-invalid-night-end-time"));
                        return;
                    }
                };
                let screen_light_duration = match ScreenLightDuration::new(screen_duration_spin.value() as u8) {
                    Ok(d) => d,
                    Err(e) => {
                        status_label.set_label(&fl!("status-invalid-screen-duration", error = e.to_string()));
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
                            current.time_format(),
                            current.temperature_unit(),
                            current.language(),
                            current.timezone(),
                            screen_light_duration,
                            brightness,
                            night_brightness,
                            night_start,
                            night_end,
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
                            Ok(()) => status_label.set_label(&fl!("status-display-settings-written")),
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
            brightness_scale: brightness_scale.clone(),
            night_brightness_scale: night_brightness_scale.clone(),
            night_mode_switch: night_mode_switch.clone(),
            screen_duration_spin: screen_duration_spin.clone(),
            night_start_entry: night_start_entry.clone(),
            night_end_entry: night_end_entry.clone(),
            manager: manager.clone(),
            runtime: runtime.clone(),
            connected_address: connected_address.clone(),
            config_store: config_store.clone(),
        }
    }

    /// Read display settings from the connected device and update the UI.
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
        let brightness_scale = self.brightness_scale.clone();
        let night_brightness_scale = self.night_brightness_scale.clone();
        let night_mode_switch = self.night_mode_switch.clone();
        let screen_duration_spin = self.screen_duration_spin.clone();
        let night_start_entry = self.night_start_entry.clone();
        let night_end_entry = self.night_end_entry.clone();
        let status_label = self.status_label.clone();
        glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
            Ok(result) => {
                match result {
                    Ok(settings) => {
                        brightness_scale.set_value(settings.brightness().value() as f64 / 10.0);
                        night_brightness_scale.set_value(settings.night_brightness().value() as f64 / 10.0);
                        night_mode_switch.set_active(settings.night_mode_enabled());
                        screen_duration_spin.set_value(settings.screen_light_duration().seconds() as f64);
                        night_start_entry.set_time(settings.night_start().hour(), settings.night_start().minute());
                        night_end_entry.set_time(settings.night_end().hour(), settings.night_end().minute());
                        status_label.set_label(&fl!("status-display-settings-loaded"));
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
