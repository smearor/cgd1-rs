use cgd1_rs::AlarmSlot;
use cgd1_rs::DayMask;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveTime;
use chrono::TimeZone;

/// Computed next alarm time with a human-readable label.
pub struct NextAlarm {
    /// The date-time when the alarm will fire.
    pub when: DateTime<Local>,
    /// Short label for display, e.g. "07:30 Mon".
    pub label: String,
}

/// Compute the next upcoming alarm from a list of alarm slots.
///
/// Only enabled alarms are considered. Returns `None` if no enabled alarm exists.
pub fn next_alarm(slots: &[AlarmSlot]) -> Option<NextAlarm> {
    let now = Local::now();
    let mut best: Option<NextAlarm> = None;

    for slot in slots.iter().filter(|s| s.entry.enabled()) {
        let hour = slot.entry.hour();
        let minute = slot.entry.minute();
        let mask = slot.entry.repeat_mask();

        let alarm_time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0)?;
        let when = compute_next_fire(now, alarm_time, mask)?;
        let candidate = NextAlarm {
            when,
            label: format_label(when, slot),
        };

        if best.as_ref().map_or(true, |b| when < b.when) {
            best = Some(candidate);
        }
    }

    best
}

/// Compute the next date-time this alarm will fire, given the current time and repeat mask.
fn compute_next_fire(now: DateTime<Local>, alarm_time: NaiveTime, mask: DayMask) -> Option<DateTime<Local>> {
    let mask_val = mask.value();

    // One-shot alarm: fires today if time hasn't passed, otherwise tomorrow.
    if mask_val == DayMask::ONCE.value() {
        let today_alarm = now.date_naive().and_time(alarm_time);
        let today_local = Local.from_local_datetime(&today_alarm).single()?;
        if today_local > now {
            return Some(today_local);
        }
        let tomorrow = today_local + chrono::Duration::days(1);
        return Some(tomorrow);
    }

    // Repeating alarm: find the next matching weekday.
    // Bit 0 = Monday, ..., Bit 6 = Sunday.
    // chrono::Weekday: Mon=0, Tue=1, ..., Sun=6 — matches our bit layout.
    for offset in 0..7 {
        let candidate_day = now + chrono::Duration::days(offset);
        let weekday_bit = 1u8 << weekday_num(candidate_day.weekday());
        if mask_val & weekday_bit == 0 {
            continue;
        }
        let candidate_naive = candidate_day.date_naive().and_time(alarm_time);
        let candidate = Local.from_local_datetime(&candidate_naive).single()?;
        if candidate > now {
            return Some(candidate);
        }
    }

    // All matching days today have passed; look at next week.
    for offset in 7..14 {
        let candidate_day = now + chrono::Duration::days(offset);
        let weekday_bit = 1u8 << weekday_num(candidate_day.weekday());
        if mask_val & weekday_bit == 0 {
            continue;
        }
        let candidate_naive = candidate_day.date_naive().and_time(alarm_time);
        let candidate = Local.from_local_datetime(&candidate_naive).single()?;
        return Some(candidate);
    }

    None
}

/// Format a short label for the next alarm, e.g. "07:30 Mon".
fn format_label(when: DateTime<Local>, slot: &AlarmSlot) -> String {
    let mask_val = slot.entry.repeat_mask().value();
    let time_str = format!("{:02}:{:02}", slot.entry.hour(), slot.entry.minute());

    if mask_val == DayMask::ONCE.value() {
        let day_name = weekday_short(when.weekday());
        format!("{time_str} {day_name}")
    } else if mask_val == DayMask::EVERY_DAY.value() {
        time_str
    } else {
        let day_name = weekday_short(when.weekday());
        format!("{time_str} {day_name}")
    }
}

/// Short weekday name for display.
fn weekday_short(wd: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;
    match wd {
        Mon => "Mon",
        Tue => "Tue",
        Wed => "Wed",
        Thu => "Thu",
        Fri => "Fri",
        Sat => "Sat",
        Sun => "Sun",
    }
}

/// Convert a `Weekday` to a 0-based number (Mon=0 ... Sun=6), matching `DayMask` bit layout.
fn weekday_num(wd: chrono::Weekday) -> u8 {
    use chrono::Weekday::*;
    match wd {
        Mon => 0,
        Tue => 1,
        Wed => 2,
        Thu => 3,
        Fri => 4,
        Sat => 5,
        Sun => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgd1_rs::AlarmEntry;
    use cgd1_rs::AlarmSlotIndex;
    use cgd1_rs::ClockTime;

    fn make_slot(slot: u8, hour: u8, minute: u8, mask: DayMask, enabled: bool) -> AlarmSlot {
        AlarmSlot {
            index: AlarmSlotIndex::new(slot).unwrap(),
            entry: AlarmEntry::new(ClockTime::new(hour, minute).unwrap(), mask, enabled, true),
        }
    }

    #[test]
    fn no_alarms_returns_none() {
        assert!(next_alarm(&[]).is_none());
    }

    #[test]
    fn disabled_alarm_returns_none() {
        let slots = vec![make_slot(0, 7, 30, DayMask::EVERY_DAY, false)];
        assert!(next_alarm(&slots).is_none());
    }

    #[test]
    fn daily_alarm_returns_some() {
        let slots = vec![make_slot(0, 7, 30, DayMask::EVERY_DAY, true)];
        let result = next_alarm(&slots);
        assert!(result.is_some());
        assert!(result.unwrap().label.contains("07:30"));
    }

    #[test]
    fn picks_earliest_alarm() {
        let slots = vec![make_slot(0, 9, 0, DayMask::EVERY_DAY, true), make_slot(1, 6, 30, DayMask::EVERY_DAY, true)];
        let result = next_alarm(&slots).unwrap();
        assert!(result.label.contains("06:30"));
    }
}
