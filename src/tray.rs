use std::collections::HashMap;
use std::mem::size_of;

use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NIM_SETFOCUS, NIM_SETVERSION, NOTIFY_ICON_INFOTIP_FLAGS,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, IDI_APPLICATION, LoadIconW,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_DISABLED, MF_POPUP, MF_SEPARATOR, MF_STRING, PostMessageW,
    SetForegroundWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_NULL,
};
use windows::core::{Error, PCWSTR, Result};

use crate::display::{Display, format_hz};
use crate::strings::Strings;

const TRAY_ICON_ID: u32 = 1;
const FIRST_REFRESH_COMMAND: u32 = 1_000;
const STARTUP_COMMAND: u32 = 10;
const DISPLAY_SETTINGS_COMMAND: u32 = 11;
const ABOUT_COMMAND: u32 = 12;
const EXIT_COMMAND: u32 = 13;

pub struct TrayIcon {
    hwnd: HWND,
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    tooltip: String,
    added: bool,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Result<Self> {
        // SAFETY: IDI_APPLICATION is a predefined shared system icon.
        let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }?;
        Ok(Self {
            hwnd,
            icon,
            tooltip: "HzPilot".to_owned(),
            added: false,
        })
    }

    pub fn add(&mut self, callback_message: u32) -> Result<()> {
        let mut data = self.base_data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = callback_message;
        data.hIcon = self.icon;
        copy_wide(&mut data.szTip, &self.tooltip);

        // SAFETY: data is fully initialized for NIM_ADD.
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
            return Err(shell_error("Shell_NotifyIconW(NIM_ADD) failed"));
        }
        self.added = true;

        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        // SAFETY: the icon was added above and data identifies it by hwnd and ID.
        if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) }.as_bool() {
            self.remove();
            return Err(shell_error("Shell_NotifyIconW(NIM_SETVERSION) failed"));
        }
        Ok(())
    }

    pub fn readd(&mut self, callback_message: u32) -> Result<()> {
        self.added = false;
        self.add(callback_message)
    }

    pub fn set_tooltip(&mut self, tooltip: String) -> Result<()> {
        self.tooltip = tooltip;
        if !self.added {
            return Ok(());
        }

        let mut data = self.base_data();
        data.uFlags = NIF_TIP | NIF_SHOWTIP;
        copy_wide(&mut data.szTip, &self.tooltip);
        // SAFETY: data identifies an existing icon and contains a valid tooltip buffer.
        if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
            Ok(())
        } else {
            Err(shell_error("Shell_NotifyIconW(NIM_MODIFY) failed"))
        }
    }

    pub fn notify_info(&self, title: &str, message: &str) {
        self.notify(title, message, NIIF_INFO);
    }

    pub fn notify_error(&self, title: &str, message: &str) {
        self.notify(title, message, NIIF_ERROR);
    }

    pub fn focus(&self) {
        let data = self.base_data();
        // SAFETY: data identifies this tray icon. Failure is non-fatal.
        let _ = unsafe { Shell_NotifyIconW(NIM_SETFOCUS, &data) };
    }

    pub fn remove(&mut self) {
        if !self.added {
            return;
        }
        let data = self.base_data();
        // SAFETY: data identifies this tray icon. Explorer may already have removed it.
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
        self.added = false;
    }

    fn notify(&self, title: &str, message: &str, flags: NOTIFY_ICON_INFOTIP_FLAGS) {
        if !self.added {
            return;
        }
        let mut data = self.base_data();
        data.uFlags = NIF_INFO;
        data.dwInfoFlags = flags;
        copy_wide(&mut data.szInfoTitle, title);
        copy_wide(&mut data.szInfo, message);
        // SAFETY: data identifies an existing icon and all text buffers are initialized.
        let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
    }

    fn base_data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}

#[derive(Clone, Debug)]
pub enum MenuAction {
    SetRefresh { device_name: String, hz: u32 },
    ToggleStartup,
    OpenDisplaySettings,
    About,
    Exit,
}

pub struct PopupMenu {
    handle: HMENU,
    actions: HashMap<u32, MenuAction>,
}

impl PopupMenu {
    pub fn build(
        displays: &[Display],
        strings: &Strings,
        full: bool,
        startup_enabled: bool,
    ) -> Result<Self> {
        let root = OwnedMenu::new()?;
        let mut actions = HashMap::new();
        let mut next_command = FIRST_REFRESH_COMMAND;

        if displays.is_empty() {
            append_text(
                root.handle(),
                MF_STRING | MF_DISABLED,
                0,
                strings.no_displays,
            )?;
        } else if displays.len() == 1 {
            let display = &displays[0];
            append_text(
                root.handle(),
                MF_STRING | MF_DISABLED,
                0,
                &display_label(display, strings),
            )?;
            append_refresh_rates(
                root.handle(),
                display,
                strings,
                &mut actions,
                &mut next_command,
            )?;
        } else {
            for display in displays {
                let submenu = OwnedMenu::new()?;
                append_refresh_rates(
                    submenu.handle(),
                    display,
                    strings,
                    &mut actions,
                    &mut next_command,
                )?;
                append_popup(
                    root.handle(),
                    submenu.handle(),
                    &display_label(display, strings),
                )?;
                submenu.release();
            }
        }

        if full {
            append_separator(root.handle())?;
            let startup_flags = if startup_enabled {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            };
            append_text(
                root.handle(),
                startup_flags,
                STARTUP_COMMAND as usize,
                strings.startup,
            )?;
            actions.insert(STARTUP_COMMAND, MenuAction::ToggleStartup);

            append_text(
                root.handle(),
                MF_STRING,
                DISPLAY_SETTINGS_COMMAND as usize,
                strings.display_settings,
            )?;
            actions.insert(DISPLAY_SETTINGS_COMMAND, MenuAction::OpenDisplaySettings);

            append_separator(root.handle())?;
            append_text(
                root.handle(),
                MF_STRING,
                ABOUT_COMMAND as usize,
                strings.about,
            )?;
            actions.insert(ABOUT_COMMAND, MenuAction::About);
            append_text(
                root.handle(),
                MF_STRING,
                EXIT_COMMAND as usize,
                strings.exit,
            )?;
            actions.insert(EXIT_COMMAND, MenuAction::Exit);
        }

        Ok(Self {
            handle: root.release(),
            actions,
        })
    }

    pub fn show(&self, hwnd: HWND) -> Result<Option<MenuAction>> {
        let mut point = POINT::default();
        // SAFETY: point is valid writable storage.
        unsafe { GetCursorPos(&mut point) }?;
        // SAFETY: hwnd is the hidden application window used by the tray icon.
        let _ = unsafe { SetForegroundWindow(hwnd) };
        // SAFETY: the menu and owner window remain alive for the duration of this call.
        let command = unsafe {
            TrackPopupMenuEx(
                self.handle,
                (TPM_RIGHTBUTTON | TPM_RETURNCMD).0,
                point.x,
                point.y,
                hwnd,
                None,
            )
        }
        .0 as u32;
        // SAFETY: posting WM_NULL is the documented way to complete tray menu dismissal.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };

        Ok(self.actions.get(&command).cloned())
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        // SAFETY: PopupMenu owns the root menu and all attached submenus.
        let _ = unsafe { DestroyMenu(self.handle) };
    }
}

fn append_refresh_rates(
    menu: HMENU,
    display: &Display,
    strings: &Strings,
    actions: &mut HashMap<u32, MenuAction>,
    next_command: &mut u32,
) -> Result<()> {
    for mode in &display.modes {
        let command = *next_command;
        *next_command += 1;
        let flags = if display.is_current_mode(mode) {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        append_text(
            menu,
            flags,
            command as usize,
            &format!("{} {}", mode.hz, strings.unit),
        )?;
        actions.insert(
            command,
            MenuAction::SetRefresh {
                device_name: display.device_name.clone(),
                hz: mode.hz,
            },
        );
    }
    Ok(())
}

fn display_label(display: &Display, strings: &Strings) -> String {
    let mut qualifiers = Vec::new();
    if display.is_primary {
        qualifiers.push(strings.primary);
    }
    if display.dynamic_refresh {
        qualifiers.push(strings.dynamic);
    }
    let qualifiers = if qualifiers.is_empty() {
        String::new()
    } else {
        format!(" ({})", qualifiers.join(", "))
    };
    format!(
        "{}{} - {} {}",
        display.friendly_name,
        qualifiers,
        format_hz(display.current_hz()),
        strings.unit
    )
}

fn append_text(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, text: &str) -> Result<()> {
    let text = menu_text(text);
    // SAFETY: text is NUL terminated and lives through the synchronous API call.
    unsafe { AppendMenuW(menu, flags, id, PCWSTR(text.as_ptr())) }
}

fn append_popup(parent: HMENU, submenu: HMENU, text: &str) -> Result<()> {
    append_text(parent, MF_POPUP, submenu.0 as usize, text)
}

fn append_separator(menu: HMENU) -> Result<()> {
    // SAFETY: separator items ignore the command ID and text pointer.
    unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) }
}

fn menu_text(value: &str) -> Vec<u16> {
    value.replace('&', "&&").encode_utf16().chain([0]).collect()
}

struct OwnedMenu(Option<HMENU>);

impl OwnedMenu {
    fn new() -> Result<Self> {
        // SAFETY: CreatePopupMenu has no preconditions.
        unsafe { CreatePopupMenu() }.map(|menu| Self(Some(menu)))
    }

    fn handle(&self) -> HMENU {
        self.0
            .expect("menu handle must exist until ownership is released")
    }

    fn release(mut self) -> HMENU {
        self.0
            .take()
            .expect("menu handle can only be released once")
    }
}

impl Drop for OwnedMenu {
    fn drop(&mut self) {
        if let Some(menu) = self.0.take() {
            // SAFETY: OwnedMenu owns unattached menus until release.
            let _ = unsafe { DestroyMenu(menu) };
        }
    }
}

fn copy_wide<const SIZE: usize>(buffer: &mut [u16; SIZE], value: &str) {
    buffer.fill(0);
    if SIZE == 0 {
        return;
    }

    let encoded: Vec<u16> = value.encode_utf16().collect();
    let mut length = encoded.len().min(SIZE - 1);
    if length > 0 && (0xD800..=0xDBFF).contains(&encoded[length - 1]) {
        length -= 1;
    }
    buffer[..length].copy_from_slice(&encoded[..length]);
}

fn shell_error(message: &str) -> Error {
    Error::new(E_FAIL, message)
}

#[cfg(test)]
mod tests {
    use super::{copy_wide, menu_text};

    #[test]
    fn menu_text_escapes_accelerator_markers() {
        assert_eq!(String::from_utf16_lossy(&menu_text("A&B")[..4]), "A&&B");
    }

    #[test]
    fn tray_text_is_nul_terminated_and_truncated() {
        let mut buffer = [9_u16; 5];
        copy_wide(&mut buffer, "123456");

        assert_eq!(buffer, [49, 50, 51, 52, 0]);
    }
}
