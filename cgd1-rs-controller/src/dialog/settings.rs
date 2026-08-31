use std::sync::Arc;

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
use cgd1_rs::Volume;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Scale;
use gtk4::SpinButton;
use gtk4::ToggleButton;
use gtk4::Window;
use gtk4::glib;
use gtk4::prelude::*;

/// Settings dialog for device settings management.
#[allow(dead_code)]
pub struct SettingsDialog {
    window: Window,
}

impl SettingsDialog {
    /// Create and show the settings dialog.
    pub fn new(
        parent: &Window,
        manager: Arc<ClockManager>,
        runtime: Arc<tokio::runtime::Runtime>,
        connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
    ) -> Self {
        let window = gtk4::Window::builder()
            .title("Settings — Alarm Clock CGD1")
            .transient_for(parent)
            .modal(true)
            .default_width(400)
            .default_height(500)
            .build();

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        let title = Label::builder().label("Device Settings").css_classes(["title-2"]).halign(Align::Start).build();
        main_box.append(&title);

        let info_label = Label::builder()
            .label("Connect to a device to read and modify settings.")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        main_box.append(&info_label);

        let volume_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let volume_label = Label::builder().label("Volume:").build();
        let volume_scale = Scale::with_range(Orientation::Horizontal, 1.0, 5.0, 1.0);
        volume_scale.set_value(3.0);
        volume_scale.set_digits(0);
        volume_scale.set_hexpand(true);
        volume_box.append(&volume_label);
        volume_box.append(&volume_scale);
        main_box.append(&volume_box);

        let brightness_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let brightness_label = Label::builder().label("Brightness:").build();
        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 10.0);
        brightness_scale.set_value(80.0);
        brightness_scale.set_digits(0);
        brightness_scale.set_hexpand(true);
        brightness_box.append(&brightness_label);
        brightness_box.append(&brightness_scale);
        main_box.append(&brightness_box);

        let night_brightness_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let night_brightness_label = Label::builder().label("Night Brightness:").build();
        let night_brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 10.0);
        night_brightness_scale.set_value(30.0);
        night_brightness_scale.set_digits(0);
        night_brightness_scale.set_hexpand(true);
        night_brightness_box.append(&night_brightness_label);
        night_brightness_box.append(&night_brightness_scale);
        main_box.append(&night_brightness_box);

        let time_format_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let time_format_label = Label::builder().label("Time Format:").build();
        let time_format_24h = ToggleButton::builder().label("24h").build();
        let time_format_12h = ToggleButton::builder().label("12h").build();
        time_format_24h.set_active(true);
        time_format_24h.set_group(Some(&time_format_12h));
        time_format_box.append(&time_format_label);
        time_format_box.append(&time_format_24h);
        time_format_box.append(&time_format_12h);
        main_box.append(&time_format_box);

        let temp_unit_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let temp_unit_label = Label::builder().label("Temperature Unit:").build();
        let temp_c = ToggleButton::builder().label("°C").build();
        let temp_f = ToggleButton::builder().label("°F").build();
        temp_c.set_active(true);
        temp_c.set_group(Some(&temp_f));
        temp_unit_box.append(&temp_unit_label);
        temp_unit_box.append(&temp_c);
        temp_unit_box.append(&temp_f);
        main_box.append(&temp_unit_box);

        let language_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let language_label = Label::builder().label("Language:").build();
        let lang_en = ToggleButton::builder().label("English").build();
        let lang_zh = ToggleButton::builder().label("中文").build();
        lang_en.set_active(true);
        lang_en.set_group(Some(&lang_zh));
        language_box.append(&language_label);
        language_box.append(&lang_en);
        language_box.append(&lang_zh);
        main_box.append(&language_box);

        let night_mode_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let night_mode_label = Label::builder().label("Night Mode:").build();
        let night_mode_toggle = ToggleButton::builder().label("Enabled").build();
        night_mode_toggle.set_active(true);
        night_mode_box.append(&night_mode_label);
        night_mode_box.append(&night_mode_toggle);
        main_box.append(&night_mode_box);

        let screen_duration_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let screen_duration_label = Label::builder().label("Screen Light Duration (s):").build();
        let screen_duration_spin = SpinButton::with_range(0.0, 255.0, 1.0);
        screen_duration_spin.set_value(10.0);
        screen_duration_box.append(&screen_duration_label);
        screen_duration_box.append(&screen_duration_spin);
        main_box.append(&screen_duration_box);

        let timezone_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let timezone_label = Label::builder().label("Timezone (hours):").build();
        let timezone_spin = SpinButton::with_range(-12.0, 14.0, 1.0);
        timezone_spin.set_value(0.0);
        timezone_box.append(&timezone_label);
        timezone_box.append(&timezone_spin);
        main_box.append(&timezone_box);

        let status_label = Label::builder().label("").css_classes(["dim-label"]).halign(Align::Start).build();
        main_box.append(&status_label);

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();
        let refresh_button = Button::builder().label("Read from Device").build();
        let apply_button = Button::builder().label("Write to Device").css_classes(["suggested-action"]).build();
        let close_button = Button::builder().label("Close").build();
        button_box.append(&refresh_button);
        button_box.append(&apply_button);
        button_box.append(&close_button);
        main_box.append(&button_box);

        let win = window.clone();
        close_button.connect_clicked(move |_| {
            win.close();
        });

        // Read from Device
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let volume_scale = volume_scale.clone();
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
            let timezone_spin = timezone_spin.clone();
            let status_label = status_label.clone();

            refresh_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().expect("mutex poisoned");
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
                let volume_scale = volume_scale.clone();
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
                let timezone_spin = timezone_spin.clone();
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(settings) => {
                                volume_scale.set_value(settings.volume().value() as f64);
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
                                timezone_spin.set_value(settings.timezone().minutes() as f64 / 60.0);
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
            let volume_scale = volume_scale.clone();
            let brightness_scale = brightness_scale.clone();
            let night_brightness_scale = night_brightness_scale.clone();
            let time_format_12h = time_format_12h.clone();
            let temp_f = temp_f.clone();
            let lang_en = lang_en.clone();
            let night_mode_toggle = night_mode_toggle.clone();
            let screen_duration_spin = screen_duration_spin.clone();
            let timezone_spin = timezone_spin.clone();
            let status_label = status_label.clone();

            apply_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().expect("mutex poisoned");
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };

                let volume = match Volume::new(volume_scale.value() as u8) {
                    Ok(v) => v,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid volume: {e}"));
                        return;
                    }
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
                let timezone_minutes = (timezone_spin.value() * 60.0) as i16;
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
                            volume,
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

        window.set_child(Some(&main_box));
        window.present();

        Self { window }
    }
}
