#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod display;
mod settings;
mod strings;
mod tray;

fn main() {
    if let Err(error) = app::run() {
        app::show_fatal_error(&error.to_string());
    }
}
