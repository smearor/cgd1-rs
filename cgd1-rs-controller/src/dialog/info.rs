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
            .title("Info — Alarm Clock CGD1")
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
            .label("Alarm Clock CGD1")
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
            .label("github.com/smearor/cgd1-rs")
            .uri("https://github.com/smearor/cgd1-rs")
            .halign(Align::Center)
            .build();
        main_box.append(&link_button);

        let license_label = Label::builder()
            .label("Licensed under the MIT License\n\nCopyright (c) 2024 smearor\n\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND.")
            .wrap(true)
            .halign(Align::Center)
            .valign(Align::Start)
            .vexpand(true)
            .build();
        main_box.append(&license_label);

        let close_button = Button::builder().label("Close").halign(Align::Center).build();
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
