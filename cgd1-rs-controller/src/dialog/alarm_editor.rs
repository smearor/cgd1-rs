use crate::dialog::TimeEntry;
use cgd1_rs::AlarmEntry;
use cgd1_rs::AlarmSlot;
use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::MacAddress;
use glib::ControlFlow;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::TryRecvError;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Separator;
use gtk4::Switch;
use gtk4::ToggleButton;
use gtk4::glib;
use gtk4::prelude::*;
use tokio::runtime::Runtime;
use tracing::error;
use tracing::warn;

/// Reusable alarm editor widget - can be embedded in the main window or a dialog.
#[allow(dead_code)]
pub struct AlarmEditorWidget {
    /// The container box holding all alarm editor content.
    pub container: Box,
    /// Status label for operation feedback.
    pub status_label: Label,
    /// Refresh button.
    pub refresh_button: Button,
    /// Row widgets for reading alarm state back from device.
    row_widgets: Vec<(TimeEntry, Switch, ToggleButton, ToggleButton, Vec<ToggleButton>)>,
    /// Clock manager for device communication.
    manager: Arc<ClockManager>,
    /// Async runtime.
    runtime: Arc<Runtime>,
    /// Connected device address.
    connected_address: Arc<Mutex<Option<MacAddress>>>,
}

/// Widgets for a single alarm row.
#[derive(Clone)]
struct AlarmRowWidgets {
    time_entry: TimeEntry,
    enabled_switch: Switch,
    snooze_toggle: ToggleButton,
    once_toggle: ToggleButton,
    day_toggles: Vec<ToggleButton>,
    set_button: Button,
    delete_button: Button,
}

impl AlarmEditorWidget {
    /// Build the alarm editor content (without window chrome).
    pub fn new(manager: Arc<ClockManager>, runtime: Arc<Runtime>, connected_address: Arc<Mutex<Option<MacAddress>>>, on_changed: impl Fn() + 'static) -> Self {
        let on_changed = Arc::new(on_changed);
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .max_content_height(300)
            .propagate_natural_height(true)
            .build();

        let alarm_list_box = Box::builder().orientation(Orientation::Vertical).spacing(4).build();

        let mut rows: Vec<(u8, AlarmRowWidgets)> = Vec::new();
        for slot in 0..16u8 {
            let (row, widgets) = create_alarm_row();
            alarm_list_box.append(&row);
            rows.push((slot, widgets));
        }

        scrolled.set_child(Some(&alarm_list_box));
        container.append(&scrolled);

        container.append(&Separator::new(Orientation::Horizontal));

        let status_label = Label::builder().label("").css_classes(["dim-label"]).halign(Align::Start).hexpand(true).build();

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::Fill).build();
        let refresh_button = Button::builder().label("Read from Device").build();
        button_box.append(&status_label);
        button_box.append(&refresh_button);
        container.append(&button_box);

        // Read from Device - connect button after widget construction via stored fields
        // (handled below after struct is built)

        // Set and Delete per row
        for (slot, widgets) in rows.iter().map(|(s, w)| (*s, w.clone())) {
            let Ok(slot_idx) = AlarmSlotIndex::new(slot) else {
                error!(slot, "invalid alarm slot index, skipping row");
                continue;
            };
            let manager_set = manager.clone();
            let runtime_set = runtime.clone();
            let connected_address_set = connected_address.clone();
            let status_label_set = status_label.clone();
            let on_changed_set = on_changed.clone();
            let time_entry = widgets.time_entry.clone();
            let enabled_switch = widgets.enabled_switch.clone();
            let snooze_toggle = widgets.snooze_toggle.clone();
            let once_toggle = widgets.once_toggle.clone();
            let day_toggles = widgets.day_toggles.clone();

            widgets.set_button.connect_clicked(move |_| {
                let addr = *connected_address_set.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label_set.set_label("No device connected");
                    return;
                };
                let Some((hour, minute)) = time_entry.get_time() else {
                    status_label_set.set_label("Invalid time");
                    return;
                };
                let enabled = enabled_switch.is_active();
                let snooze = snooze_toggle.is_active();
                let repeat_mask = if once_toggle.is_active() {
                    DayMask::ONCE
                } else {
                    let mut mask = 0u8;
                    for (j, btn) in day_toggles.iter().enumerate() {
                        if btn.is_active() {
                            mask |= 1 << j;
                        }
                    }
                    DayMask::new(mask)
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
                let on_changed = on_changed_set.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => {
                                status_label_set.set_label(&format!("Alarm #{slot:02} set"));
                                (*on_changed)();
                            }
                            Err(e) => status_label_set.set_label(&format!("Set failed: {e}")),
                        }
                        ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        status_label_set.set_label("Set task failed");
                        ControlFlow::Break
                    }
                });
            });

            let manager_del = manager.clone();
            let runtime_del = runtime.clone();
            let connected_address_del = connected_address.clone();
            let status_label_del = status_label.clone();
            let on_changed_del = on_changed.clone();

            widgets.delete_button.connect_clicked(move |_| {
                let addr = *connected_address_del.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
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
                let on_changed = on_changed_del.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => {
                                status_label_del.set_label(&format!("Alarm #{slot:02} deleted"));
                                (*on_changed)();
                            }
                            Err(e) => status_label_del.set_label(&format!("Delete failed: {e}")),
                        }
                        ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        status_label_del.set_label("Delete task failed");
                        ControlFlow::Break
                    }
                });
            });
        }

        let row_widgets: Vec<(TimeEntry, Switch, ToggleButton, ToggleButton, Vec<ToggleButton>)> = rows
            .iter()
            .map(|(_, w)| {
                (
                    w.time_entry.clone(),
                    w.enabled_switch.clone(),
                    w.snooze_toggle.clone(),
                    w.once_toggle.clone(),
                    w.day_toggles.clone(),
                )
            })
            .collect();

        let widget = Self {
            container,
            status_label: status_label.clone(),
            refresh_button: refresh_button.clone(),
            row_widgets,
            manager: manager.clone(),
            runtime: runtime.clone(),
            connected_address: connected_address.clone(),
        };

        // Connect refresh button to read_alarms
        {
            let widget_manager = manager.clone();
            let widget_runtime = runtime.clone();
            let widget_addr = connected_address.clone();
            let widget_status = status_label.clone();
            let widget_rows = rows
                .iter()
                .map(|(_, w)| {
                    (
                        w.time_entry.clone(),
                        w.enabled_switch.clone(),
                        w.snooze_toggle.clone(),
                        w.once_toggle.clone(),
                        w.day_toggles.clone(),
                    )
                })
                .collect::<Vec<_>>();
            refresh_button.connect_clicked(move |_| {
                let addr = *widget_addr.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    widget_status.set_label("No device connected");
                    return;
                };
                widget_status.set_label("Reading alarms...");
                let manager = widget_manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<cgd1_rs::AlarmSlot>, String>>();
                widget_runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.read_alarms().await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let row_widgets = widget_rows.clone();
                let status_label = widget_status.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(slots) => {
                                for (i, (time_entry, enabled, snooze, once, days)) in row_widgets.iter().enumerate() {
                                    let slot_alarm = slots.iter().find(|s| s.index.value() as usize == i);
                                    match slot_alarm {
                                        Some(s) => {
                                            time_entry.set_time(s.entry.hour(), s.entry.minute());
                                            enabled.set_active(s.entry.enabled());
                                            snooze.set_active(s.entry.snooze());
                                            let mask = s.entry.repeat_mask().value();
                                            once.set_active(mask == DayMask::ONCE.value());
                                            for (j, btn) in days.iter().enumerate() {
                                                btn.set_active(mask & (1 << j) != 0);
                                            }
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
                        ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        status_label.set_label("Read task failed");
                        ControlFlow::Break
                    }
                });
            });
        }

        widget
    }

    /// Read alarms from the connected device and update the UI.
    pub fn read_alarms(&self) {
        let addr = *self.connected_address.lock().unwrap_or_else(|p| {
            warn!("mutex poisoned - recovering");
            p.into_inner()
        });
        let Some(addr) = addr else {
            self.status_label.set_label("No device connected");
            return;
        };
        self.status_label.set_label("Reading alarms...");
        let manager = self.manager.clone();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<AlarmSlot>, String>>();
        self.runtime.spawn(async move {
            let result = async {
                let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                device.read_alarms().await
            }
            .await;
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
        let rx = std::cell::RefCell::new(rx);
        let row_widgets = self.row_widgets.clone();
        let status_label = self.status_label.clone();
        glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
            Ok(result) => {
                match result {
                    Ok(slots) => {
                        for (i, (time_entry, enabled, snooze, once, days)) in row_widgets.iter().enumerate() {
                            let slot_alarm = slots.iter().find(|s| s.index.value() as usize == i);
                            match slot_alarm {
                                Some(s) => {
                                    time_entry.set_time(s.entry.hour(), s.entry.minute());
                                    enabled.set_active(s.entry.enabled());
                                    snooze.set_active(s.entry.snooze());
                                    let mask = s.entry.repeat_mask().value();
                                    once.set_active(mask == DayMask::ONCE.value());
                                    for (j, btn) in days.iter().enumerate() {
                                        btn.set_active(mask & (1 << j) != 0);
                                    }
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
                ControlFlow::Break
            }
            Err(TryRecvError::Empty) => ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status_label.set_label("Read task failed");
                ControlFlow::Break
            }
        });
    }
}

/// Create a single alarm slot row with time, repeat, enabled, and snooze controls.
fn create_alarm_row() -> (Box, AlarmRowWidgets) {
    let row = Box::builder().orientation(Orientation::Horizontal).spacing(4).build();

    let enabled_switch = Switch::builder().tooltip_text("Enable alarm").build();
    enabled_switch.set_active(false);
    row.append(&enabled_switch);

    let time_entry = TimeEntry::new();
    row.append(time_entry.widget());

    let snooze_toggle = ToggleButton::builder().icon_name("nf-iec-sleep-mode-symbolic").tooltip_text("Snooze").build();
    snooze_toggle.set_active(true);
    row.append(&snooze_toggle);

    let once_toggle = ToggleButton::builder().icon_name("nf-cod-calendar-symbolic").tooltip_text("Once").build();
    once_toggle.set_active(true);
    row.append(&once_toggle);

    let day_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    let day_toggles: Vec<ToggleButton> = day_labels.iter().map(|label| ToggleButton::builder().label(*label).build()).collect();

    // Mutual exclusivity: Once vs day toggles
    let once_for_days = once_toggle.clone();
    let days_for_once = day_toggles.clone();
    for btn in &day_toggles {
        let once_clone = once_for_days.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() && once_clone.is_active() {
                once_clone.set_active(false);
            }
        });
    }
    once_toggle.connect_toggled(move |on| {
        if on.is_active() {
            for d in &days_for_once {
                d.set_active(false);
            }
        }
    });

    for btn in &day_toggles {
        row.append(btn);
    }

    let spacer = Box::builder().hexpand(true).build();
    row.append(&spacer);

    let set_button = Button::builder()
        .icon_name("nf-cod-check-symbolic")
        .tooltip_text("Set")
        .css_classes(["suggested-action"])
        .build();
    row.append(&set_button);

    let delete_button = Button::builder()
        .icon_name("nf-cod-trash-symbolic")
        .tooltip_text("Del")
        .css_classes(["destructive-action"])
        .build();
    row.append(&delete_button);

    let widgets = AlarmRowWidgets {
        time_entry,
        enabled_switch,
        snooze_toggle,
        once_toggle,
        day_toggles,
        set_button,
        delete_button,
    };

    (row, widgets)
}
