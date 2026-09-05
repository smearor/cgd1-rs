use gtk4::Box;
use gtk4::Frame;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::prelude::*;

/// Create a labeled settings row with a fixed-width label.
pub fn settings_row(label_text: &str) -> Box {
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
pub fn settings_frame(icon_name: &str, title: &str) -> Frame {
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

/// Return a list of (display_label, offset_minutes) for common timezones.
///
/// The device stores timezone in 6-minute units, so fractional offsets
/// like +5:30 (India) or +5:45 (Nepal) are supported.
pub fn timezone_list() -> Vec<(&'static str, i16)> {
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
pub fn find_timezone_index(offset_minutes: i16) -> Option<u32> {
    timezone_list().iter().position(|(_, mins)| *mins == offset_minutes).map(|idx| idx as u32)
}
