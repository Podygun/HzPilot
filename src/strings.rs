use windows::Win32::Globalization::GetUserDefaultUILanguage;

const RUSSIAN_LANGUAGE_ID: u16 = 0x19;
const PRIMARY_LANGUAGE_MASK: u16 = 0x03ff;

pub struct Strings {
    pub no_displays: &'static str,
    pub primary: &'static str,
    pub dynamic: &'static str,
    pub unit: &'static str,
    pub startup: &'static str,
    pub display_settings: &'static str,
    pub about: &'static str,
    pub exit: &'static str,
    pub error_title: &'static str,
    pub success_title: &'static str,
    pub about_description: &'static str,
    pub about_privacy: &'static str,
}

impl Strings {
    pub fn current() -> &'static Self {
        // SAFETY: GetUserDefaultUILanguage has no preconditions.
        let language = unsafe { GetUserDefaultUILanguage() } & PRIMARY_LANGUAGE_MASK;
        if language == RUSSIAN_LANGUAGE_ID {
            &RUSSIAN
        } else {
            &ENGLISH
        }
    }

    pub fn about_text(&self) -> String {
        format!(
            "HzPilot {}\n\n{}\n\n{}",
            env!("CARGO_PKG_VERSION"),
            self.about_description,
            self.about_privacy
        )
    }
}

static ENGLISH: Strings = Strings {
    no_displays: "No active displays",
    primary: "primary",
    dynamic: "dynamic",
    unit: "Hz",
    startup: "Start with Windows",
    display_settings: "Windows display settings",
    about: "About HzPilot",
    exit: "Exit",
    error_title: "HzPilot error",
    success_title: "Refresh rate changed",
    about_description: "A lightweight utility for switching monitor refresh rates.",
    about_privacy: "No telemetry, network access, background service, or administrator rights.",
};

static RUSSIAN: Strings = Strings {
    no_displays: "Нет активных мониторов",
    primary: "основной",
    dynamic: "динамическая",
    unit: "Гц",
    startup: "Запускать вместе с Windows",
    display_settings: "Параметры дисплея Windows",
    about: "О программе HzPilot",
    exit: "Выход",
    error_title: "Ошибка HzPilot",
    success_title: "Частота обновления изменена",
    about_description: "Лёгкая утилита для переключения частоты обновления мониторов.",
    about_privacy: "Без телеметрии, сети, фоновой службы и прав администратора.",
};
