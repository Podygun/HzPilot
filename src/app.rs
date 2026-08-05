use std::cell::{Cell, RefCell};
use std::error::Error as StdError;
use std::ffi::c_void;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT,
    WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW,
    GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, IDI_APPLICATION, LoadIconW, MB_ICONERROR,
    MB_ICONINFORMATION, MB_OK, MSG, MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SW_SHOWNORMAL, SetWindowLongPtrW, TranslateMessage, WM_APP, WM_CLOSE,
    WM_CONTEXTMENU, WM_DESTROY, WM_DEVICECHANGE, WM_DISPLAYCHANGE, WM_LBUTTONUP, WM_NCCREATE,
    WM_NCDESTROY, WM_RBUTTONUP, WNDCLASSW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{Error, PCWSTR, Result as WindowsResult, w};

use crate::display::{Display, apply_refresh_rate, enumerate_displays, format_hz};
use crate::settings;
use crate::strings::Strings;
use crate::tray::{MenuAction, PopupMenu, TrayIcon};

const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;
const OPEN_MENU_MESSAGE: u32 = WM_APP + 2;
const REFRESH_DISPLAYS_MESSAGE: u32 = WM_APP + 3;
const NIN_SELECT: u32 = 0x400;
const NIN_KEYSELECT: u32 = 0x401;

type AppResult<T> = Result<T, Box<dyn StdError>>;

pub fn run() -> AppResult<()> {
    let instance_guard = SingleInstance::acquire()?;
    if instance_guard.already_exists {
        notify_running_instance();
        return Ok(());
    }

    // SAFETY: this runs before creating any application windows. Windows 10 versions
    // without PerMonitorV2 support may reject it, in which case system DPI behavior is used.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    // SAFETY: a null module name requests the current process module.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }?;
    let hinstance = HINSTANCE(module.0);
    // SAFETY: IDI_APPLICATION is a predefined shared icon.
    let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }?;
    // SAFETY: TaskbarCreated is a stable shell-defined broadcast message name.
    let taskbar_created_message = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    if taskbar_created_message == 0 {
        return Err(Error::from_thread().into());
    }

    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: hinstance,
        hIcon: icon,
        lpszClassName: w!("HzPilot.HiddenWindow"),
        ..Default::default()
    };
    // SAFETY: window_class and its static strings remain valid after registration.
    if unsafe { RegisterClassW(&window_class) } == 0 {
        return Err(Error::from_thread().into());
    }

    let mut state = Box::new(WindowState::new(App::new(
        taskbar_created_message,
        Strings::current(),
    )));
    let state_pointer = state.as_mut() as *mut WindowState;
    // SAFETY: state_pointer stays stable in its Box until the message loop exits.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("HzPilot.HiddenWindow"),
            w!("HzPilot"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            Some(state_pointer.cast::<c_void>()),
        )
    }?;

    let initialization = state.app.borrow_mut().initialize();
    if let Err(error) = initialization {
        // SAFETY: hwnd was created above and state remains alive during destruction callbacks.
        let _ = unsafe { DestroyWindow(hwnd) };
        return Err(error);
    }

    let result = message_loop();
    drop(state);
    drop(instance_guard);
    result
}

pub fn show_fatal_error(message: &str) {
    let message = nul_terminated(message);
    // SAFETY: both strings are NUL terminated and the message box is process-modal.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            w!("HzPilot"),
            MB_OK | MB_ICONERROR,
        );
    }
}

struct App {
    hwnd: HWND,
    tray: Option<TrayIcon>,
    displays: Vec<Display>,
    strings: &'static Strings,
    taskbar_created_message: u32,
}

impl App {
    fn new(taskbar_created_message: u32, strings: &'static Strings) -> Self {
        Self {
            hwnd: HWND::default(),
            tray: None,
            displays: Vec::new(),
            strings,
            taskbar_created_message,
        }
    }

    fn initialize(&mut self) -> AppResult<()> {
        self.displays = enumerate_displays()?;
        let mut tray = TrayIcon::new(self.hwnd)?;
        tray.add(TRAY_CALLBACK_MESSAGE)?;
        tray.set_tooltip(self.tooltip())?;
        self.tray = Some(tray);
        Ok(())
    }

    fn handle_message(&mut self, message: u32, _wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        if message == self.taskbar_created_message {
            if let Some(tray) = self.tray.as_mut() {
                if let Err(error) = tray.readd(TRAY_CALLBACK_MESSAGE) {
                    show_fatal_error(&error.to_string());
                }
            }
            return Some(LRESULT(0));
        }

        match message {
            TRAY_CALLBACK_MESSAGE => {
                let event = lparam.0 as u32 & 0xffff;
                match event {
                    WM_CONTEXTMENU | WM_RBUTTONUP => self.show_menu(true),
                    NIN_SELECT | NIN_KEYSELECT | WM_LBUTTONUP => self.show_menu(false),
                    _ => {}
                }
                Some(LRESULT(0))
            }
            OPEN_MENU_MESSAGE => {
                self.show_menu(false);
                Some(LRESULT(0))
            }
            WM_DISPLAYCHANGE | WM_DEVICECHANGE | REFRESH_DISPLAYS_MESSAGE => {
                if let Err(error) = self.refresh_displays() {
                    self.notify_error(&error.to_string());
                }
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                self.tray.take();
                // SAFETY: called from the UI thread to terminate its message loop.
                unsafe { PostQuitMessage(0) };
                Some(LRESULT(0))
            }
            _ => None,
        }
    }

    fn show_menu(&mut self, full: bool) {
        if let Err(error) = self.refresh_displays() {
            self.notify_error(&error.to_string());
        }

        let startup_enabled = if full {
            match settings::is_startup_enabled() {
                Ok(enabled) => enabled,
                Err(error) => {
                    self.notify_error(&error.to_string());
                    false
                }
            }
        } else {
            false
        };

        let selected = PopupMenu::build(&self.displays, self.strings, full, startup_enabled)
            .and_then(|menu| menu.show(self.hwnd));
        if let Some(tray) = self.tray.as_ref() {
            tray.focus();
        }

        match selected {
            Ok(Some(action)) => self.execute(action),
            Ok(None) => {}
            Err(error) => self.notify_error(&error.to_string()),
        }
    }

    fn execute(&mut self, action: MenuAction) {
        match action {
            MenuAction::SetRefresh { device_name, hz } => {
                let Some(display) = self
                    .displays
                    .iter()
                    .find(|display| display.device_name == device_name)
                    .cloned()
                else {
                    self.notify_error("The selected display is no longer active");
                    return;
                };

                if display
                    .modes
                    .iter()
                    .any(|mode| mode.hz == hz && display.is_current_mode(mode))
                {
                    return;
                }

                match apply_refresh_rate(&display, hz) {
                    Ok(()) => {
                        let message =
                            format!("{}: {} {}", display.friendly_name, hz, self.strings.unit);
                        if let Err(error) = self.refresh_displays() {
                            self.notify_error(&error.to_string());
                        }
                        if let Some(tray) = self.tray.as_ref() {
                            tray.notify_info(self.strings.success_title, &message);
                        }
                    }
                    Err(error) => self.notify_error(&error.to_string()),
                }
            }
            MenuAction::ToggleStartup => {
                let result = settings::is_startup_enabled()
                    .and_then(|enabled| settings::set_startup_enabled(!enabled));
                if let Err(error) = result {
                    self.notify_error(&error.to_string());
                }
            }
            MenuAction::OpenDisplaySettings => {
                if !open_display_settings(self.hwnd) {
                    self.notify_error("Windows could not open display settings");
                }
            }
            MenuAction::About => {
                let text = nul_terminated(&self.strings.about_text());
                // SAFETY: text is NUL terminated and hwnd belongs to this UI thread.
                unsafe {
                    MessageBoxW(
                        Some(self.hwnd),
                        PCWSTR(text.as_ptr()),
                        w!("HzPilot"),
                        MB_OK | MB_ICONINFORMATION,
                    );
                }
            }
            MenuAction::Exit => {
                // Posting WM_CLOSE avoids re-entering the window procedure while App is borrowed.
                if let Err(error) =
                    unsafe { PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }
                {
                    self.notify_error(&error.to_string());
                }
            }
        }
    }

    fn refresh_displays(&mut self) -> AppResult<()> {
        self.displays = enumerate_displays()?;
        let tooltip = self.tooltip();
        if let Some(tray) = self.tray.as_mut() {
            tray.set_tooltip(tooltip)?;
        }
        Ok(())
    }

    fn tooltip(&self) -> String {
        if self.displays.is_empty() {
            return format!("HzPilot - {}", self.strings.no_displays);
        }
        let displays = self
            .displays
            .iter()
            .map(|display| {
                format!(
                    "{}: {} {}",
                    display.friendly_name,
                    format_hz(display.current_hz()),
                    self.strings.unit
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        format!("HzPilot | {displays}")
    }

    fn notify_error(&self, message: &str) {
        if let Some(tray) = self.tray.as_ref() {
            tray.notify_error(self.strings.error_title, message);
        } else {
            show_fatal_error(message);
        }
    }
}

struct WindowState {
    app: RefCell<App>,
    refresh_pending: Cell<bool>,
}

impl WindowState {
    fn new(app: App) -> Self {
        Self {
            app: RefCell::new(app),
            refresh_pending: Cell::new(false),
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        // SAFETY: WM_NCCREATE lparam points to a CREATESTRUCTW for this call.
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let state = create.lpCreateParams.cast::<WindowState>();
        if state.is_null() {
            return LRESULT(0);
        }
        // SAFETY: state points to the boxed WindowState passed to CreateWindowExW.
        unsafe {
            (*state).app.borrow_mut().hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
        }
    }

    // SAFETY: GWLP_USERDATA is either zero or the WindowState pointer stored during WM_NCCREATE.
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
    let handled = if state.is_null() {
        None
    } else {
        // SAFETY: the boxed WindowState outlives the window and all dispatched messages.
        let state = unsafe { &*state };
        match state.app.try_borrow_mut() {
            Ok(mut app) => app.handle_message(message, wparam, lparam),
            Err(_) => {
                if matches!(message, WM_DISPLAYCHANGE | WM_DEVICECHANGE) {
                    state.refresh_pending.set(true);
                }
                None
            }
        }
    };

    if !state.is_null() {
        // SAFETY: state has the same lifetime described above.
        let state = unsafe { &*state };
        if state.refresh_pending.get() && state.app.try_borrow().is_ok() {
            state.refresh_pending.set(false);
            // SAFETY: this private message carries no borrowed data.
            let _ =
                unsafe { PostMessageW(Some(hwnd), REFRESH_DISPLAYS_MESSAGE, WPARAM(0), LPARAM(0)) };
        }
    }

    if let Some(result) = handled {
        return result;
    }

    if message == WM_NCDESTROY {
        // SAFETY: clearing user data prevents access after the window is gone.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    }
    // SAFETY: unhandled messages are delegated to the default window procedure.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn message_loop() -> AppResult<()> {
    let mut message = MSG::default();
    loop {
        // SAFETY: message points to writable storage and this is the UI thread's loop.
        let status = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        match status {
            -1 => return Err(Error::from_thread().into()),
            0 => return Ok(()),
            _ => {
                // SAFETY: message was populated by GetMessageW.
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }
}

fn open_display_settings(hwnd: HWND) -> bool {
    for uri in ["ms-settings:display-advanced", "ms-settings:display"] {
        let uri = nul_terminated(uri);
        // SAFETY: all strings are NUL terminated and ShellExecuteW does not retain them.
        let result = unsafe {
            ShellExecuteW(
                Some(hwnd),
                w!("open"),
                PCWSTR(uri.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize > 32 {
            return true;
        }
    }
    false
}

fn notify_running_instance() {
    // SAFETY: class name is static and a null title accepts any matching window.
    if let Ok(hwnd) = unsafe { FindWindowW(w!("HzPilot.HiddenWindow"), PCWSTR::null()) } {
        // SAFETY: posting a private application message does not retain borrowed data.
        let _ = unsafe { PostMessageW(Some(hwnd), OPEN_MENU_MESSAGE, WPARAM(0), LPARAM(0)) };
    }
}

struct SingleInstance {
    handle: HANDLE,
    already_exists: bool,
}

impl SingleInstance {
    fn acquire() -> WindowsResult<Self> {
        // SAFETY: the mutex uses default security and a static local-session name.
        let handle = unsafe { CreateMutexW(None, false, w!("Local\\HzPilot.SingleInstance")) }?;
        // SAFETY: CreateMutexW sets the calling thread's last-error value.
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        Ok(Self {
            handle,
            already_exists,
        })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the mutex handle returned by CreateMutexW.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

fn nul_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}
