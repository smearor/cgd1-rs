use std::sync::Arc;

use cgd1_rs::AlarmEntry;
use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::MacAddress;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::ScrolledWindow;
use gtk4::Separator;
use gtk4::SpinButton;
use gtk4::StringList;
use gtk4::ToggleButton;
use gtk4::Window;
use gtk4::glib;
use gtk4::prelude::*;

/// Alarms dialog for alarm management.
#[allow(dead_code)]
pub struct AlarmsDialog {
    window: Window,
}

/// Widgets for a single alarm row.
#[derive(Clone)]
struct AlarmRowWidgets {
    hour_spin: SpinButton,
    minute_spin: SpinButton,
    enabled_toggle: ToggleButton,
    snooze_toggle: ToggleButton,
    repeat_dropdown: gtk4::DropDown,
    set_button: Button,
    delete_button: Button,
}

impl AlarmsDialog {
    /// Create and show the alarms dialog.
    pub fn new(
        parent: &Window,
        manager: Arc<ClockManager>,
        runtime: Arc<tokio::runtime::Runtime>,
        connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
    ) -> Self {
        let window = gtk4::Window::builder()
            .title("Alarms — Alarm Clock CGD1")
            .transient_for(parent)
            .modal(true)
            .default_width(420)
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

        let title = Label::builder().label("Alarm Management").css_classes(["title-2"]).halign(Align::Start).build();
        main_box.append(&title);

        let info_label = Label::builder()
            .label("Connect to a device to manage alarms (up to 16 slots).")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        main_box.append(&info_label);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let alarm_list_box = Box::builder().orientation(Orientation::Vertical).spacing(4).build();

        let mut rows: Vec<(u8, AlarmRowWidgets)> = Vec::new();
        for slot in 0..16u8 {
            let (row, widgets) = create_alarm_row(slot);
            alarm_list_box.append(&row);
            rows.push((slot, widgets));
        }

        scrolled.set_child(Some(&alarm_list_box));
        main_box.append(&scrolled);

        main_box.append(&Separator::new(Orientation::Horizontal));

        let status_label = Label::builder().label("").css_classes(["dim-label"]).halign(Align::Start).build();

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();
        let refresh_button = Button::builder().label("Read from Device").build();
        let close_button = Button::builder().label("Close").build();
        button_box.append(&refresh_button);
        button_box.append(&close_button);
        main_box.append(&status_label);
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
            let status_label = status_label.clone();
            let row_widgets: Vec<(SpinButton, SpinButton, ToggleButton, ToggleButton, gtk4::DropDown)> = rows
                .iter()
                .map(|(_, w)| {
                    (
                        w.hour_spin.clone(),
                        w.minute_spin.clone(),
                        w.enabled_toggle.clone(),
                        w.snooze_toggle.clone(),
                        w.repeat_dropdown.clone(),
                    )
                })
                .collect();

            refresh_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().expect("mutex poisoned");
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };
                status_label.set_label("Reading alarms...");
                let manager = manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<cgd1_rs::AlarmSlot>, String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.read_alarms().await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let row_widgets = row_widgets.clone();
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(slots) => {
                                for (i, (hour, minute, enabled, snooze, repeat)) in row_widgets.iter().enumerate() {
                                    let slot_alarm = slots.iter().find(|s| s.index.value() as usize == i);
                                    match slot_alarm {
                                        Some(s) => {
                                            hour.set_value(s.entry.hour() as f64);
                                            minute.set_value(s.entry.minute() as f64);
                                            enabled.set_active(s.entry.enabled());
                                            snooze.set_active(s.entry.snooze());
                                            let mask = s.entry.repeat_mask().value();
                                            let idx = if mask == DayMask::EVERY_DAY.value() {
                                                1
                                            } else if mask == DayMask::WEEKDAYS.value() {
                                                2
                                            } else if mask == DayMask::WEEKENDS.value() {
                                                3
                                            } else {
                                                0
                                            };
                                            repeat.set_selected(idx as u32);
                                        }
                                        None => {
                                            enabled.set_active(false);
                                        }
                                    }
                                }
                                status_label.set_label(&format!("Loaded {} alarm(s)", slots.len()));
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

        // Set and Delete per row
        for (slot, widgets) in rows.iter().map(|(s, w)| (*s, w.clone())) {
            let slot_idx = AlarmSlotIndex::new(slot).expect("slot index 0-15 is valid");
            let manager_set = manager.clone();
            let runtime_set = runtime.clone();
            let connected_address_set = connected_address.clone();
            let status_label_set = status_label.clone();
            let hour_spin = widgets.hour_spin.clone();
            let minute_spin = widgets.minute_spin.clone();
            let enabled_toggle = widgets.enabled_toggle.clone();
            let snooze_toggle = widgets.snooze_toggle.clone();
            let repeat_dropdown = widgets.repeat_dropdown.clone();

            widgets.set_button.connect_clicked(move |_| {
                let addr = *connected_address_set.lock().expect("mutex poisoned");
                let Some(addr) = addr else {
                    status_label_set.set_label("No device connected");
                    return;
                };
                let hour = hour_spin.value() as u8;
                let minute = minute_spin.value() as u8;
                let enabled = enabled_toggle.is_active();
                let snooze = snooze_toggle.is_active();
                let repeat_mask = match repeat_dropdown.selected() {
                    1 => DayMask::EVERY_DAY,
                    2 => DayMask::WEEKDAYS,
                    3 => DayMask::WEEKENDS,
                    _ => DayMask::ONCE,
                };
                let time = match ClockTime::new(hour, minute) {
                    Ok(t) => t,
                    Err(e) => {
                        status_label_set.set_label(&format!("Invalid time: {e}"));
                        return;
                    }
                };
                let entry = AlarmEntry::new(time, repeat_mask, enabled, snooze);
                status_label_set.set_label(&format!("Setting alarm #{slot:02}..."));
                let manager_set = manager_set.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime_set.spawn(async move {
                    let result = async {
                        let device = manager_set.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.set_alarm(&entry, slot_idx).await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label_set = status_label_set.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => status_label_set.set_label(&format!("Alarm #{slot:02} set")),
                            Err(e) => status_label_set.set_label(&format!("Set failed: {e}")),
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label_set.set_label("Set task failed");
                        glib::ControlFlow::Break
                    }
                });
            });

            let manager_del = manager.clone();
            let runtime_del = runtime.clone();
            let connected_address_del = connected_address.clone();
            let status_label_del = status_label.clone();

            widgets.delete_button.connect_clicked(move |_| {
                let addr = *connected_address_del.lock().expect("mutex poisoned");
                let Some(addr) = addr else {
                    status_label_del.set_label("No device connected");
                    return;
                };
                status_label_del.set_label(&format!("Deleting alarm #{slot:02}..."));
                let manager_del = manager_del.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime_del.spawn(async move {
                    let result = async {
                        let device = manager_del.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.delete_alarm(slot_idx).await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label_del = status_label_del.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => status_label_del.set_label(&format!("Alarm #{slot:02} deleted")),
                            Err(e) => status_label_del.set_label(&format!("Delete failed: {e}")),
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        status_label_del.set_label("Delete task failed");
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

/// Create a single alarm slot row with time, repeat, enabled, and snooze controls.
fn create_alarm_row(slot: u8) -> (Box, AlarmRowWidgets) {
    let row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();

    let slot_label = Label::builder().label(format!("#{slot:02}")).css_classes(["dim-label"]).width_chars(4).build();
    row.append(&slot_label);

    let hour_spin = SpinButton::with_range(0.0, 23.0, 1.0);
    hour_spin.set_value(7.0);
    row.append(&hour_spin);

    let colon_label = Label::builder().label(":").build();
    row.append(&colon_label);

    let minute_spin = SpinButton::with_range(0.0, 59.0, 1.0);
    minute_spin.set_value(30.0);
    row.append(&minute_spin);

    let enabled_toggle = ToggleButton::builder().label("On").build();
    enabled_toggle.set_active(false);
    row.append(&enabled_toggle);

    let snooze_toggle = ToggleButton::builder().label("Snooze").build();
    snooze_toggle.set_active(true);
    row.append(&snooze_toggle);

    let repeat_label = Label::builder().label("Repeat:").css_classes(["dim-label"]).build();
    row.append(&repeat_label);

    let repeat_model = StringList::new(&["Once", "Every day", "Weekdays", "Weekends"]);
    let repeat_dropdown = gtk4::DropDown::new(Some(repeat_model), None::<&gtk4::Expression>);
    row.append(&repeat_dropdown);

    let spacer = Box::builder().hexpand(true).build();
    row.append(&spacer);

    let set_button = Button::builder().label("Set").css_classes(["suggested-action"]).build();
    row.append(&set_button);

    let delete_button = Button::builder().label("Del").css_classes(["destructive-action"]).build();
    row.append(&delete_button);

    let widgets = AlarmRowWidgets {
        hour_spin,
        minute_spin,
        enabled_toggle,
        snooze_toggle,
        repeat_dropdown,
        set_button,
        delete_button,
    };

    (row, widgets)
}
