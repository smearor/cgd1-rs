use std::cell::Cell;
use std::rc::Rc;

use gtk4::Entry;
use gtk4::EventControllerKey;
use gtk4::GestureClick;
use gtk4::glib;
use gtk4::prelude::*;

/// A time entry widget with `HH:MM` input masking.
///
/// - Typing digits overwrites the digit at the cursor position.
/// - After typing two hour digits the cursor auto-advances to minutes.
/// - Clicking anywhere in the field resets the cursor to the start of hours.
/// - First hour digit is restricted to 0–2, first minute digit to 0–5.
/// - Arrow keys increment/decrement the field under the cursor.
pub struct TimeEntry {
    entry: Entry,
    /// Guard to suppress the changed handler during programmatic text changes.
    guard: Rc<Cell<bool>>,
}

impl TimeEntry {
    /// Create a new `TimeEntry` pre-filled with `07:30`.
    pub fn new() -> Self {
        let entry = Entry::builder()
            .text("07:30")
            .max_length(5)
            .width_chars(5)
            .xalign(0.5f32)
            .input_purpose(gtk4::InputPurpose::Digits)
            .css_classes(["time-entry"])
            .build();

        let guard = Rc::new(Cell::new(false));
        Self::connect_signals(&entry, &guard);

        Self { entry, guard }
    }

    /// Parse the current text into `(hour, minute)`.
    ///
    /// Returns `None` if the text is not a valid `HH:MM` time.
    pub fn get_time(&self) -> Option<(u8, u8)> {
        let text = self.entry.text();
        let parts: Vec<&str> = text.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let hour: u8 = parts[0].parse().ok()?;
        let minute: u8 = parts[1].parse().ok()?;
        if hour > 23 || minute > 59 {
            return None;
        }
        Some((hour, minute))
    }

    /// Set the time displayed in the entry.
    pub fn set_time(&self, hour: u8, minute: u8) {
        self.guard.set(true);
        self.entry.set_text(&format!("{hour:02}:{minute:02}"));
        self.guard.set(false);
    }

    /// Borrow the underlying `Entry` widget for layout.
    pub fn widget(&self) -> &Entry {
        &self.entry
    }

    /// Connect validation, overwrite, auto-advance, click-reset, and keyboard navigation.
    fn connect_signals(entry: &Entry, guard: &Rc<Cell<bool>>) {
        // --- Validation on changed ---
        let entry_for_changed = entry.clone();
        let guard_for_changed = guard.clone();
        entry.connect_changed(move |_| {
            if guard_for_changed.get() {
                return;
            }
            let text = entry_for_changed.text();
            if Self::validate(&text) {
                entry_for_changed.remove_css_class("error");
            } else {
                entry_for_changed.add_css_class("error");
            }
        });

        // --- Click: always reset cursor to start of hours ---
        let entry_for_click = entry.clone();
        let click = GestureClick::new();
        click.connect_pressed(move |_, _, _, _| {
            entry_for_click.grab_focus();
            entry_for_click.set_position(0);
        });
        entry.add_controller(click);

        // --- Focus: reset cursor to start on focus ---
        let entry_for_focus_enter = entry.clone();
        let focus_enter = gtk4::EventControllerFocus::new();
        focus_enter.connect_enter(move |_| {
            entry_for_focus_enter.set_position(0);
        });
        entry.add_controller(focus_enter);

        // --- Focus leave: normalize format ---
        let entry_for_focus = entry.clone();
        let guard_for_focus = guard.clone();
        let focus_leave = gtk4::EventControllerFocus::new();
        focus_leave.connect_leave(move |_| {
            let text = entry_for_focus.text();
            if let Some((h, m)) = Self::validate_and_parse(&text) {
                guard_for_focus.set(true);
                entry_for_focus.set_text(&format!("{h:02}:{m:02}"));
                guard_for_focus.set(false);
                entry_for_focus.remove_css_class("error");
            }
        });
        entry.add_controller(focus_leave);

        // --- Key handler: full manual control ---
        let entry_for_key = entry.clone();
        let guard_for_key = guard.clone();
        let controller = EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, _| {
            let text = entry_for_key.text();
            let cursor = entry_for_key.position();
            let chars: Vec<char> = text.chars().collect();
            let (hour, minute) = Self::validate_and_parse(&text).unwrap_or((0, 0));

            // Digit keys: full manual overwrite + auto-advance + validation
            if let Some(digit) = key.to_unicode().and_then(|c| c.to_digit(10)) {
                let d = digit as u8;
                let pos = if cursor == 2 { 3 } else { cursor as usize };

                let (new_hour, new_minute, next_pos) = if pos == 0 {
                    // First hour digit: only 0, 1, 2 allowed
                    if d > 2 {
                        return glib::Propagation::Stop;
                    }
                    let h_ones = chars.get(1).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
                    let new_h = d * 10 + h_ones;
                    (new_h, minute, 1)
                } else if pos == 1 {
                    // Second hour digit
                    let h_tens = chars.get(0).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
                    let new_h = h_tens * 10 + d;
                    if new_h > 23 {
                        return glib::Propagation::Stop;
                    }
                    (new_h, minute, 3)
                } else if pos == 3 {
                    // First minute digit: only 0–5 allowed
                    if d > 5 {
                        return glib::Propagation::Stop;
                    }
                    let m_ones = chars.get(4).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
                    let new_m = d * 10 + m_ones;
                    (hour, new_m, 4)
                } else if pos == 4 {
                    // Second minute digit
                    let m_tens = chars.get(3).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
                    let new_m = m_tens * 10 + d;
                    if new_m > 59 {
                        return glib::Propagation::Stop;
                    }
                    (hour, new_m, 5)
                } else {
                    return glib::Propagation::Stop;
                };

                guard_for_key.set(true);
                entry_for_key.set_text(&format!("{new_hour:02}:{new_minute:02}"));
                guard_for_key.set(false);
                entry_for_key.set_position(next_pos as i32);
                return glib::Propagation::Stop;
            }

            // Non-digit keys
            match key {
                gtk4::gdk::Key::Up => {
                    let (new_h, new_m) = if cursor <= 2 { ((hour + 1) % 24, minute) } else { (hour, (minute + 1) % 60) };
                    guard_for_key.set(true);
                    entry_for_key.set_text(&format!("{new_h:02}:{new_m:02}"));
                    guard_for_key.set(false);
                    entry_for_key.set_position(cursor);
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Down => {
                    let (new_h, new_m) = if cursor <= 2 {
                        (if hour == 0 { 23 } else { hour - 1 }, minute)
                    } else {
                        (hour, if minute == 0 { 59 } else { minute - 1 })
                    };
                    guard_for_key.set(true);
                    entry_for_key.set_text(&format!("{new_h:02}:{new_m:02}"));
                    guard_for_key.set(false);
                    entry_for_key.set_position(cursor);
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Tab | gtk4::gdk::Key::ISO_Left_Tab => {
                    if cursor <= 2 {
                        entry_for_key.set_position(3);
                    } else {
                        entry_for_key.set_position(0);
                    }
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Left | gtk4::gdk::Key::Right => glib::Propagation::Proceed,
                _ => glib::Propagation::Stop,
            }
        });
        entry.add_controller(controller);
    }

    /// Check whether `text` matches `HH:MM` with valid ranges.
    fn validate(text: &str) -> bool {
        Self::validate_and_parse(text).is_some()
    }

    /// Parse `text` as `HH:MM`, returning `Some((hour, minute))` if valid.
    fn validate_and_parse(text: &str) -> Option<(u8, u8)> {
        let parts: Vec<&str> = text.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let hour: u8 = parts[0].parse().ok()?;
        let minute: u8 = parts[1].parse().ok()?;
        if hour > 23 || minute > 59 {
            return None;
        }
        Some((hour, minute))
    }
}

impl Clone for TimeEntry {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            guard: self.guard.clone(),
        }
    }
}
