mod imp {
    use std::cell::RefCell;
    use std::sync::Once;

    use glib::Properties;
    use gtk4::CssProvider;
    use gtk4::Snapshot;
    use gtk4::pango;
    use gtk4::pango::prelude::FontMapExt;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;

    /// Bundled DSEG7 Classic Regular font (7-segment style).
    const DSEG7_CLASSIC_REGULAR: &[u8] = include_bytes!("../assets/fonts/DSEG7Classic-Regular.ttf");
    /// Bundled DSEG7 Classic Light font (for dim/ghost segments).
    const DSEG7_CLASSIC_LIGHT: &[u8] = include_bytes!("../assets/fonts/DSEG7Classic-Light.ttf");

    static FONT_LOAD: Once = Once::new();

    /// Load bundled DSEG7 fonts into Pango's font map at runtime.
    /// Fonts are embedded in the binary and written to temp files for Pango to load.
    fn load_fonts(obj: &super::SevenSegmentDisplay) {
        FONT_LOAD.call_once(|| {
            let Some(font_map) = obj.pango_context().font_map() else {
                return;
            };
            let tmp = std::env::temp_dir();
            let regular_path = tmp.join("cgd1-DSEG7Classic-Regular.ttf");
            let light_path = tmp.join("cgd1-DSEG7Classic-Light.ttf");
            if std::fs::write(&regular_path, DSEG7_CLASSIC_REGULAR).is_ok() {
                let _ = font_map.add_font_file(&regular_path);
            }
            if std::fs::write(&light_path, DSEG7_CLASSIC_LIGHT).is_ok() {
                let _ = font_map.add_font_file(&light_path);
            }
        });
    }

    const CSS: &str = "
seven-segment-display {
    color: #00ff41;
    font-family: 'DSEG7 Classic', 'Courier New', monospace;
    padding: 6px;
}
seven-segment-display .dim {
    color: #003311;
}
";

    /// Internal implementation of [`super::SevenSegmentDisplay`].
    #[derive(Properties)]
    #[properties(wrapper_type = super::SevenSegmentDisplay)]
    pub struct SevenSegmentDisplay {
        /// The text to display.
        #[property(get, set)]
        pub text: RefCell<String>,
        /// Font size scale factor.
        #[property(get, set = Self::set_scale_impl)]
        pub scale: RefCell<f64>,
    }

    impl SevenSegmentDisplay {
        fn set_scale_impl(&self, value: f64) {
            self.scale.replace(value);
            self.obj().queue_resize();
            self.obj().queue_draw();
        }
    }

    impl Default for SevenSegmentDisplay {
        fn default() -> Self {
            Self {
                text: RefCell::new(String::new()),
                scale: RefCell::new(1.0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SevenSegmentDisplay {
        const NAME: &'static str = "SevenSegmentDisplay";
        type Type = super::SevenSegmentDisplay;
        type ParentType = gtk4::Widget;
        type Interfaces = ();
    }

    #[glib::derived_properties]
    impl ObjectImpl for SevenSegmentDisplay {
        fn constructed(&self) {
            self.parent_constructed();
            load_fonts(self.obj().as_ref());
            let provider = CssProvider::new();
            provider.load_from_data(CSS);
            let context = self.obj().style_context();
            context.add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
    }

    impl WidgetImpl for SevenSegmentDisplay {
        fn measure(&self, orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let scale = *self.scale.borrow();
            let text = self.text.borrow();
            let display_text = if text.is_empty() { "0" } else { &*text };
            let font_size = (48.0 * scale) as i32;

            let char_width = font_size / 2;
            let char_height = font_size;
            let text_len = display_text.len().max(1) as i32;

            match orientation {
                gtk4::Orientation::Horizontal => {
                    let min = char_width * text_len + 12;
                    let nat = min;
                    (min, nat, -1, -1)
                }
                _ => {
                    let min = char_height + 12;
                    let nat = min;
                    (min, nat, -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            let obj = self.obj();
            let text = self.text.borrow();
            let scale = *self.scale.borrow();
            let font_size = 48.0 * scale;

            if text.is_empty() {
                return;
            }

            let width = obj.width() as f64;
            let height = obj.height() as f64;

            let layout = obj.create_pango_layout(Some(&text));
            let mut desc = pango::FontDescription::new();
            desc.set_family("DSEG7 Classic");
            desc.set_size((font_size * pango::SCALE as f64) as i32);
            layout.set_font_description(Some(&desc));

            let (text_width, text_height) = layout.pixel_size();
            if text_width == 0 || text_height == 0 {
                return;
            }

            let x = (width - text_width as f64) / 2.0;
            let y = (height - text_height as f64) / 2.0;

            let dim_layout = obj.create_pango_layout(Some(&text));
            dim_layout.set_font_description(Some(&desc));

            snapshot.save();
            snapshot.translate(&gtk4::graphene::Point::new(x as f32, y as f32));

            let dim_color = gtk4::gdk::RGBA::new(0.0, 0.2, 0.07, 1.0);
            snapshot.append_layout(&dim_layout, &dim_color);

            let bright_color = gtk4::gdk::RGBA::new(0.0, 1.0, 0.25, 1.0);
            snapshot.append_layout(&layout, &bright_color);

            snapshot.restore();
        }
    }
}

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

glib::wrapper! {
    /// A 7-segment-style display widget that shows text with a digital clock aesthetic.
    ///
    /// Renders dim "ghost" segments behind the active text to simulate the
    /// appearance of a real 7-segment LED/LCD display.
    pub struct SevenSegmentDisplay(ObjectSubclass<imp::SevenSegmentDisplay>)
        @extends gtk4::Widget,
        @implements gtk4::Buildable, gtk4::ConstraintTarget;
}

impl SevenSegmentDisplay {
    /// Create a new empty 7-segment display.
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Set the display text.
    pub fn set_display_text(&self, text: &str) {
        self.imp().text.replace(text.to_string());
        self.queue_resize();
        self.queue_draw();
    }

    /// Set the font size scale (1.0 = default).
    pub fn set_font_scale(&self, scale: f64) {
        self.imp().scale.replace(scale);
        self.queue_resize();
        self.queue_draw();
    }
}

impl Default for SevenSegmentDisplay {
    fn default() -> Self {
        Self::new()
    }
}
