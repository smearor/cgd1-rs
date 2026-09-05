use crate::fl;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Picture;
use gtk4::Window;
use gtk4::glib;
use gtk4::prelude::*;

/// SVG image of the CGD1 alarm clock rendered as a data URI.
const CLOCK_SVG: &str = include_str!("../../assets/clock.svg");

/// Info dialog showing application title, clock image, repository link, and license.
#[allow(dead_code)]
pub struct InfoDialog {
    window: Window,
}

impl InfoDialog {
    /// Create and show the info dialog.
    pub fn new(parent: &Window) -> Self {
        let window = gtk4::Window::builder()
            .title(&fl!("info-dialog-title"))
            .transient_for(parent)
            .modal(true)
            .default_width(360)
            .default_height(420)
            .build();

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .halign(Align::Center)
            .build();

        let title = Label::builder()
            .label(&fl!("info-app-name"))
            .css_classes(["title-1"])
            .halign(Align::Center)
            .build();
        main_box.append(&title);

        let svg_bytes = glib::Bytes::from(CLOCK_SVG.as_bytes());
        let texture = gtk4::gdk::Texture::from_bytes(&svg_bytes).ok();
        if let Some(texture) = texture {
            let picture = Picture::builder().paintable(&texture).halign(Align::Center).build();
            main_box.append(&picture);
        }

        let link_button = gtk4::LinkButton::builder()
            .label(&fl!("info-github-link"))
            .uri("https://github.com/smearor/cgd1-rs")
            .halign(Align::Center)
            .build();
        main_box.append(&link_button);

        let license_label = Label::builder()
            .label(&fl!("info-license-text"))
            .wrap(true)
            .halign(Align::Center)
            .valign(Align::Start)
            .vexpand(true)
            .build();
        main_box.append(&license_label);

        let close_button = Button::builder().label(&fl!("info-close")).halign(Align::Center).build();
        let win = window.clone();
        close_button.connect_clicked(move |_| {
            win.close();
        });
        main_box.append(&close_button);

        window.set_child(Some(&main_box));
        window.present();

        Self { window }
    }
}
