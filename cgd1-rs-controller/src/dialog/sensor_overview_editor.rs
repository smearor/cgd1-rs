use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use cgd1_rs::MacAddress;

use gtk4::Align;
use gtk4::Box;
use gtk4::Button;
use gtk4::ColumnView;
use gtk4::ColumnViewColumn;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;

use crate::device_runtime_state::DeviceRuntimeState;

/// Reusable sensor overview widget - can be embedded in the main window or a dialog.
#[allow(dead_code)]
pub struct SensorOverviewWidget {
    /// The container box holding all sensor overview content.
    pub container: Box,
    /// Refresh button.
    pub refresh_button: Button,
    /// The ListStore backing the column view.
    model: gio::ListStore,
}

impl SensorOverviewWidget {
    /// Build the sensor overview content (without window chrome).
    pub fn new(device_states: Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>>, known_devices: Arc<Mutex<Vec<MacAddress>>>) -> Self {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let (model, column_view) = build_column_view();
        container.append(&column_view);

        let button_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::End).build();
        let refresh_button = Button::builder().label("Refresh").build();
        button_box.append(&refresh_button);
        container.append(&button_box);

        let widget = Self {
            container,
            refresh_button: refresh_button.clone(),
            model: model.clone(),
        };

        // Initial populate
        widget.populate(&device_states, &known_devices);

        // Refresh button
        {
            let model = model.clone();
            let device_states = device_states.clone();
            let known_devices = known_devices.clone();
            refresh_button.connect_clicked(move |_| {
                populate_model(&model, &device_states, &known_devices);
            });
        }

        widget
    }

    /// Populate the model from the current device states and known devices.
    pub fn populate(&self, device_states: &Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>>, known_devices: &Arc<Mutex<Vec<MacAddress>>>) {
        populate_model(&self.model, device_states, known_devices);
    }
}

/// Build the ColumnView with columns for MAC, temperature, humidity, and battery.
fn build_column_view() -> (gio::ListStore, ColumnView) {
    let model = gio::ListStore::new::<DeviceRowItem>();

    let selection_model = gtk4::SingleSelection::new(Some(model.clone()));
    let column_view = ColumnView::new(Some(selection_model));
    column_view.set_hexpand(true);
    column_view.set_vexpand(true);

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
fn populate_model(model: &gio::ListStore, device_states: &Arc<Mutex<HashMap<MacAddress, DeviceRuntimeState>>>, known_devices: &Arc<Mutex<Vec<MacAddress>>>) {
    model.remove_all();
    let states = device_states.lock().unwrap_or_else(|p| p.into_inner());
    let known = known_devices.lock().unwrap_or_else(|p| p.into_inner());
    for addr in known.iter() {
        let state = states.get(addr);
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

glib::wrapper! {
    pub struct DeviceRowItem(ObjectSubclass<imp::DeviceRowItem>);
}

impl DeviceRowItem {
    fn new(addr: &MacAddress, state: Option<&DeviceRuntimeState>) -> Self {
        let (temperature, humidity, battery) = match state {
            Some(s) => (
                s.temperature.map_or("--".to_string(), |t| format!("{:.1} °C", t.value())),
                s.humidity.map_or("--".to_string(), |h| format!("{:.0} %", h.value())),
                s.battery_level.map_or("--".to_string(), |b| format!("{:.0} %", b.value())),
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
