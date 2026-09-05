use crate::dialog::InfoDialog;
use crate::fl;
use crate::window::MainWindow;
use cgd1_rs::Backend;
use cgd1_rs::TokenStore;
use gio::ApplicationFlags;
use glib::clone;
use gtk4::Application;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use tracing::info;
use tracing::warn;

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
            nerd_fonts_gtk::init(None);
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::IconTheme::for_display(&display).add_resource_path("/com/nerd/icons");
            }

            let window = MainWindow::new(app, backend);
            window.present();

            let manager = window.manager().clone();
            let connected_address = window.selected_address_arc();

            // Ensure the process exits when the window is closed.
            // The tokio runtime keeps background threads alive otherwise.
            window.window().connect_close_request(move |_| {
                std::process::exit(0);
            });

            add_action(app, "info", window.window(), |w| {
                let _ = InfoDialog::new(w);
            });

            let token_store_reset = window.token_store_arc();
            let selected_address_reset = connected_address.clone();
            let manager_reset = manager.clone();
            let connect_switch_reset = window.connect_switch_arc();
            add_action(app, "reset_token", window.window(), move |w| {
                let addr = selected_address_reset.lock().unwrap_or_else(|p| {
                    warn!("mutex poisoned - recovering");
                    p.into_inner()
                });
                let Some(addr) = *addr else {
                    let dialog = gtk4::MessageDialog::builder()
                        .transient_for(w)
                        .modal(true)
                        .message_type(gtk4::MessageType::Warning)
                        .buttons(gtk4::ButtonsType::Ok)
                        .text(&fl!("dialog-no-device-title"))
                        .secondary_text(&fl!("dialog-no-device-body"))
                        .build();
                    dialog.connect_response(|d, _| d.close());
                    dialog.present();
                    return;
                };

                let dialog = gtk4::MessageDialog::builder()
                    .transient_for(w)
                    .modal(true)
                    .message_type(gtk4::MessageType::Question)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text(&fl!("dialog-reset-token-title"))
                    .secondary_text(&fl!("dialog-reset-token-body", addr = addr.to_string()))
                    .build();

                let token_store = token_store_reset.clone();
                let manager = manager_reset.clone();
                let connect_switch = connect_switch_reset.clone();
                let addr_for_delete = addr;
                dialog.connect_response(move |d, response| {
                    if response == gtk4::ResponseType::Yes {
                        if let Err(e) = token_store.delete(&addr_for_delete) {
                            warn!(%addr_for_delete, error = %e, "failed to delete token");
                        } else {
                            info!(%addr_for_delete, "token deleted by user");
                        }
                        // Disconnect if connected
                        let manager = manager.clone();
                        let connect_switch = connect_switch.clone();
                        glib::spawn_future_local(clone!(async move {
                            let _ = manager.disconnect(&addr_for_delete).await;
                            connect_switch.set_active(false);
                        }));
                    }
                    d.close();
                });
                dialog.present();
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
