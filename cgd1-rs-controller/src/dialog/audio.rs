use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::TryRecvError;

use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::DeviceSettings;
use cgd1_rs::MacAddress;
use cgd1_rs::RingtoneSignature;
use cgd1_rs::Volume;

/// Bundled PCM data for each built-in ringtone, keyed by hex signature.
fn builtin_ringtone_pcm(sig: RingtoneSignature) -> Option<&'static [u8]> {
    match sig {
        RingtoneSignature::Beep => Some(include_bytes!("../../assets/ringtones/fdc366a5.pcm")),
        RingtoneSignature::Digital => Some(include_bytes!("../../assets/ringtones/0961bb77.pcm")),
        RingtoneSignature::Digital2 => Some(include_bytes!("../../assets/ringtones/ba2c2c8c.pcm")),
        RingtoneSignature::Cuckoo => Some(include_bytes!("../../assets/ringtones/ea2d4c02.pcm")),
        RingtoneSignature::Telephone => Some(include_bytes!("../../assets/ringtones/791bacb3.pcm")),
        RingtoneSignature::ExoticGuitar => Some(include_bytes!("../../assets/ringtones/1d019fd6.pcm")),
        RingtoneSignature::LivelyPiano => Some(include_bytes!("../../assets/ringtones/6e70b659.pcm")),
        RingtoneSignature::StoryPiano => Some(include_bytes!("../../assets/ringtones/8f004886.pcm")),
        RingtoneSignature::ForestPiano => Some(include_bytes!("../../assets/ringtones/26522519.pcm")),
        RingtoneSignature::MonkeyIsland => Some(include_bytes!("../../assets/ringtones/4d6f6e6b.pcm")),
        RingtoneSignature::AlarmSynth => Some(include_bytes!("../../assets/ringtones/416c5379.pcm")),
        RingtoneSignature::ArrayMbira => Some(include_bytes!("../../assets/ringtones/41724d62.pcm")),
        RingtoneSignature::Bliss => Some(include_bytes!("../../assets/ringtones/426c6973.pcm")),
        RingtoneSignature::Celestial => Some(include_bytes!("../../assets/ringtones/43656c73.pcm")),
        RingtoneSignature::Entropy => Some(include_bytes!("../../assets/ringtones/456e7472.pcm")),
        RingtoneSignature::GlassMarimba => Some(include_bytes!("../../assets/ringtones/476c4d61.pcm")),
        RingtoneSignature::HaloPentatonic => Some(include_bytes!("../../assets/ringtones/48616c6f.pcm")),
        RingtoneSignature::Harmonics => Some(include_bytes!("../../assets/ringtones/4861726d.pcm")),
        RingtoneSignature::HarpArp => Some(include_bytes!("../../assets/ringtones/48617270.pcm")),
        RingtoneSignature::KotoChords => Some(include_bytes!("../../assets/ringtones/4b6f746f.pcm")),
        RingtoneSignature::Sakenointi => Some(include_bytes!("../../assets/ringtones/53616b65.pcm")),
        RingtoneSignature::SamsSong => Some(include_bytes!("../../assets/ringtones/53616d73.pcm")),
        RingtoneSignature::Soul => Some(include_bytes!("../../assets/ringtones/536f756c.pcm")),
        RingtoneSignature::Sparkle => Some(include_bytes!("../../assets/ringtones/53706172.pcm")),
        RingtoneSignature::Supreme => Some(include_bytes!("../../assets/ringtones/53757072.pcm")),
        RingtoneSignature::SuruArpeggio => Some(include_bytes!("../../assets/ringtones/53757275.pcm")),
        RingtoneSignature::TimeNotLost => Some(include_bytes!("../../assets/ringtones/54696d65.pcm")),
        RingtoneSignature::WoodenDrive => Some(include_bytes!("../../assets/ringtones/576f6f64.pcm")),
        RingtoneSignature::Elysium => Some(include_bytes!("../../assets/ringtones/958f8a83.pcm")),
        _ => None,
    }
}

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::DropDown;
use gtk4::FileChooserAction;
use gtk4::FileChooserDialog;
use gtk4::Frame;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::ProgressBar;
use gtk4::Scale;
use gtk4::StringList;
use gtk4::ToggleButton;
use gtk4::glib;
use gtk4::prelude::*;
use tracing::warn;

/// Extract raw PCM data from a WAV file by finding the `data` chunk.
/// If the file does not start with `RIFF....WAVE`, it is returned as-is (assumed raw PCM).
fn extract_pcm_from_wav(data: &[u8]) -> &[u8] {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return data;
    }
    let mut offset = 12;
    while offset + 8 <= data.len() {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
        if chunk_id == b"data" {
            let start = offset + 8;
            let end = (start + chunk_size).min(data.len());
            return &data[start..end];
        }
        offset += 8 + chunk_size + (chunk_size & 1);
    }
    data
}

/// All selectable ringtone signatures in display order.
const RINGTONE_SIGNATURES: &[RingtoneSignature] = &[
    RingtoneSignature::Beep,
    RingtoneSignature::Digital,
    RingtoneSignature::Digital2,
    RingtoneSignature::Cuckoo,
    RingtoneSignature::Telephone,
    RingtoneSignature::ExoticGuitar,
    RingtoneSignature::LivelyPiano,
    RingtoneSignature::StoryPiano,
    RingtoneSignature::ForestPiano,
    RingtoneSignature::MonkeyIsland,
    RingtoneSignature::AlarmSynth,
    RingtoneSignature::ArrayMbira,
    RingtoneSignature::Bliss,
    RingtoneSignature::Celestial,
    RingtoneSignature::Entropy,
    RingtoneSignature::GlassMarimba,
    RingtoneSignature::HaloPentatonic,
    RingtoneSignature::Harmonics,
    RingtoneSignature::HarpArp,
    RingtoneSignature::KotoChords,
    RingtoneSignature::Sakenointi,
    RingtoneSignature::SamsSong,
    RingtoneSignature::Soul,
    RingtoneSignature::Sparkle,
    RingtoneSignature::Supreme,
    RingtoneSignature::SuruArpeggio,
    RingtoneSignature::TimeNotLost,
    RingtoneSignature::WoodenDrive,
    RingtoneSignature::Elysium,
    RingtoneSignature::CustomSlotA,
    RingtoneSignature::CustomSlotB,
    RingtoneSignature::Unused,
];

/// Derive a deterministic 4-byte signature from a filename by hashing.
///
/// Avoids collisions with all known built-in and slot signatures by
/// incrementing a salt until a non-colliding hash is found.
fn derive_custom_signature(filename: &str) -> [u8; 4] {
    let known: Vec<[u8; 4]> = RINGTONE_SIGNATURES.iter().map(|s| s.bytes()).collect();
    let mut attempt = 0u64;
    loop {
        let mut hasher = DefaultHasher::new();
        filename.hash(&mut hasher);
        attempt.hash(&mut hasher);
        let hash = hasher.finish().to_le_bytes();
        let sig = [hash[0], hash[1], hash[2], hash[3]];
        if !known.contains(&sig) {
            return sig;
        }
        attempt += 1;
    }
}

/// Scan `~/.config/cgd1-rs/ringtones/*.pcm` for custom ringtone files.
///
/// Returns a map from derived signature bytes to file path.
fn scan_custom_ringtones() -> HashMap<[u8; 4], PathBuf> {
    let mut map = HashMap::new();
    let config_dir = match dirs::config_dir() {
        Some(d) => d.join("cgd1-rs").join("ringtones"),
        None => return map,
    };
    let entries = match std::fs::read_dir(&config_dir) {
        Ok(e) => e,
        Err(_) => return map,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pcm") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let sig_bytes = derive_custom_signature(stem);
                map.insert(sig_bytes, path);
            }
        }
    }
    map
}

/// Embeddable audio editor widget for ringtone selection, preview, and upload.
#[allow(dead_code)]
pub struct AudioEditorWidget {
    /// Container box that can be placed inside a revealer.
    pub container: Box,
    /// Status label for operation feedback.
    pub status_label: Label,
    /// Ringtone signature dropdown.
    ringtone_dropdown: DropDown,
    /// Slot A toggle for custom upload.
    slot_a_toggle: ToggleButton,
    /// Slot B toggle for custom upload.
    slot_b_toggle: ToggleButton,
    /// Volume scale control.
    volume_scale: Scale,
    /// Clock manager for device communication.
    manager: Arc<ClockManager>,
    /// Async runtime.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Connected device address.
    connected_address: Arc<std::sync::Mutex<Option<MacAddress>>>,
    /// Map from signature bytes to file path for custom ringtones from ~/.config/cgd1-rs/ringtones/.
    custom_ringtone_files: HashMap<[u8; 4], PathBuf>,
    /// All ringtone signatures in dropdown order (built-in + custom + slots).
    ringtone_signatures: Vec<RingtoneSignature>,
}

/// Create a frame with an icon + title header.
fn audio_frame(icon_name: &str, title: &str) -> Frame {
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

/// Create a labeled settings row with a fixed-width label.
fn audio_row(label_text: &str) -> Box {
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

impl AudioEditorWidget {
    /// Build the audio editor content (without window chrome).
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

        // --- Active Ringtone frame ---
        let ringtone_frame = audio_frame("nf-cod-bell-symbolic", "Active Ringtone");
        let ringtone_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let custom_ringtone_files = scan_custom_ringtones();

        let builtin_sigs: Vec<RingtoneSignature> = RINGTONE_SIGNATURES
            .iter()
            .filter(|s| !matches!(s, RingtoneSignature::CustomSlotA | RingtoneSignature::CustomSlotB | RingtoneSignature::Unused))
            .copied()
            .collect();

        let mut custom_sigs: Vec<RingtoneSignature> = custom_ringtone_files.keys().map(|bytes| RingtoneSignature::from_bytes(*bytes)).collect();
        custom_sigs.sort_by_key(|s| {
            custom_ringtone_files
                .get(&s.bytes())
                .and_then(|p| p.file_stem().and_then(|s| s.to_str()).map(String::from))
                .unwrap_or_default()
        });

        let mut ringtone_signatures = builtin_sigs;
        ringtone_signatures.extend(custom_sigs);
        ringtone_signatures.push(RingtoneSignature::CustomSlotA);
        ringtone_signatures.push(RingtoneSignature::CustomSlotB);
        ringtone_signatures.push(RingtoneSignature::Unused);

        let ringtone_names: Vec<String> = ringtone_signatures
            .iter()
            .map(|s| {
                if let Some(path) = custom_ringtone_files.get(&s.bytes()) {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| s.name().to_string())
                } else {
                    s.name().to_string()
                }
            })
            .collect();
        let ringtone_names_ref: Vec<&str> = ringtone_names.iter().map(|s| s.as_str()).collect();
        let ringtone_model = StringList::new(&ringtone_names_ref);
        let ringtone_row = audio_row("Ringtone");
        let ringtone_dropdown = DropDown::new(Some(ringtone_model), None::<&gtk4::Expression>);
        ringtone_dropdown.set_selected(0);
        ringtone_dropdown.set_hexpand(true);
        ringtone_row.append(&ringtone_dropdown);
        ringtone_box.append(&ringtone_row);

        let volume_row = audio_row("Volume");
        let volume_scale = Scale::with_range(Orientation::Horizontal, 1.0, 5.0, 1.0);
        volume_scale.set_value(3.0);
        volume_scale.set_digits(0);
        volume_scale.set_hexpand(true);
        volume_scale.set_draw_value(true);
        volume_scale.set_value_pos(gtk4::PositionType::Right);
        volume_row.append(&volume_scale);
        ringtone_box.append(&volume_row);

        let ringtone_info = Label::builder()
            .label("Selects the ringtone and volume. Built-in ringtones are uploaded to the device (the firmware does not persist ringtone selection via settings).")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        ringtone_box.append(&ringtone_info);

        let ringtone_progress = ProgressBar::builder().fraction(0.0).hexpand(true).build();
        ringtone_box.append(&ringtone_progress);

        let ringtone_button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();
        let read_button = Button::builder()
            .icon_name("nf-cod-sync-symbolic")
            .label("Read")
            .tooltip_text("Read current ringtone from device")
            .build();
        let preview_button = Button::builder()
            .label("Preview (Beep)")
            .tooltip_text("Play a test beep at current volume")
            .build();
        let apply_ringtone_button = Button::builder().label("Apply").css_classes(["suggested-action"]).build();
        ringtone_button_box.append(&read_button);
        ringtone_button_box.append(&preview_button);
        ringtone_button_box.append(&apply_ringtone_button);
        ringtone_box.append(&ringtone_button_box);

        ringtone_frame.set_child(Some(&ringtone_box));
        content.append(&ringtone_frame);

        // --- Custom Upload frame ---
        let upload_frame = audio_frame("nf-fa-upload-symbolic", "Custom Upload");
        let upload_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let slot_row = audio_row("Target Slot");
        let slot_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["segmented"])
            .build();
        let slot_a_toggle = ToggleButton::builder().label("Slot A").css_classes(["segmented-btn"]).build();
        let slot_b_toggle = ToggleButton::builder().label("Slot B").css_classes(["segmented-btn"]).build();
        slot_a_toggle.set_active(true);
        slot_a_toggle.set_group(Some(&slot_b_toggle));
        slot_box.append(&slot_a_toggle);
        slot_box.append(&slot_b_toggle);
        slot_row.append(&slot_box);
        slot_row.append(&Box::builder().hexpand(true).build());
        upload_box.append(&slot_row);

        let file_label = Label::builder()
            .label("No file selected")
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        upload_box.append(&file_label);

        let select_button = Button::builder().label("Select Audio File…").halign(Align::Start).build();
        upload_box.append(&select_button);

        let upload_info = Label::builder()
            .label("8-bit PCM, 8 kHz, mono, max ~12 seconds. Always alternate slots between uploads.")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        upload_box.append(&upload_info);

        let progress = ProgressBar::builder().fraction(0.0).hexpand(true).build();
        upload_box.append(&progress);

        let upload_button = Button::builder().label("Upload").css_classes(["suggested-action"]).halign(Align::End).build();
        upload_box.append(&upload_button);

        upload_frame.set_child(Some(&upload_box));
        content.append(&upload_frame);

        scrolled.set_child(Some(&content));
        container.append(&scrolled);

        container.append(&gtk4::Separator::new(Orientation::Horizontal));

        let status_label = Label::builder().label("").css_classes(["dim-label"]).halign(Align::Start).hexpand(true).build();
        container.append(&status_label);

        // --- File chooser ---
        let file_label_clone = file_label.clone();
        select_button.connect_clicked(move |_| {
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Audio files"));
            filter.add_pattern("*.wav");
            filter.add_pattern("*.raw");
            filter.add_pattern("*.pcm");

            let dialog = FileChooserDialog::builder().title("Select Audio File").action(FileChooserAction::Open).build();
            dialog.add_filter(&filter);
            dialog.add_buttons(&[("Cancel", gtk4::ResponseType::Cancel), ("Open", gtk4::ResponseType::Accept)]);

            let label = file_label_clone.clone();
            dialog.connect_response(move |d, response| {
                if response == gtk4::ResponseType::Accept
                    && let Some(file) = d.file()
                {
                    let path = file.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                    label.set_label(&path);
                }
                d.close();
            });
            dialog.present();
        });

        // --- Read button ---
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let status_label = status_label.clone();
            let ringtone_dropdown = ringtone_dropdown.clone();
            let volume_scale = volume_scale.clone();
            let ringtone_signatures = ringtone_signatures.clone();

            read_button.connect_clicked(move |_| {
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
                let ringtone_dropdown = ringtone_dropdown.clone();
                let volume_scale = volume_scale.clone();
                let status_label = status_label.clone();
                let ringtone_signatures = ringtone_signatures.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(settings) => {
                                let sig = settings.ringtone_signature();
                                if let Some(idx) = ringtone_signatures.iter().position(|s| *s == sig) {
                                    ringtone_dropdown.set_selected(idx as u32);
                                }
                                volume_scale.set_value(settings.volume().value() as f64);
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
            });
        }

        // --- Preview button ---
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let status_label = status_label.clone();

            preview_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };
                status_label.set_label("Playing preview beep...");
                let manager = manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.preview_ringtone(None).await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label = status_label.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => status_label.set_label("Preview played"),
                            Err(e) => status_label.set_label(&format!("Preview failed: {e}")),
                        }
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        status_label.set_label("Preview task failed");
                        glib::ControlFlow::Break
                    }
                });
            });
        }

        // --- Apply ringtone button ---
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let status_label = status_label.clone();
            let ringtone_dropdown = ringtone_dropdown.clone();
            let volume_scale = volume_scale.clone();
            let ringtone_progress = ringtone_progress.clone();
            let read_button = read_button.clone();
            let ringtone_signatures = ringtone_signatures.clone();
            let custom_ringtone_files = custom_ringtone_files.clone();

            apply_ringtone_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };
                let selected = ringtone_dropdown.selected() as usize;
                let signature = ringtone_signatures[selected];
                let volume = match Volume::new(volume_scale.value() as u8) {
                    Ok(v) => v,
                    Err(e) => {
                        status_label.set_label(&format!("Invalid volume: {e}"));
                        return;
                    }
                };

                let pcm_data = if let Some(path) = custom_ringtone_files.get(&signature.bytes()) {
                    match std::fs::read(path) {
                        Ok(data) => Some(extract_pcm_from_wav(&data).to_vec()),
                        Err(e) => {
                            status_label.set_label(&format!("Failed to read custom ringtone: {e}"));
                            return;
                        }
                    }
                } else {
                    builtin_ringtone_pcm(signature).map(|d| d.to_vec())
                };
                let is_builtin = pcm_data.is_some();

                if is_builtin {
                    status_label.set_label("Uploading ringtone audio to device...");
                } else {
                    status_label.set_label("Writing ringtone to settings...");
                }

                ringtone_progress.set_fraction(0.0);
                let manager = manager.clone();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;

                        // For built-in ringtones, upload the PCM audio first.
                        if let Some(pcm) = pcm_data {
                            device.upload_ringtone(&pcm, signature.bytes()).await?;
                        }

                        // Write settings with the new signature and volume.
                        let current = device.read_settings().await?;
                        let updated = DeviceSettings::new(
                            volume,
                            current.time_format(),
                            current.temperature_unit(),
                            current.language(),
                            current.timezone(),
                            current.screen_light_duration(),
                            current.brightness(),
                            current.night_brightness(),
                            current.night_start(),
                            current.night_end(),
                            current.night_mode_enabled(),
                            current.master_alarm_disabled(),
                            signature,
                        )?;
                        device.write_settings(&updated).await?;
                        Ok(())
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e: ClockError| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label = status_label.clone();
                let ringtone_progress = ringtone_progress.clone();
                let read_button = read_button.clone();
                let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let done_timer = done.clone();
                let pulse_progress = ringtone_progress.clone();
                let timer = glib::source::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if done_timer.load(std::sync::atomic::Ordering::SeqCst) {
                        glib::ControlFlow::Break
                    } else {
                        pulse_progress.pulse();
                        glib::ControlFlow::Continue
                    }
                });
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        done.store(true, std::sync::atomic::Ordering::SeqCst);
                        match result {
                            Ok(()) => {
                                ringtone_progress.set_fraction(1.0);
                                status_label.set_label("Ringtone applied");
                                read_button.emit_clicked();
                            }
                            Err(e) => {
                                ringtone_progress.set_fraction(0.0);
                                status_label.set_label(&format!("Apply failed: {e}"));
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        done.store(true, std::sync::atomic::Ordering::SeqCst);
                        ringtone_progress.set_fraction(0.0);
                        status_label.set_label("Apply task failed");
                        glib::ControlFlow::Break
                    }
                });
                let _ = timer;
            });
        }

        // --- Upload button ---
        {
            let manager = manager.clone();
            let runtime = runtime.clone();
            let connected_address = connected_address.clone();
            let status_label = status_label.clone();
            let file_label = file_label.clone();
            let slot_a_toggle = slot_a_toggle.clone();
            let progress = progress.clone();
            let read_button = read_button.clone();

            upload_button.connect_clicked(move |_| {
                let addr = *connected_address.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = addr else {
                    status_label.set_label("No device connected");
                    return;
                };
                let file_path = file_label.label().to_string();
                if file_path == "No file selected" || file_path.is_empty() {
                    status_label.set_label("No file selected");
                    return;
                }
                let signature = if slot_a_toggle.is_active() {
                    RingtoneSignature::CustomSlotA
                } else {
                    RingtoneSignature::CustomSlotB
                };
                let raw = match std::fs::read(&file_path) {
                    Ok(data) => data,
                    Err(e) => {
                        status_label.set_label(&format!("Read failed: {e}"));
                        return;
                    }
                };
                let audio = extract_pcm_from_wav(&raw).to_vec();
                status_label.set_label(&format!("Uploading to {}...", signature.name()));
                progress.set_fraction(0.0);
                let manager = manager.clone();
                let sig_bytes = signature.bytes();
                let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
                runtime.spawn(async move {
                    let result = async {
                        let device = manager.device(&addr).await.ok_or_else(|| ClockError::Parse("device not found".into()))?;
                        device.upload_ringtone(&audio, sig_bytes).await
                    }
                    .await;
                    let _ = tx.send(result.map_err(|e| e.to_string()));
                });
                let rx = std::cell::RefCell::new(rx);
                let status_label = status_label.clone();
                let progress = progress.clone();
                let read_button = read_button.clone();
                let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let done_timer = done.clone();
                let pulse_progress = progress.clone();
                let timer = glib::source::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if done_timer.load(std::sync::atomic::Ordering::SeqCst) {
                        glib::ControlFlow::Break
                    } else {
                        pulse_progress.pulse();
                        glib::ControlFlow::Continue
                    }
                });
                let status_label_idle = status_label.clone();
                let progress_idle = progress.clone();
                let read_button_idle = read_button.clone();
                glib::source::idle_add_local(move || match rx.borrow_mut().try_recv() {
                    Ok(result) => {
                        done.store(true, std::sync::atomic::Ordering::SeqCst);
                        match result {
                            Ok(()) => {
                                progress_idle.set_fraction(1.0);
                                status_label_idle.set_label("Upload complete");
                                read_button_idle.emit_clicked();
                            }
                            Err(e) => {
                                progress_idle.set_fraction(0.0);
                                status_label_idle.set_label(&format!("Upload failed: {e}"));
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        done.store(true, std::sync::atomic::Ordering::SeqCst);
                        progress_idle.set_fraction(0.0);
                        status_label_idle.set_label("Upload task failed");
                        glib::ControlFlow::Break
                    }
                });
                let _ = timer;
            });
        }

        // Auto-read settings when the audio editor is opened.
        read_button.emit_clicked();

        Self {
            container,
            status_label,
            ringtone_dropdown,
            slot_a_toggle,
            slot_b_toggle,
            volume_scale,
            manager,
            runtime,
            connected_address,
            custom_ringtone_files,
            ringtone_signatures,
        }
    }
}
