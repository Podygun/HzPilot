<div align="center">
  <img src="assets/hzpilot.png" width="112" alt="Иконка HzPilot">
  <h1>HzPilot</h1>
  <p><strong>Миниатюрная нативная утилита для безопасного переключения частоты мониторов из трея Windows.</strong></p>
  <p>
    <a href="https://github.com/Podygun/HzPilot/actions/workflows/ci.yml"><img src="https://github.com/Podygun/HzPilot/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/Podygun/HzPilot?color=0e7490" alt="Лицензия MIT"></a>
    <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078d4" alt="Windows 10 и 11">
    <img src="https://img.shields.io/badge/Rust-1.85%2B-b7410e" alt="Rust 1.85 или новее">
  </p>
  <p>
    <a href="README.md">English</a> ·
    <a href="#быстрый-старт">Быстрый старт</a> ·
    <a href="docs/ARCHITECTURE.md">Архитектура</a> ·
    <a href="CONTRIBUTING.md">Участие</a>
  </p>
</div>

HzPilot работает в области уведомлений Windows и показывает частоты, уже поддерживаемые активными мониторами и графическим драйвером. Программа использует прямые вызовы Win32, занимает около 200 КБ и не требует прав администратора или отдельного runtime.

## Возможности

- Определяет все активные дисплеи, их понятные названия и точные текущие частоты.
- Показывает только режимы драйвера для текущего разрешения и семейства режима.
- Проверяет каждый выбор средствами Windows перед применением.
- Поддерживает расширенный рабочий стол и зеркальные группы дисплеев.
- Определяет состояние Dynamic Refresh Rate в Windows 11.
- Восстанавливает значок после перезапуска Проводника.
- Поддерживает необязательный автозапуск для текущего пользователя.
- Выбирает русский или английский интерфейс по языку Windows.
- Не использует телеметрию, сеть во время работы или фоновую службу.

## Кратко

| | |
| --- | --- |
| Платформа | Windows 10 1809+ и Windows 11 x64 |
| Runtime | Не нужен; нативные Rust и Win32 |
| Права | Текущий пользователь, без UAC |
| Установка | Portable ZIP или per-user installer Inno Setup |
| Лицензия | MIT |

## Быстрый старт

Публичный бинарный релиз пока не опубликован. Текущий MVP можно собрать из исходного кода:

```powershell
git clone https://github.com/Podygun/HzPilot.git
cd HzPilot
cargo build --release --locked
.\target\release\hzpilot.exe
```

У программы нет главного окна. Windows может сначала поместить значок под стрелку скрытой области трея; закрепить его рядом с часами может только пользователь.

| Действие | Результат |
| --- | --- |
| Левый клик | Мониторы и доступные частоты |
| Правый клик | Автозапуск, параметры Windows, информация и выход |
| Наведение | Активные мониторы и текущие частоты |

## Модель безопасности

HzPilot вызывает `EnumDisplaySettingsExW` без `EDS_RAWMODE`, оставляет режимы с текущими разрешением, глубиной цвета, ориентацией, fixed-output и scan-line ordering, затем проверяет выбранный `DEVMODEW` через `CDS_TEST`. Только после этого вызывается `ChangeDisplaySettingsExW`.

RAW-режимы и пользовательские значения не предлагаются. HzPilot намеренно не меняет разрешение, масштабирование, HDR, топологию или положение мониторов. Полный процесс описан в [документе об архитектуре](docs/ARCHITECTURE.md).

MVP управляет фиксированными частотами. Выбор числового значения при активном Dynamic Refresh Rate запрашивает фиксированную частоту. Remote Desktop и виртуальные GPU могут не предоставлять переключаемые режимы.

## Сборка и тестирование

Понадобятся Rust 1.85 или новее, Visual Studio Build Tools и Windows SDK. Для ресурсов используется `rc.exe`.

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

Реальное переключение дисплея проверяется вручную на физическом оборудовании. Перед релизом следуйте [инструкции по тестированию](docs/TESTING.md).

## Проект

- [Архитектура](docs/ARCHITECTURE.md)
- [Тестирование](docs/TESTING.md)
- [Процесс релиза](docs/RELEASING.md)
- [Участие в разработке](CONTRIBUTING.md)
- [Поддержка](SUPPORT.md)
- [Политика безопасности](SECURITY.md)
- [История изменений](CHANGELOG.md)

## Лицензия

HzPilot распространяется по [лицензии MIT](LICENSE).
