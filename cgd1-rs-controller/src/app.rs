use cgd1_rs::Backend;
use gio::ApplicationFlags;
use glib::clone;
use gtk4::Application;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;

use crate::dialog::AlarmsDialog;
use crate::dialog::AudioDialog;
use crate::dialog::InfoDialog;
use crate::dialog::SettingsDialog;
use crate::window::MainWindow;

/// GTK 4 application for the CGD1 alarm clock controller.
pub struct ClockControllerApp {
    app: Application,
    backend: Backend,
}

impl ClockControllerApp {
    /// Create a new application instance with the given BLE backend.
    pub fn new(backend: Backend) -> Self {
        let app = Application::new(Some("com.github.smearor.cgd1-rs.controller"), ApplicationFlags::FLAGS_NONE);
        Self { app, backend }
    }

    /// Run the application.
    pub fn run(&self) {
        let backend = self.backend;
        self.app.connect_activate(clone!(move |app| {
            let window = MainWindow::new(app, backend);
            window.present();

            let manager = window.manager().clone();
            let runtime = window.runtime().clone();
            let connected_address = window.connected_address_arc();

            // Ensure the process exits when the window is closed.
            // The tokio runtime keeps background threads alive otherwise.
            window.window().connect_close_request(move |_| {
                std::process::exit(0);
            });

            let manager_alarms = manager.clone();
            let runtime_alarms = runtime.clone();
            let connected_address_alarms = connected_address.clone();
            add_action(app, "alarms", window.window(), move |w| {
                let _ = AlarmsDialog::new(w, manager_alarms.clone(), runtime_alarms.clone(), connected_address_alarms.clone());
            });
            let manager_settings = manager.clone();
            let runtime_settings = runtime.clone();
            let connected_address_settings = connected_address.clone();
            add_action(app, "settings", window.window(), move |w| {
                let _ = SettingsDialog::new(w, manager_settings.clone(), runtime_settings.clone(), connected_address_settings.clone());
            });
            add_action(app, "audio", window.window(), |w| {
                let _ = AudioDialog::new(w);
            });
            add_action(app, "info", window.window(), |w| {
                let _ = InfoDialog::new(w);
            });
        }));

        let _ = self.app.run_with_args(&["cgd1-controller"]);
    }
}

fn add_action<F>(app: &Application, name: &str, window: &gtk4::Window, callback: F)
where
    F: Fn(&gtk4::Window) + 'static,
{
    let action = gio::SimpleAction::new(name, None);
    let win = window.clone();
    action.connect_activate(move |_, _| {
        callback(&win);
    });
    app.add_action(&action);
}
