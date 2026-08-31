use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::FileChooserAction;
use gtk4::FileChooserDialog;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::ProgressBar;
use gtk4::Window;
use gtk4::prelude::*;

/// Audio dialog for ringtone upload and preview.
#[allow(dead_code)]
pub struct AudioDialog {
    window: Window,
}

impl AudioDialog {
    /// Create and show the audio dialog.
    pub fn new(parent: &Window) -> Self {
        let window = gtk4::Window::builder()
            .title("Audio — Alarm Clock CGD1")
            .transient_for(parent)
            .modal(true)
            .default_width(400)
            .default_height(300)
            .build();

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        let title = Label::builder().label("Custom Ringtone").css_classes(["title-2"]).halign(Align::Start).build();
        main_box.append(&title);

        let info_label = Label::builder()
            .label("Upload a custom ringtone (8-bit PCM, 8 kHz, mono, max ~12 seconds).\nConnect to a device to upload.")
            .wrap(true)
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build();
        main_box.append(&info_label);

        let file_label = Label::builder().label("No file selected").halign(Align::Start).build();
        main_box.append(&file_label);

        let select_button = Button::builder().label("Select Audio File…").halign(Align::Start).build();
        main_box.append(&select_button);

        let progress = ProgressBar::builder().fraction(0.0).hexpand(true).build();
        main_box.append(&progress);

        let status_label = Label::builder().label("").halign(Align::Start).build();
        main_box.append(&status_label);

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();
        let preview_button = Button::builder().label("Preview Ringtone").build();
        let upload_button = Button::builder().label("Upload").css_classes(["suggested-action"]).build();
        let close_button = Button::builder().label("Close").build();
        button_box.append(&preview_button);
        button_box.append(&upload_button);
        button_box.append(&close_button);
        main_box.append(&button_box);

        let win = window.clone();
        close_button.connect_clicked(move |_| {
            win.close();
        });

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

        window.set_child(Some(&main_box));
        window.present();

        Self { window }
    }
}
