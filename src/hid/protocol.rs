use crate::model::{
    BatteryState, ButtonAction, MacroDef, MouseProfile, PollingRate, MAX_DPI_STAGES,
    TOTAL_BUTTON_SLOTS,
};

pub const ATLAS_VID: u16 = 0x1D57;
pub const ATLAS_PID_WIRED: u16 = 0x2112;
pub const ATLAS_PID_WIRELESS: u16 = 0xFA60;

pub const FEATURE_USAGE_PAGE: u16 = 0x000B;
pub const FEATURE_USAGE: u16 = 0x0000;

pub const MONITOR_USAGE_PAGE: u16 = 0x000A;
pub const MONITOR_USAGE: u16 = 0x0000;

/// Exact PAW3311 DPI register lookup table extracted from Press Play ATLAS Mouse.exe
/// (`0x0041D444 .. 0x0041D7B8`, 221 entries).
pub const DPI_LUT: [u8; 221] = [
    0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 21, 22, 23, 24, 25, 27, 28,
    29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 41, 42, 43, 44, 45, 47, 48, 49, 50, 51, 52, 54, 55,
    56, 57, 58, 59, 61, 62, 63, 64, 65, 67, 68, 69, 70, 71, 72, 74, 75, 76, 77, 78, 79, 81, 82,
    83, 84, 85, 87, 88, 89, 90, 91, 92, 94, 95, 96, 97, 98, 99, 101, 102, 103, 104, 105, 107, 108,
    109, 110, 111, 112, 114, 115, 116, 117, 118, 119, 121, 122, 123, 124, 125, 127, 128, 129, 130,
    131, 132, 134, 135, 136, 137, 138, 139, 141, 142, 143, 144, 145, 147, 148, 149, 150, 151, 152,
    154, 155, 156, 157, 158, 159, 161, 162, 163, 164, 165, 167, 168, 169, 170, 171, 172, 174, 175,
    176, 177, 178, 179, 181, 182, 183, 184, 185, 187, 188, 189, 190, 191, 192, 194, 195, 196, 197,
    198, 199, 201, 202, 203, 204, 205, 207, 208, 209, 210, 211, 212, 214, 215, 216, 217, 218, 219,
    221, 222, 223, 224, 225, 227, 228, 229, 230, 231, 232, 234, // 0..199
    235, // 200
    118, 119, 121, 122, 123, 124, 125, 127, 128, 129, 130, 131, 132, 134, 135, 136, 137, 138, 139,
    141, // 201..220
];

/// Fixed factory indicator bytes kept in the wire format for firmware compatibility.
const WIRE_DPI_PLACEHOLDER_COLORS: [[u8; 3]; 8] = [
    [255, 255, 0],
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [0, 255, 255],
    [255, 0, 255],
    [255, 64, 0],
    [255, 255, 255],
];

/// Encodes a DPI slider value (1..=340) into `(register_byte, high_flag_byte)`.
pub fn encode_dpi_register(slider: u32) -> (u8, u8) {
    let s = slider.clamp(1, 340) as usize;
    if s > 300 {
        let idx = 200 + (s - 300) / 2;
        (DPI_LUT[idx.min(220)], 1)
    } else if s > 220 {
        (DPI_LUT[s - 100], 0)
    } else if s > 200 {
        (DPI_LUT[s], 1)
    } else {
        (DPI_LUT[s], 0)
    }
}

/// Report ID 0x0C (10 bytes): Profile switch / commit handshake.
pub fn build_init_report() -> Vec<u8> {
    vec![0x0C, 0x0A, 0x01, 0xFE, 0x01, 0xFE, 0x00, 0x00, 0x00, 0x00]
}

/// Report ID 0x04 (56 bytes = 0x38): DPI stages and sensor settings (Angle Snap, Ripple Control).
pub fn build_dpi_sensor_report(profile: &MouseProfile) -> Vec<u8> {
    let mut buf = vec![0u8; 0x38];
    buf[0] = 0x04;
    buf[1] = 0x38;
    buf[2] = 0x01;
    buf[3] = u8::from(profile.angle_snap);
    buf[4] = u8::from(profile.ripple_control);

    let mut enable_mask = 0u8;
    let mut high_mask = 0u8;

    for i in 0..MAX_DPI_STAGES {
        let st = &profile.dpi_stages[i];
        if st.enabled {
            enable_mask |= 1 << i;
        }
        let slider = st.slider_value();
        if slider > 220 {
            high_mask |= 1 << i;
        }
        let (reg, flag) = encode_dpi_register(slider);
        buf[8 + i] = reg;
        buf[16 + i] = flag;

        let c_off = 25 + i * 3;
        buf[c_off] = WIRE_DPI_PLACEHOLDER_COLORS[i][0];
        buf[c_off + 1] = WIRE_DPI_PLACEHOLDER_COLORS[i][1];
        buf[c_off + 2] = WIRE_DPI_PLACEHOLDER_COLORS[i][2];
    }

    buf[5] = enable_mask;
    buf[6] = high_mask;
    buf[7] = high_mask;
    buf[24] = (profile.active_dpi_stage.min(7) as u8) + 1;
    buf[49] = 0x01;

    let sum: u16 = buf[3..50]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    buf[50] = (sum >> 8) as u8;
    buf[51] = (sum & 0xFF) as u8;
    buf
}

/// Report ID 0x05 (15 bytes = 0x0F): Sleep timers and Key Debounce (no RGB enabled).
pub fn build_power_debounce_report(profile: &MouseProfile) -> Vec<u8> {
    let mut buf = vec![0u8; 0x0F];
    let deep_sleep = profile.power.deep_sleep_minutes.clamp(1, 60);
    let sleep = profile.power.sleep_minutes.clamp(1, 60);
    let debounce_step = (profile.debounce_ms / 2).clamp(1, 25);

    buf[0] = 0x05;
    buf[1] = 0x0F;
    buf[2] = 0x01;
    buf[3] = 0x00; // LED Off
    buf[4] = (deep_sleep & 0xF0) | 0x03;
    buf[5] = ((deep_sleep & 0x0F) << 4) | 0x08;
    buf[6] = 0x00;
    buf[7] = 0x00;
    buf[8] = 0x00;
    buf[9] = sleep;
    buf[10] = debounce_step;

    let sum: u16 = buf[3..=10]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    buf[11] = (sum >> 8) as u8;
    buf[12] = (sum & 0xFF) as u8;
    buf
}

/// Report ID 0x06 (9 bytes = 0x09): Polling Rate.
pub fn build_polling_rate_report(rate: PollingRate) -> Vec<u8> {
    let code = rate.hid_code();
    vec![0x06, 0x09, 0x01, code, !code, 0x00, 0x00, 0x00, 0x00]
}

/// Report ID 0x08 (59 bytes = 0x3B): 18 button slots × 3 bytes + checksum.
pub fn build_buttons_report(profile: &MouseProfile) -> Vec<u8> {
    let mut buf = vec![0u8; 0x3B];
    buf[0] = 0x08;
    buf[1] = 0x3B;
    buf[2] = 0x01;

    for slot in 0..TOTAL_BUTTON_SLOTS {
        let bytes = profile.buttons[slot].to_hid_bytes(slot);
        let off = 3 + slot * 3;
        buf[off] = bytes[0];
        buf[off + 1] = bytes[1];
        buf[off + 2] = bytes[2];
    }

    let sum: u16 = buf[3..57]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    buf[57] = (sum >> 8) as u8;
    buf[58] = (sum & 0xFF) as u8;
    buf
}

/// Report ID 0x09 (split into 3 × 64-byte feature packets): Macro upload for a specific 1-based button slot.
pub fn build_macro_reports(
    slot_0based: usize,
    macro_def: &MacroDef,
    is_wireless: bool,
) -> Vec<Vec<u8>> {
    let slot_1based = (slot_0based + 1) as u8;
    let mut raw = vec![0u8; 0x83]; // 131 bytes
    raw[0] = 0x09;
    raw[1] = 0x83;
    raw[2] = slot_1based;
    raw[3] = macro_def.loop_mode as u8;
    raw[4] = 0x01;
    raw[5] = 0x01;
    raw[6] = 0x01;
    raw[7] = macro_def.loop_count.max(1);

    let max_events = 50usize;
    let mut cursor = 29usize;
    let mut written_pairs = 0u8;

    for ev in macro_def.events.iter().take(max_events) {
        if cursor + 1 >= 129 {
            break;
        }
        let delay_clamped = ev.delay_ms.clamp(1, 1270);
        let ticks = (((delay_clamped as f32) / 10.0).round() as u8).clamp(1, 63);
        let action_flag = if ev.pressed { 0x01u8 } else { 0x81u8 };
        let header_byte = ((ticks & 0x3F) << 1) | action_flag;

        raw[cursor] = header_byte;
        raw[cursor + 1] = ev.hid_key;
        cursor += 2;
        written_pairs += 1;
    }

    raw[28] = written_pairs;

    let sum: u16 = raw[3..129]
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    raw[129] = (sum >> 8) as u8;
    raw[130] = (sum & 0xFF) as u8;

    let mut packets = Vec::with_capacity(3);
    for chunk in 0..3usize {
        let mut pkt = vec![0u8; 64];
        pkt[0] = 0x09;
        pkt[1] = if is_wireless && chunk == 2 { 0x0C } else { 0x40 };
        pkt[2] = slot_1based;
        pkt[3] = chunk as u8;
        let src_start = 3 + chunk * 60;
        let src_end = (src_start + 60).min(raw.len());
        if src_start < raw.len() {
            let len = src_end - src_start;
            pkt[4..4 + len].copy_from_slice(&raw[src_start..src_end]);
        }
        packets.push(pkt);
    }
    packets
}

pub fn build_full_sync_packets(
    profile: &MouseProfile,
    macros: &[MacroDef],
    is_wireless: bool,
) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    out.push(("Init Sync (0x0C)".into(), build_init_report()));
    out.push(("DPI & Sensor (0x04)".into(), build_dpi_sensor_report(profile)));
    out.push((
        "Power & Debounce (0x05)".into(),
        build_power_debounce_report(profile),
    ));
    out.push((
        format!("Polling Rate {}Hz (0x06)", profile.polling_rate.hz()),
        build_polling_rate_report(profile.polling_rate),
    ));
    out.push(("Button Map (0x08)".into(), build_buttons_report(profile)));

    for (slot, action) in profile.buttons.iter().enumerate() {
        if let ButtonAction::Macro { macro_id } = action {
            if let Some(m) = macros.iter().find(|m| &m.id == macro_id) {
                for (ci, pkt) in build_macro_reports(slot, m, is_wireless)
                    .into_iter()
                    .enumerate()
                {
                    out.push((format!("Macro Slot {} Chunk {} (0x09)", slot + 1, ci), pkt));
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorEvent {
    ActiveDpiStage(usize),
    PollingRateChanged(PollingRate),
    Battery(BatteryState),
    BatteryBar(u8),
    Ack(bool),
}

pub fn parse_monitor_report(buf: &[u8]) -> Option<MonitorEvent> {
    if buf.len() < 5 || buf[0] != 0x03 || buf[1] != 0xB3 {
        return None;
    }
    let event_type = buf[2];
    let lo = buf[3];
    let hi = buf[4];

    match event_type {
        0x10 => {
            if (1..=8).contains(&lo) {
                Some(MonitorEvent::ActiveDpiStage((lo - 1) as usize))
            } else {
                None
            }
        }
        0x20 => PollingRate::from_monitor_idx(lo).map(MonitorEvent::PollingRateChanged),
        0x40 => {
            let state = match lo {
                2 => BatteryState::Full,
                3 => BatteryState::Charging(hi.min(100)),
                _ => {
                    if hi <= 100 {
                        BatteryState::Discharging(hi)
                    } else {
                        BatteryState::Unknown
                    }
                }
            };
            Some(MonitorEvent::Battery(state))
        }
        0x50 => Some(MonitorEvent::Ack(lo == 0)),
        0x60 => {
            if (1..=10).contains(&lo) {
                Some(MonitorEvent::BatteryBar(lo))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn hex_preview(bytes: &[u8], max: usize) -> String {
    let shown = bytes.len().min(max);
    let mut s = bytes[..shown]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > max {
        s.push_str(&format!(" ... ({}B)", bytes.len()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{dpi_to_slider, slider_to_dpi};

    #[test]
    fn test_dpi_slider_roundtrip() {
        assert_eq!(slider_to_dpi(1), 50);
        assert_eq!(slider_to_dpi(8), 400);
        assert_eq!(slider_to_dpi(16), 800);
        assert_eq!(slider_to_dpi(32), 1600);
        assert_eq!(slider_to_dpi(200), 10000);
        assert_eq!(slider_to_dpi(220), 12000);
        assert_eq!(slider_to_dpi(340), 24000);
        assert_eq!(dpi_to_slider(800), 16);
        assert_eq!(dpi_to_slider(12000), 220);
        assert_eq!(dpi_to_slider(24000), 340);
    }

    #[test]
    fn test_polling_rate_packets() {
        assert_eq!(
            build_polling_rate_report(PollingRate::Hz1000)[..5],
            [0x06, 0x09, 0x01, 0x01, 0xFE]
        );
        assert_eq!(
            build_polling_rate_report(PollingRate::Hz500)[..5],
            [0x06, 0x09, 0x01, 0x02, 0xFD]
        );
        assert_eq!(
            build_polling_rate_report(PollingRate::Hz250)[..5],
            [0x06, 0x09, 0x01, 0x04, 0xFB]
        );
        assert_eq!(
            build_polling_rate_report(PollingRate::Hz125)[..5],
            [0x06, 0x09, 0x01, 0x08, 0xF7]
        );
    }

    #[test]
    fn test_dpi_and_buttons_checksums() {
        let profile = MouseProfile::factory_default("Test");
        let dpi_pkt = build_dpi_sensor_report(&profile);
        assert_eq!(dpi_pkt.len(), 0x38);
        assert_eq!(dpi_pkt[0], 0x04);
        assert_eq!(dpi_pkt[5], 0b0011_1111);
        assert_eq!(dpi_pkt[24], 2);

        let btn_pkt = build_buttons_report(&profile);
        assert_eq!(btn_pkt.len(), 0x3B);
        assert_eq!(btn_pkt[3..6], [0x02, 0x00, 0x00]);
        assert_eq!(btn_pkt[6..9], [0x03, 0x00, 0x00]);
        assert_eq!(btn_pkt[9..12], [0x04, 0x00, 0x00]);
    }

    #[test]
    fn test_monitor_report_parsing() {
        assert_eq!(
            parse_monitor_report(&[0x03, 0xB3, 0x40, 0x00, 85]),
            Some(MonitorEvent::Battery(BatteryState::Discharging(85)))
        );
        assert_eq!(
            parse_monitor_report(&[0x03, 0xB3, 0x40, 0x02, 100]),
            Some(MonitorEvent::Battery(BatteryState::Full))
        );
        assert_eq!(
            parse_monitor_report(&[0x03, 0xB3, 0x10, 0x03, 0x00]),
            Some(MonitorEvent::ActiveDpiStage(2))
        );
    }

    #[test]
    fn test_power_debounce_and_macro_reports() {
        let profile = MouseProfile::factory_default("PowerTest");
        let pwr = build_power_debounce_report(&profile);
        assert_eq!(pwr.len(), 0x0F);
        assert_eq!(pwr[0], 0x05);
        assert_eq!(pwr[3], 0x00); // LED off
        assert_eq!(pwr[9], profile.power.sleep_minutes);
        assert_eq!(pwr[10], profile.debounce_ms / 2);
        let expected_sum: u16 = pwr[3..=10].iter().map(|&b| b as u16).sum();
        assert_eq!(u16::from_be_bytes([pwr[11], pwr[12]]), expected_sum);

        let mut empty_macro = MacroDef::new("m0".into(), "Empty".into());
        empty_macro.events.clear();
        let empty_pkts = build_macro_reports(4, &empty_macro, false);
        assert_eq!(empty_pkts.len(), 3);
        assert_eq!(empty_pkts[0][1], 0x40);
        assert_eq!(empty_pkts[2][1], 0x40);

        let mut full_macro = MacroDef::new("m50".into(), "Full".into());
        full_macro.events = (0..60)
            .map(|i| crate::model::MacroEvent {
                hid_key: 0x04,
                pressed: i % 2 == 0,
                delay_ms: 20,
            })
            .collect();
        let wl_pkts = build_macro_reports(5, &full_macro, true);
        assert_eq!(wl_pkts.len(), 3);
        assert_eq!(wl_pkts[0][2], 6); // 1-based slot
        assert_eq!(wl_pkts[2][1], 0x0C); // Wireless final chunk length
    }
}
