use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::mem::size_of;

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_MODE_AWARE,
    QDC_VIRTUAL_REFRESH_RATE_AWARE, QUERY_DISPLAY_CONFIG_FLAGS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::Graphics::Gdi::{
    CDS_TEST, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE,
    DISP_CHANGE_BADDUALVIEW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE, DISP_CHANGE_BADPARAM,
    DISP_CHANGE_FAILED, DISP_CHANGE_NOTUPDATED, DISP_CHANGE_RESTART, DISP_CHANGE_SUCCESSFUL,
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER,
    DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW, DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT,
    DM_DISPLAYFLAGS, DM_DISPLAYFREQUENCY, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH,
    ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE,
    EnumDisplayDevicesW, EnumDisplaySettingsExW,
};
use windows::core::PCWSTR;

const DISPLAYCONFIG_PATH_BOOST_REFRESH_RATE: u32 = 0x10;
const DISPLAY_CONFIG_RETRIES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalRefreshRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl RationalRefreshRate {
    pub fn as_hz(self) -> Option<f64> {
        (self.denominator != 0).then(|| self.numerator as f64 / self.denominator as f64)
    }
}

#[derive(Clone)]
pub struct RefreshMode {
    pub hz: u32,
    dev_mode: DEVMODEW,
}

impl fmt::Debug for RefreshMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RefreshMode")
            .field("hz", &self.hz)
            .finish()
    }
}

#[derive(Clone)]
pub struct Display {
    pub device_name: String,
    pub friendly_name: String,
    pub is_primary: bool,
    pub dynamic_refresh: bool,
    pub precise_refresh: Option<RationalRefreshRate>,
    pub modes: Vec<RefreshMode>,
    current_mode: DEVMODEW,
}

impl Display {
    pub fn current_hz(&self) -> f64 {
        self.precise_refresh
            .and_then(RationalRefreshRate::as_hz)
            .unwrap_or(self.current_mode.dmDisplayFrequency as f64)
    }

    pub fn is_current_mode(&self, mode: &RefreshMode) -> bool {
        mode.hz == self.current_mode.dmDisplayFrequency
            || (mode.hz as f64 - self.current_hz()).abs() < 0.51
    }
}

#[derive(Debug)]
pub enum DisplayError {
    Win32 { operation: &'static str, code: u32 },
    ModeChange { operation: &'static str, code: i32 },
    NoMode { display: String, hz: u32 },
}

impl fmt::Display for DisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Win32 { operation, code } => {
                write!(formatter, "{operation} failed with Windows error {code}")
            }
            Self::ModeChange { operation, code } => {
                write!(
                    formatter,
                    "{operation} failed: {} ({code})",
                    mode_change_message(*code)
                )
            }
            Self::NoMode { display, hz } => {
                write!(formatter, "{display} no longer exposes a {hz} Hz mode")
            }
        }
    }
}

impl std::error::Error for DisplayError {}

#[derive(Clone, Debug)]
struct PathMetadata {
    friendly_name: String,
    refresh: RationalRefreshRate,
    dynamic_refresh: bool,
}

pub fn enumerate_displays() -> Result<Vec<Display>, DisplayError> {
    let path_metadata = query_path_metadata().unwrap_or_default();
    let mut displays = Vec::new();
    let mut index = 0;

    loop {
        let mut adapter = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        // SAFETY: adapter points to a correctly sized writable DISPLAY_DEVICEW.
        if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), index, &mut adapter, 0) }.as_bool() {
            break;
        }
        index += 1;

        if !adapter
            .StateFlags
            .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
            || adapter.StateFlags.contains(DISPLAY_DEVICE_MIRRORING_DRIVER)
        {
            continue;
        }

        let device_name = wide_array_to_string(&adapter.DeviceName);
        if device_name.is_empty() {
            continue;
        }

        let mut current_mode = initialized_dev_mode();
        let device_name_wide = nul_terminated(&device_name);
        // SAFETY: device_name_wide is NUL terminated and current_mode is correctly initialized.
        if !unsafe {
            EnumDisplaySettingsExW(
                PCWSTR(device_name_wide.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut current_mode,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
        }
        .as_bool()
        {
            continue;
        }

        let modes = enumerate_refresh_modes(&device_name_wide, &current_mode);
        if modes.is_empty() {
            continue;
        }

        let metadata = path_metadata.get(&device_name);
        let fallback_name = monitor_name_from_gdi(&device_name_wide)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| wide_array_to_string(&adapter.DeviceString));

        displays.push(Display {
            device_name,
            friendly_name: metadata
                .map(|item| item.friendly_name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or(fallback_name),
            is_primary: adapter.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
            dynamic_refresh: metadata.is_some_and(|item| item.dynamic_refresh),
            precise_refresh: metadata.map(|item| item.refresh),
            modes,
            current_mode,
        });
    }

    displays.sort_by(|left, right| {
        right
            .is_primary
            .cmp(&left.is_primary)
            .then_with(|| left.friendly_name.cmp(&right.friendly_name))
    });
    Ok(displays)
}

pub fn apply_refresh_rate(display: &Display, hz: u32) -> Result<(), DisplayError> {
    let selected = display
        .modes
        .iter()
        .find(|mode| mode.hz == hz)
        .ok_or_else(|| DisplayError::NoMode {
            display: display.friendly_name.clone(),
            hz,
        })?;

    let mut requested = selected.dev_mode;
    requested.dmFields =
        DM_BITSPERPEL | DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFLAGS | DM_DISPLAYFREQUENCY;
    if selected.dev_mode.dmFields.contains(DM_DISPLAYORIENTATION) {
        requested.dmFields |= DM_DISPLAYORIENTATION;
    }
    if selected.dev_mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT) {
        requested.dmFields |= DM_DISPLAYFIXEDOUTPUT;
    }

    let device_name = nul_terminated(&display.device_name);
    // SAFETY: requested came from EnumDisplaySettingsExW and the device name is NUL terminated.
    let test_result = unsafe {
        ChangeDisplaySettingsExW(
            PCWSTR(device_name.as_ptr()),
            Some(&requested),
            None,
            CDS_TEST,
            None,
        )
    };
    ensure_mode_change("Display mode validation", test_result)?;

    // SAFETY: the same driver-provided mode passed validation immediately above.
    let apply_result = unsafe {
        ChangeDisplaySettingsExW(
            PCWSTR(device_name.as_ptr()),
            Some(&requested),
            None,
            CDS_UPDATEREGISTRY,
            None,
        )
    };
    ensure_mode_change("Display mode change", apply_result)
}

pub fn format_hz(hz: f64) -> String {
    if (hz - hz.round()).abs() < 0.01 {
        format!("{:.0}", hz)
    } else {
        let value = format!("{hz:.2}");
        value.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

fn enumerate_refresh_modes(device_name: &[u16], current: &DEVMODEW) -> Vec<RefreshMode> {
    let mut modes = BTreeMap::new();
    let mut index = 0;

    loop {
        let mut candidate = initialized_dev_mode();
        // SAFETY: device_name is NUL terminated and candidate is correctly initialized.
        let found = unsafe {
            EnumDisplaySettingsExW(
                PCWSTR(device_name.as_ptr()),
                ENUM_DISPLAY_SETTINGS_MODE(index),
                &mut candidate,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
        }
        .as_bool();
        if !found {
            break;
        }
        index += 1;

        if candidate.dmDisplayFrequency > 1 && same_mode_family(current, &candidate) {
            modes
                .entry(candidate.dmDisplayFrequency)
                .or_insert(candidate);
        }
    }

    modes
        .into_iter()
        .rev()
        .map(|(hz, dev_mode)| RefreshMode { hz, dev_mode })
        .collect()
}

fn same_mode_family(current: &DEVMODEW, candidate: &DEVMODEW) -> bool {
    if current.dmPelsWidth != candidate.dmPelsWidth
        || current.dmPelsHeight != candidate.dmPelsHeight
        || current.dmBitsPerPel != candidate.dmBitsPerPel
    {
        return false;
    }

    // SAFETY: these union members are the display-mode variants populated by
    // EnumDisplaySettingsExW for both values.
    unsafe {
        current.Anonymous1.Anonymous2.dmDisplayOrientation
            == candidate.Anonymous1.Anonymous2.dmDisplayOrientation
            && current.Anonymous1.Anonymous2.dmDisplayFixedOutput
                == candidate.Anonymous1.Anonymous2.dmDisplayFixedOutput
            && current.Anonymous2.dmDisplayFlags == candidate.Anonymous2.dmDisplayFlags
    }
}

fn initialized_dev_mode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

fn monitor_name_from_gdi(device_name: &[u16]) -> Option<String> {
    let mut monitor = DISPLAY_DEVICEW {
        cb: size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    // SAFETY: device_name is NUL terminated and monitor is correctly sized.
    unsafe { EnumDisplayDevicesW(PCWSTR(device_name.as_ptr()), 0, &mut monitor, 0) }
        .as_bool()
        .then(|| wide_array_to_string(&monitor.DeviceString))
}

fn query_path_metadata() -> Result<HashMap<String, PathMetadata>, DisplayError> {
    let modern_flags =
        QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE | QDC_VIRTUAL_REFRESH_RATE_AWARE;
    match query_display_paths(modern_flags) {
        Err(DisplayError::Win32 { code, .. }) if code == ERROR_INVALID_PARAMETER.0 => {
            query_display_paths(QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE)
        }
        result => result,
    }
}

fn query_display_paths(
    flags: QUERY_DISPLAY_CONFIG_FLAGS,
) -> Result<HashMap<String, PathMetadata>, DisplayError> {
    for _ in 0..DISPLAY_CONFIG_RETRIES {
        let mut path_count = 0;
        let mut mode_count = 0;
        // SAFETY: both output pointers are valid for writes.
        let size_result =
            unsafe { GetDisplayConfigBufferSizes(flags, &mut path_count, &mut mode_count) };
        check_win32("GetDisplayConfigBufferSizes", size_result)?;

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        // SAFETY: vectors have the capacities reported by GetDisplayConfigBufferSizes.
        let query_result = unsafe {
            QueryDisplayConfig(
                flags,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if query_result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        check_win32("QueryDisplayConfig", query_result)?;
        paths.truncate(path_count as usize);

        let mut result: HashMap<String, PathMetadata> = HashMap::new();
        for path in paths {
            let Some(source_name) = source_name_for_path(&path) else {
                continue;
            };
            let friendly_name = target_name_for_path(&path).unwrap_or_default();
            let refresh = RationalRefreshRate {
                numerator: path.targetInfo.refreshRate.Numerator,
                denominator: path.targetInfo.refreshRate.Denominator,
            };
            let dynamic_refresh = path.flags & DISPLAYCONFIG_PATH_BOOST_REFRESH_RATE != 0;

            result
                .entry(source_name)
                .and_modify(|existing| {
                    if existing.friendly_name.is_empty() {
                        existing.friendly_name.clone_from(&friendly_name);
                    } else if !friendly_name.is_empty()
                        && !existing.friendly_name.contains(&friendly_name)
                    {
                        existing.friendly_name =
                            format!("{} + {}", existing.friendly_name, friendly_name);
                    }
                    existing.dynamic_refresh |= dynamic_refresh;
                })
                .or_insert(PathMetadata {
                    friendly_name,
                    refresh,
                    dynamic_refresh,
                });
        }
        return Ok(result);
    }

    Err(DisplayError::Win32 {
        operation: "QueryDisplayConfig",
        code: ERROR_INSUFFICIENT_BUFFER.0,
    })
}

fn source_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: path.sourceInfo.adapterId,
            id: path.sourceInfo.id,
        },
        ..Default::default()
    };
    // SAFETY: header is the first field and identifies the complete source structure.
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
    (result == ERROR_SUCCESS.0 as i32).then(|| wide_array_to_string(&source.viewGdiDeviceName))
}

fn target_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
            adapterId: path.targetInfo.adapterId,
            id: path.targetInfo.id,
        },
        ..Default::default()
    };
    // SAFETY: header is the first field and identifies the complete target structure.
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
    (result == ERROR_SUCCESS.0 as i32)
        .then(|| wide_array_to_string(&target.monitorFriendlyDeviceName))
}

fn check_win32(operation: &'static str, result: WIN32_ERROR) -> Result<(), DisplayError> {
    (result == ERROR_SUCCESS)
        .then_some(())
        .ok_or(DisplayError::Win32 {
            operation,
            code: result.0,
        })
}

fn ensure_mode_change(operation: &'static str, result: DISP_CHANGE) -> Result<(), DisplayError> {
    (result == DISP_CHANGE_SUCCESSFUL)
        .then_some(())
        .ok_or(DisplayError::ModeChange {
            operation,
            code: result.0,
        })
}

fn mode_change_message(code: i32) -> &'static str {
    match DISP_CHANGE(code) {
        DISP_CHANGE_BADDUALVIEW => "the system cannot change this multi-monitor mode",
        DISP_CHANGE_BADFLAGS => "invalid display flags",
        DISP_CHANGE_BADMODE => "the display mode is unsupported",
        DISP_CHANGE_BADPARAM => "invalid display parameters",
        DISP_CHANGE_FAILED => "the display driver rejected the change",
        DISP_CHANGE_NOTUPDATED => "the display registry could not be updated",
        DISP_CHANGE_RESTART => "Windows must restart to apply this mode",
        _ => "unknown display error",
    }
}

fn wide_array_to_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length]).trim().to_owned()
}

fn nul_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

#[cfg(test)]
mod tests {
    use super::{RationalRefreshRate, format_hz, wide_array_to_string};

    #[test]
    fn rational_refresh_rate_handles_fractional_modes() {
        let refresh = RationalRefreshRate {
            numerator: 59_940,
            denominator: 1_000,
        };

        assert_eq!(refresh.as_hz(), Some(59.94));
        assert_eq!(format_hz(refresh.as_hz().unwrap()), "59.94");
        assert_eq!(format_hz(144.0), "144");
    }

    #[test]
    fn rational_refresh_rate_rejects_zero_denominator() {
        let refresh = RationalRefreshRate {
            numerator: 60,
            denominator: 0,
        };

        assert_eq!(refresh.as_hz(), None);
    }

    #[test]
    fn wide_strings_stop_at_the_first_nul() {
        assert_eq!(wide_array_to_string(&[72, 122, 0, 88]), "Hz");
    }
}
