use std::fmt;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::slice;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, RegCloseKey,
    RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::w;

#[derive(Debug)]
pub struct SettingsError {
    operation: &'static str,
    code: u32,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} failed with Windows error {}",
            self.operation, self.code
        )
    }
}

impl std::error::Error for SettingsError {}

pub fn is_startup_enabled() -> Result<bool, SettingsError> {
    let key = RegistryKey::open(KEY_QUERY_VALUE)?;
    let result = unsafe {
        // SAFETY: key is valid and only the existence and type are queried.
        RegQueryValueExW(key.0, w!("HzPilot"), None, None, None, None)
    };

    if result == ERROR_SUCCESS {
        Ok(true)
    } else if result == ERROR_FILE_NOT_FOUND {
        Ok(false)
    } else {
        Err(error("RegQueryValueExW", result))
    }
}

pub fn set_startup_enabled(enabled: bool) -> Result<(), SettingsError> {
    let key = RegistryKey::open(KEY_SET_VALUE)?;
    if enabled {
        let executable = std::env::current_exe().map_err(|_| SettingsError {
            operation: "current_exe",
            code: 0,
        })?;
        let command = quoted_path(&executable);
        // SAFETY: u16 has no padding and the byte slice covers command's initialized memory.
        let bytes = unsafe {
            slice::from_raw_parts(
                command.as_ptr().cast::<u8>(),
                command.len() * size_of::<u16>(),
            )
        };
        // SAFETY: key is writable and bytes contain a NUL-terminated UTF-16 REG_SZ value.
        let result = unsafe { RegSetValueExW(key.0, w!("HzPilot"), None, REG_SZ, Some(bytes)) };
        check("RegSetValueExW", result)
    } else {
        // SAFETY: key is writable and the value name is NUL terminated.
        let result = unsafe { RegDeleteValueW(key.0, w!("HzPilot")) };
        if result == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check("RegDeleteValueW", result)
        }
    }
}

fn quoted_path(path: &std::path::Path) -> Vec<u16> {
    std::iter::once('"' as u16)
        .chain(path.as_os_str().encode_wide())
        .chain(['"' as u16, 0])
        .collect()
}

struct RegistryKey(HKEY);

impl RegistryKey {
    fn open(access: REG_SAM_FLAGS) -> Result<Self, SettingsError> {
        let mut key = HKEY::default();
        // SAFETY: key points to writable storage for the opened handle.
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
                None,
                access,
                &mut key,
            )
        };
        check("RegOpenKeyExW", result)?;
        Ok(Self(key))
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the key returned by RegOpenKeyExW.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

fn check(operation: &'static str, result: WIN32_ERROR) -> Result<(), SettingsError> {
    (result == ERROR_SUCCESS)
        .then_some(())
        .ok_or_else(|| error(operation, result))
}

fn error(operation: &'static str, result: WIN32_ERROR) -> SettingsError {
    SettingsError {
        operation,
        code: result.0,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::quoted_path;

    #[test]
    fn startup_path_is_quoted_and_nul_terminated() {
        let value = quoted_path(Path::new(r"C:\Program Files\HzPilot\hzpilot.exe"));
        let decoded = String::from_utf16_lossy(&value[..value.len() - 1]);

        assert_eq!(decoded, r#""C:\Program Files\HzPilot\hzpilot.exe""#);
        assert_eq!(value.last(), Some(&0));
    }
}
