use serde::{Deserialize, Serialize};

pub const MAX_DPI_STAGES: usize = 8;
pub const TOTAL_BUTTON_SLOTS: usize = 18;

/// Physical programmable buttons exposed on the Press Play ATLAS mouse.
/// Maps each physical button on the mouse diagram to its firmware slot index (0..17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalButtonInfo {
    pub id: usize,          // Display number 1..=6
    pub slot: usize,        // Firmware slot index in Report 0x08
    pub short_name: &'static str,
    pub full_name: &'static str,
}

pub const PHYSICAL_BUTTONS: [PhysicalButtonInfo; 6] = [
    PhysicalButtonInfo { id: 1, slot: 0, short_name: "LMB", full_name: "Klik Kiri" },
    PhysicalButtonInfo { id: 2, slot: 1, short_name: "RMB", full_name: "Klik Kanan" },
    PhysicalButtonInfo { id: 3, slot: 2, short_name: "MMB", full_name: "Klik Tengah (Scroll)" },
    PhysicalButtonInfo { id: 4, slot: 5, short_name: "M4",  full_name: "Samping Maju" },
    PhysicalButtonInfo { id: 5, slot: 4, short_name: "M5",  full_name: "Samping Mundur" },
    PhysicalButtonInfo { id: 6, slot: 3, short_name: "DPI", full_name: "Tombol DPI Bawah" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PollingRate {
    Hz125,
    Hz250,
    Hz500,
    Hz1000,
}

impl PollingRate {
    pub const ALL: [PollingRate; 4] = [
        PollingRate::Hz125,
        PollingRate::Hz250,
        PollingRate::Hz500,
        PollingRate::Hz1000,
    ];

    pub fn hz(self) -> u16 {
        match self {
            PollingRate::Hz125 => 125,
            PollingRate::Hz250 => 250,
            PollingRate::Hz500 => 500,
            PollingRate::Hz1000 => 1000,
        }
    }

    pub fn latency_ms(self) -> f32 {
        1000.0 / (self.hz() as f32)
    }

    pub fn tier_label(self) -> &'static str {
        match self {
            PollingRate::Hz125 => "Hemat Baterai",
            PollingRate::Hz250 => "Harian / Kerja",
            PollingRate::Hz500 => "Gaming Stabil",
            PollingRate::Hz1000 => "Esports (Tercepat)",
        }
    }

    pub fn hid_code(self) -> u8 {
        match self {
            PollingRate::Hz125 => 0x08,
            PollingRate::Hz250 => 0x04,
            PollingRate::Hz500 => 0x02,
            PollingRate::Hz1000 => 0x01,
        }
    }

    pub fn from_monitor_idx(idx: u8) -> Option<Self> {
        match idx {
            0 | 8 => Some(PollingRate::Hz125),
            1 | 4 => Some(PollingRate::Hz250),
            2 => Some(PollingRate::Hz500),
            3 => Some(PollingRate::Hz1000),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiftOffDistance {
    Mm1,
    Mm2,
}

impl LiftOffDistance {
    pub fn label(self) -> &'static str {
        match self {
            LiftOffDistance::Mm1 => "1.0 mm",
            LiftOffDistance::Mm2 => "2.0 mm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiStage {
    pub dpi: u32,     // 50..=24000
    pub enabled: bool,
}

impl DpiStage {
    pub fn slider_value(&self) -> u32 {
        dpi_to_slider(self.dpi)
    }
}

/// Convert firmware slider step (1..=340) to actual DPI (50..=24000).
pub fn slider_to_dpi(slider: u32) -> u32 {
    let s = slider.clamp(1, 340);
    if s <= 200 {
        s * 50
    } else {
        (s - 100) * 100
    }
}

/// Convert actual DPI (50..=24000) to firmware slider step (1..=340).
pub fn dpi_to_slider(dpi: u32) -> u32 {
    let d = dpi.clamp(50, 24000);
    if d <= 10000 {
        ((d + 25) / 50).clamp(1, 200)
    } else {
        ((d + 50) / 100 + 100).clamp(201, 340)
    }
}

/// Snap any DPI input to the nearest hardware-supported PAW3311 step.
pub fn snap_dpi(dpi: u32) -> u32 {
    slider_to_dpi(dpi_to_slider(dpi))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSettings {
    pub sleep_minutes: u8,      // 1..=60
    pub deep_sleep_minutes: u8, // 1..=60
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            sleep_minutes: 3,
            deep_sleep_minutes: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonCategory {
    Mouse,
    Dpi,
    Multimedia,
    Browser,
    Shortcuts,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonAction {
    Disabled,
    LeftClick,
    RightClick,
    MiddleClick,
    Forward,
    Backward,
    DoubleClick,
    FireButton,
    EasyAim,
    ScrollUp,
    ScrollDown,
    TiltLeft,
    TiltRight,
    DpiCycle,
    DpiUp,
    DpiDown,
    ProfileCycle,
    // Multimedia
    MediaPlayer,
    PlayPause,
    MediaStop,
    PrevTrack,
    NextTrack,
    VolumeUp,
    VolumeDown,
    Mute,
    // Browser / System
    BrowserHome,
    BrowserFavorites,
    BrowserForward,
    BrowserBackward,
    BrowserStop,
    BrowserRefresh,
    BrowserSearch,
    Email,
    Calculator,
    MyComputer,
    // Common Shortcuts
    Cut,
    Copy,
    Paste,
    Open,
    Save,
    Find,
    Redo,
    Undo,
    SelectAll,
    Print,
    CloseWindow,
    SwapWindows,
    ShowDesktop,
    RunCommand,
    LockPc,
    ScreenCapture,
    // Custom Shortcut: modifier bitmask (1=Ctrl, 2=Shift, 4=Alt, 8=Win) + USB HID usage keycode
    Shortcut { modifiers: u8, hid_key: u8 },
    // Macro by ID
    Macro { macro_id: String },
}

impl ButtonAction {
    /// Returns the exact 3-byte firmware payload `[cmd, p1, p2]` for Report 0x08.
    pub fn to_hid_bytes(&self, slot_index: usize) -> [u8; 3] {
        match self {
            ButtonAction::Disabled => [0x00, 0x00, 0x00],
            ButtonAction::LeftClick => [0x02, 0x00, 0x00],
            ButtonAction::RightClick => [0x03, 0x00, 0x00],
            ButtonAction::MiddleClick => [0x04, 0x00, 0x00],
            ButtonAction::Backward => [0x05, 0x00, 0x00],
            ButtonAction::Forward => [0x06, 0x00, 0x00],
            ButtonAction::ScrollUp => [0x09, 0x00, 0x00],
            ButtonAction::ScrollDown => [0x0A, 0x00, 0x00],
            ButtonAction::TiltRight => [0x0B, 0x00, 0x00],
            ButtonAction::TiltLeft => [0x0C, 0x00, 0x00],
            ButtonAction::DpiCycle => [0x0D, 0x00, 0x00],
            ButtonAction::DpiUp => [0x0E, 0x00, 0x00],
            ButtonAction::DpiDown => [0x0F, 0x00, 0x00],
            ButtonAction::DoubleClick => [0x13, 0x00, 0x00],
            ButtonAction::EasyAim => [0x14, 0x00, 0x00],
            ButtonAction::FireButton => [0x2B, 0x00, 0x00],
            ButtonAction::ProfileCycle => [0x29, 0x00, 0x03],
            // Multimedia
            ButtonAction::MediaPlayer => [0x15, 0x00, 0x00],
            ButtonAction::PlayPause => [0x16, 0x00, 0x00],
            ButtonAction::MediaStop => [0x17, 0x00, 0x00],
            ButtonAction::PrevTrack => [0x18, 0x00, 0x00],
            ButtonAction::NextTrack => [0x19, 0x00, 0x00],
            ButtonAction::Mute => [0x1A, 0x00, 0x00],
            ButtonAction::VolumeUp => [0x1B, 0x00, 0x00],
            ButtonAction::VolumeDown => [0x1C, 0x00, 0x00],
            // Browser / System
            ButtonAction::BrowserSearch => [0x1D, 0x00, 0x00],
            ButtonAction::BrowserRefresh => [0x1E, 0x00, 0x00],
            ButtonAction::Email => [0x11, 0x03, 0x12],
            ButtonAction::Calculator => [0x20, 0x00, 0x00],
            ButtonAction::BrowserHome => [0x21, 0x00, 0x00],
            ButtonAction::BrowserBackward => [0x22, 0x00, 0x00],
            ButtonAction::BrowserFavorites => [0x23, 0x00, 0x00],
            ButtonAction::BrowserForward => [0x24, 0x00, 0x00],
            ButtonAction::MyComputer => [0x25, 0x00, 0x00],
            ButtonAction::BrowserStop => [0x26, 0x00, 0x00],
            // Shortcuts
            ButtonAction::Cut => [0x11, 0x01, 0x1B],
            ButtonAction::Copy => [0x11, 0x01, 0x06],
            ButtonAction::Paste => [0x11, 0x01, 0x19],
            ButtonAction::Open => [0x11, 0x01, 0x12],
            ButtonAction::Save => [0x11, 0x01, 0x16],
            ButtonAction::Print => [0x11, 0x01, 0x13],
            ButtonAction::Find => [0x11, 0x01, 0x09],
            ButtonAction::Redo => [0x11, 0x01, 0x1C],
            ButtonAction::Undo => [0x11, 0x01, 0x1D],
            ButtonAction::SelectAll => [0x11, 0x01, 0x04],
            ButtonAction::SwapWindows => [0x11, 0x04, 0x2B],
            ButtonAction::ShowDesktop => [0x11, 0x08, 0x07],
            ButtonAction::CloseWindow => [0x11, 0x04, 0x3D],
            ButtonAction::RunCommand => [0x11, 0x08, 0x15],
            ButtonAction::LockPc => [0x11, 0x08, 0x0F],
            ButtonAction::ScreenCapture => [0x11, 0x0A, 0x16],
            ButtonAction::Shortcut { modifiers, hid_key } => [0x11, *modifiers, *hid_key],
            ButtonAction::Macro { .. } => [0x12, 0x00, (slot_index + 1) as u8],
        }
    }

    pub fn display_label(&self, macros: &[MacroDef]) -> String {
        match self {
            ButtonAction::Disabled => "Nonaktif".into(),
            ButtonAction::LeftClick => "Klik Kiri".into(),
            ButtonAction::RightClick => "Klik Kanan".into(),
            ButtonAction::MiddleClick => "Klik Tengah".into(),
            ButtonAction::Forward => "Maju (Forward)".into(),
            ButtonAction::Backward => "Mundur (Back)".into(),
            ButtonAction::DoubleClick => "Klik Ganda".into(),
            ButtonAction::FireButton => "Tembak Cepat (Fire)".into(),
            ButtonAction::EasyAim => "Sniper / Easy Aim".into(),
            ButtonAction::ScrollUp => "Scroll Atas".into(),
            ButtonAction::ScrollDown => "Scroll Bawah".into(),
            ButtonAction::TiltLeft => "Geser Kiri".into(),
            ButtonAction::TiltRight => "Geser Kanan".into(),
            ButtonAction::DpiCycle => "Ganti DPI (Cycle)".into(),
            ButtonAction::DpiUp => "DPI Naik (+)".into(),
            ButtonAction::DpiDown => "DPI Turun (-)".into(),
            ButtonAction::ProfileCycle => "Ganti Profil".into(),
            ButtonAction::MediaPlayer => "Pemutar Media".into(),
            ButtonAction::PlayPause => "Play / Pause".into(),
            ButtonAction::MediaStop => "Stop Media".into(),
            ButtonAction::PrevTrack => "Lagu Sebelumnya".into(),
            ButtonAction::NextTrack => "Lagu Berikutnya".into(),
            ButtonAction::VolumeUp => "Volume Naik (+)".into(),
            ButtonAction::VolumeDown => "Volume Turun (-)".into(),
            ButtonAction::Mute => "Bisukan Suara (Mute)".into(),
            ButtonAction::BrowserHome => "Beranda Browser".into(),
            ButtonAction::BrowserFavorites => "Bookmark".into(),
            ButtonAction::BrowserForward => "Browser Maju".into(),
            ButtonAction::BrowserBackward => "Browser Kembali".into(),
            ButtonAction::BrowserStop => "Stop Muat".into(),
            ButtonAction::BrowserRefresh => "Muat Ulang (Refresh)".into(),
            ButtonAction::BrowserSearch => "Cari di Browser".into(),
            ButtonAction::Email => "Buka Email".into(),
            ButtonAction::Calculator => "Kalkulator".into(),
            ButtonAction::MyComputer => "File Explorer".into(),
            ButtonAction::Cut => "Potong (Ctrl+X)".into(),
            ButtonAction::Copy => "Salin (Ctrl+C)".into(),
            ButtonAction::Paste => "Tempel (Ctrl+V)".into(),
            ButtonAction::Open => "Buka (Ctrl+O)".into(),
            ButtonAction::Save => "Simpan (Ctrl+S)".into(),
            ButtonAction::Find => "Cari (Ctrl+F)".into(),
            ButtonAction::Redo => "Redo (Ctrl+Y)".into(),
            ButtonAction::Undo => "Undo (Ctrl+Z)".into(),
            ButtonAction::SelectAll => "Pilih Semua (Ctrl+A)".into(),
            ButtonAction::Print => "Cetak (Ctrl+P)".into(),
            ButtonAction::CloseWindow => "Tutup (Alt+F4)".into(),
            ButtonAction::SwapWindows => "Pindah App (Alt+Tab)".into(),
            ButtonAction::ShowDesktop => "Desktop (Win+D)".into(),
            ButtonAction::RunCommand => "Run (Win+R)".into(),
            ButtonAction::LockPc => "Kunci PC (Win+L)".into(),
            ButtonAction::ScreenCapture => "Screenshot (Win+Shift+S)".into(),
            ButtonAction::Shortcut { modifiers, hid_key } => {
                format_shortcut(*modifiers, *hid_key)
            }
            ButtonAction::Macro { macro_id } => {
                let name = macros
                    .iter()
                    .find(|m| &m.id == macro_id)
                    .map(|m| m.name.as_str())
                    .unwrap_or("Tidak Ada");
                format!("Makro: {name}")
            }
        }
    }
}

pub fn format_shortcut(modifiers: u8, hid_key: u8) -> String {
    let mut parts = Vec::new();
    if modifiers & 0x01 != 0 { parts.push("Ctrl"); }
    if modifiers & 0x02 != 0 { parts.push("Shift"); }
    if modifiers & 0x04 != 0 { parts.push("Alt"); }
    if modifiers & 0x08 != 0 { parts.push("Win"); }
    parts.push(hid_keycode_name(hid_key));
    parts.join(" + ")
}

pub fn hid_keycode_name(code: u8) -> &'static str {
    match code {
        0x04 => "A", 0x05 => "B", 0x06 => "C", 0x07 => "D",
        0x08 => "E", 0x09 => "F", 0x0A => "G", 0x0B => "H",
        0x0C => "I", 0x0D => "J", 0x0E => "K", 0x0F => "L",
        0x10 => "M", 0x11 => "N", 0x12 => "O", 0x13 => "P",
        0x14 => "Q", 0x15 => "R", 0x16 => "S", 0x17 => "T",
        0x18 => "U", 0x19 => "V", 0x1A => "W", 0x1B => "X",
        0x1C => "Y", 0x1D => "Z",
        0x1E => "1", 0x1F => "2", 0x20 => "3", 0x21 => "4",
        0x22 => "5", 0x23 => "6", 0x24 => "7", 0x25 => "8",
        0x26 => "9", 0x27 => "0",
        0x28 => "Enter", 0x29 => "Esc", 0x2A => "Backspace",
        0x2B => "Tab", 0x2C => "Space",
        0x2D => "-", 0x2E => "=", 0x2F => "[", 0x30 => "]",
        0x31 => "\\", 0x33 => ";", 0x34 => "'", 0x35 => "`",
        0x36 => ",", 0x37 => ".", 0x38 => "/", 0x39 => "CapsLock",
        0x3A => "F1", 0x3B => "F2", 0x3C => "F3", 0x3D => "F4",
        0x3E => "F5", 0x3F => "F6", 0x40 => "F7", 0x41 => "F8",
        0x42 => "F9", 0x43 => "F10", 0x44 => "F11", 0x45 => "F12",
        0x46 => "PrtSc", 0x47 => "ScrLk", 0x48 => "Pause",
        0x49 => "Insert", 0x4A => "Home", 0x4B => "PgUp",
        0x4C => "Delete", 0x4D => "End", 0x4E => "PgDn",
        0x4F => "Right", 0x50 => "Left", 0x51 => "Down", 0x52 => "Up",
        0xE0 => "LCtrl", 0xE1 => "LShift", 0xE2 => "LAlt", 0xE3 => "LWin",
        0xE4 => "RCtrl", 0xE5 => "RShift", 0xE6 => "RAlt", 0xE7 => "RWin",
        0xF0 => "Mouse Left", 0xF1 => "Mouse Right", 0xF2 => "Mouse Middle",
        0xF3 => "Mouse Forward", 0xF4 => "Mouse Back",
        _ => "Key",
    }
}

pub const SHORTCUT_KEY_CHOICES: &[(u8, &str)] = &[
    (0x04, "A"), (0x05, "B"), (0x06, "C"), (0x07, "D"), (0x08, "E"), (0x09, "F"),
    (0x0A, "G"), (0x0B, "H"), (0x0C, "I"), (0x0D, "J"), (0x0E, "K"), (0x0F, "L"),
    (0x10, "M"), (0x11, "N"), (0x12, "O"), (0x13, "P"), (0x14, "Q"), (0x15, "R"),
    (0x16, "S"), (0x17, "T"), (0x18, "U"), (0x19, "V"), (0x1A, "W"), (0x1B, "X"),
    (0x1C, "Y"), (0x1D, "Z"),
    (0x1E, "1"), (0x1F, "2"), (0x20, "3"), (0x21, "4"), (0x22, "5"),
    (0x23, "6"), (0x24, "7"), (0x25, "8"), (0x26, "9"), (0x27, "0"),
    (0x28, "Enter"), (0x29, "Esc"), (0x2A, "Backspace"), (0x2B, "Tab"), (0x2C, "Space"),
    (0x3A, "F1"), (0x3B, "F2"), (0x3C, "F3"), (0x3D, "F4"), (0x3E, "F5"), (0x3F, "F6"),
    (0x40, "F7"), (0x41, "F8"), (0x42, "F9"), (0x43, "F10"), (0x44, "F11"), (0x45, "F12"),
    (0x49, "Insert"), (0x4A, "Home"), (0x4B, "PgUp"), (0x4C, "Delete"), (0x4D, "End"), (0x4E, "PgDn"),
    (0x52, "Up"), (0x51, "Down"), (0x50, "Left"), (0x4F, "Right"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroLoopMode {
    RepeatCount = 0,
    UntilAnyKey = 1,
    WhileHeld = 2,
    Toggle = 3,
}

impl MacroLoopMode {
    pub const ALL: [MacroLoopMode; 4] = [
        MacroLoopMode::RepeatCount,
        MacroLoopMode::UntilAnyKey,
        MacroLoopMode::WhileHeld,
        MacroLoopMode::Toggle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MacroLoopMode::RepeatCount => "Ulangi Sebanyak N Kali",
            MacroLoopMode::UntilAnyKey => "Berhenti Jika Tekan Tombol Lain",
            MacroLoopMode::WhileHeld => "Jalan Selama Tombol Ditahan",
            MacroLoopMode::Toggle => "Tekan Lagi Untuk Berhenti",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroEvent {
    pub hid_key: u8,
    pub pressed: bool,
    pub delay_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: String,
    pub name: String,
    pub loop_mode: MacroLoopMode,
    pub loop_count: u8,
    pub events: Vec<MacroEvent>,
}

impl MacroDef {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            loop_mode: MacroLoopMode::RepeatCount,
            loop_count: 1,
            events: vec![
                MacroEvent { hid_key: 0x04, pressed: true, delay_ms: 20 },
                MacroEvent { hid_key: 0x04, pressed: false, delay_ms: 20 },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseProfile {
    pub name: String,
    pub dpi_stages: [DpiStage; MAX_DPI_STAGES],
    pub active_dpi_stage: usize, // 0..7
    pub polling_rate: PollingRate,
    pub lod: LiftOffDistance,
    pub debounce_ms: u8, // 2..=50 (even numbers; firmware stores step = debounce_ms / 2)
    pub ripple_control: bool,
    pub angle_snap: bool,
    #[serde(default)]
    pub power: PowerSettings,
    pub buttons: [ButtonAction; TOTAL_BUTTON_SLOTS],
}

impl MouseProfile {
    pub fn factory_default(name: impl Into<String>) -> Self {
        let mut buttons = std::array::from_fn(|_| ButtonAction::Disabled);
        buttons[0] = ButtonAction::LeftClick;
        buttons[1] = ButtonAction::RightClick;
        buttons[2] = ButtonAction::MiddleClick;
        buttons[3] = ButtonAction::DpiCycle;
        buttons[4] = ButtonAction::Backward;
        buttons[5] = ButtonAction::Forward;
        buttons[16] = ButtonAction::ScrollDown;
        buttons[17] = ButtonAction::ScrollUp;

        Self {
            name: name.into(),
            dpi_stages: [
                DpiStage { dpi: 400,   enabled: true },
                DpiStage { dpi: 800,   enabled: true },
                DpiStage { dpi: 1600,  enabled: true },
                DpiStage { dpi: 3200,  enabled: true },
                DpiStage { dpi: 6400,  enabled: true },
                DpiStage { dpi: 12000, enabled: true },
                DpiStage { dpi: 16000, enabled: false },
                DpiStage { dpi: 24000, enabled: false },
            ],
            active_dpi_stage: 1, // 800 DPI default
            polling_rate: PollingRate::Hz1000,
            lod: LiftOffDistance::Mm1,
            debounce_ms: 8,
            ripple_control: true,
            angle_snap: false,
            power: PowerSettings::default(),
            buttons,
        }
    }

    pub fn active_dpi(&self) -> u32 {
        self.dpi_stages
            .get(self.active_dpi_stage)
            .map(|s| s.dpi)
            .unwrap_or(800)
    }

    pub fn count_left_clicks(&self) -> usize {
        PHYSICAL_BUTTONS
            .iter()
            .filter(|b| self.buttons[b.slot] == ButtonAction::LeftClick)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccentTheme {
    CyberCyan,
    AtlasCrimson,
    EsportsLime,
    ObsidianGold,
    IceSilver,
}

impl AccentTheme {
    pub const ALL: [AccentTheme; 5] = [
        AccentTheme::CyberCyan,
        AccentTheme::AtlasCrimson,
        AccentTheme::EsportsLime,
        AccentTheme::ObsidianGold,
        AccentTheme::IceSilver,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AccentTheme::CyberCyan => "Arctic Cyan",
            AccentTheme::AtlasCrimson => "ATLAS Crimson",
            AccentTheme::EsportsLime => "Volt Mint",
            AccentTheme::ObsidianGold => "Titanium Gold",
            AccentTheme::IceSilver => "Stealth Silver",
        }
    }

    pub fn rgb(self) -> [u8; 3] {
        match self {
            AccentTheme::CyberCyan => [56, 214, 255],
            AccentTheme::AtlasCrimson => [255, 62, 92],
            AccentTheme::EsportsLime => [62, 232, 160],
            AccentTheme::ObsidianGold => [242, 192, 74],
            AccentTheme::IceSilver => [216, 224, 240],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_profile_idx: usize,
    pub profiles: Vec<MouseProfile>,
    pub macros: Vec<MacroDef>,
    pub accent: AccentTheme,
    pub auto_sync: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut fps = MouseProfile::factory_default("Competitive FPS");
        fps.active_dpi_stage = 1;
        fps.debounce_ms = 4;
        fps.polling_rate = PollingRate::Hz1000;
        fps.ripple_control = false;
        fps.angle_snap = false;

        let mut desktop = MouseProfile::factory_default("Productivity");
        desktop.active_dpi_stage = 2;
        desktop.polling_rate = PollingRate::Hz500;
        desktop.buttons[5] = ButtonAction::Copy;
        desktop.buttons[4] = ButtonAction::Paste;

        Self {
            active_profile_idx: 0,
            profiles: vec![
                MouseProfile::factory_default("ATLAS Default"),
                fps,
                desktop,
            ],
            macros: vec![MacroDef::new("macro_1".into(), "Quick Tap".into())],
            accent: AccentTheme::CyberCyan,
            auto_sync: true,
        }
    }
}

impl AppConfig {
    pub fn active_profile(&self) -> &MouseProfile {
        let idx = self.active_profile_idx.min(self.profiles.len().saturating_sub(1));
        &self.profiles[idx]
    }

    pub fn active_profile_mut(&mut self) -> &mut MouseProfile {
        if self.profiles.is_empty() {
            self.profiles.push(MouseProfile::factory_default("ATLAS Default"));
        }
        let idx = self.active_profile_idx.min(self.profiles.len() - 1);
        self.active_profile_idx = idx;
        &mut self.profiles[idx]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Disconnected,
    Wired,
    Wireless24G,
}

impl ConnectionMode {
    pub fn label(self) -> &'static str {
        match self {
            ConnectionMode::Disconnected => "Mode Offline (Mouse Belum Terdeteksi)",
            ConnectionMode::Wired => "Kabel USB-C",
            ConnectionMode::Wireless24G => "Wireless 2.4GHz",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    Unknown,
    Discharging(u8),
    Charging(u8),
    Full,
}

impl BatteryState {
    pub fn percent(self) -> Option<u8> {
        match self {
            BatteryState::Unknown => None,
            BatteryState::Discharging(p) | BatteryState::Charging(p) => Some(p.min(100)),
            BatteryState::Full => Some(100),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub mode: ConnectionMode,
    pub vid: u16,
    pub pid: u16,
    pub battery: BatteryState,
    pub sleeping: bool,
    pub last_sync_ok: Option<bool>,
    pub sync_in_flight: bool,
    pub packet_log: Vec<String>,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            mode: ConnectionMode::Disconnected,
            vid: 0,
            pid: 0,
            battery: BatteryState::Unknown,
            sleeping: false,
            last_sync_ok: None,
            sync_in_flight: false,
            packet_log: Vec::new(),
        }
    }
}
