#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod display;

fn main() {
    match display::enumerate_displays() {
        Ok(displays) => {
            for display in displays {
                println!("{}: {} Hz", display.friendly_name, display.current_hz());
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}
