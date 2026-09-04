use std::collections::HashMap;

use cgd1_rs::MacAddress;
use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::ColumnView;
use gtk4::ColumnViewColumn;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Window;
use gtk4::gio;
use gtk4::prelude::*;

use crate::device_runtime_state::DeviceRuntimeState;

/// Sensor overview dialog showing a table of all known devices and their sensor data.
#[allow(dead_code)]
pub struct SensorOverviewDialog {
    window: Window,
}

impl SensorOverviewDialog {
    /// Create and show the sensor overview dialog.
    pub fn new(parent: &Window, device_states: &HashMap<MacAddress, DeviceRuntimeState>, known_devices: &[MacAddress]) -> Self {
        let window = gtk4::Window::builder()
            .title("Sensor Overview - Alarm Clock CGD1")
            .transient_for(parent)
            .modal(true)
            .default_width(500)
            .default_height(360)
            .build();

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let (model, column_view) = build_column_view();
        populate_model(&model, device_states, known_devices);
        main_box.append(&column_view);

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();

        let close_button = Button::builder().label("Close").build();

        let win = window.clone();
        close_button.connect_clicked(move |_| {
            win.close();
        });

        button_box.append(&close_button);
        main_box.append(&button_box);

        window.set_child(Some(&main_box));
        window.present();

        Self { window }
    }
}

/// Build the ColumnView with columns for MAC, temperature, humidity, and battery.
fn build_column_view() -> (gio::ListStore, ColumnView) {
    let model = gio::ListStore::new::<DeviceRowItem>();

    let selection_model = gtk4::SingleSelection::new(Some(model.clone()));
    let column_view = ColumnView::new(Some(selection_model));

    let addr_col = build_text_column("MAC Address", |item| item.address());
    let temp_col = build_text_column("Temp", |item| item.temperature());
    let humidity_col = build_text_column("Humidity", |item| item.humidity());
    let battery_col = build_text_column("Battery", |item| item.battery());

    column_view.append_column(&addr_col);
    column_view.append_column(&temp_col);
    column_view.append_column(&humidity_col);
    column_view.append_column(&battery_col);

    (model, column_view)
}

/// Build a single column with a text cell.
fn build_text_column<F>(title: &str, bind_fn: F) -> ColumnViewColumn
where
    F: Fn(&DeviceRowItem) -> String + 'static,
{
    let factory = gtk4::SignalListItemFactory::new();

    factory.connect_setup(move |_factory, item| {
        let label = Label::builder().halign(Align::Start).build();
        item.set_child(Some(&label));
    });

    factory.connect_bind(move |_factory, item| {
        let child = match item.child() {
            Some(c) => c,
            None => return,
        };
        let label = match child.downcast::<Label>().ok() {
            Some(l) => l,
            None => return,
        };
        let row_item = match item.item().and_then(|i| i.downcast::<DeviceRowItem>().ok()) {
            Some(r) => r,
            None => return,
        };
        let text = bind_fn(&row_item);
        label.set_label(&text);
    });

    ColumnViewColumn::builder().title(title).factory(&factory).build()
}

/// Populate the ListStore from device_states and known_devices.
fn populate_model(model: &gio::ListStore, device_states: &HashMap<MacAddress, DeviceRuntimeState>, known_devices: &[MacAddress]) {
    for addr in known_devices {
        let state = device_states.get(addr);
        let item = DeviceRowItem::new(addr, state);
        model.append(&item);
    }
}

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use gtk4::glib;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::DeviceRowItem)]
    pub struct DeviceRowItem {
        #[property(get, set)]
        pub address: RefCell<String>,
        #[property(get, set)]
        pub temperature: RefCell<String>,
        #[property(get, set)]
        pub humidity: RefCell<String>,
        #[property(get, set)]
        pub battery: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceRowItem {
        const NAME: &'static str = "DeviceRowItem";
        type Type = super::DeviceRowItem;
        type ParentType = glib::Object;
        type Interfaces = ();
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceRowItem {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct DeviceRowItem(ObjectSubclass<imp::DeviceRowItem>);
}

impl DeviceRowItem {
    fn new(addr: &MacAddress, state: Option<&DeviceRuntimeState>) -> Self {
        let (temperature, humidity, battery) = match state {
            Some(s) => (
                s.temperature.map_or("--".to_string(), |t| format!("{:.1} °C", t)),
                s.humidity.map_or("--".to_string(), |h| format!("{:.0} %", h)),
                s.battery_level.map_or("--".to_string(), |b| format!("{:.0} %", b)),
            ),
            None => ("--".to_string(), "--".to_string(), "--".to_string()),
        };
        let obj: Self = glib::Object::new();
        obj.set_address(&addr.to_string() as &str);
        obj.set_temperature(&temperature as &str);
        obj.set_humidity(&humidity as &str);
        obj.set_battery(&battery as &str);
        obj
    }
}
