use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::hid::protocol::{
    build_buttons_report, build_dpi_sensor_report, build_full_sync_packets,
    build_polling_rate_report, build_power_debounce_report, hex_preview, parse_monitor_report,
    MonitorEvent,
};
use crate::hid::win32::AtlasHidDevice;
use crate::model::{ConnectionMode, MacroDef, MouseProfile, PollingRate};

#[derive(Debug, Clone)]
pub enum HidCommand {
    Reconnect,
    SyncFull {
        profile: MouseProfile,
        macros: Vec<MacroDef>,
    },
    SyncDpiAndSensor(MouseProfile),
    SyncPowerDebounce(MouseProfile),
    SyncPollingRate(PollingRate),
    SyncButtons {
        profile: MouseProfile,
        macros: Vec<MacroDef>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum HidEvent {
    ConnectionChanged {
        mode: ConnectionMode,
        vid: u16,
        pid: u16,
    },
    Monitor(MonitorEvent),
    SyncStarted,
    SyncFinished {
        ok: bool,
        message: String,
    },
    Log(String),
}

pub struct HidManager {
    tx: Sender<HidCommand>,
    rx: Receiver<HidEvent>,
}

impl HidManager {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<HidCommand>();
        let (ev_tx, ev_rx) = mpsc::channel::<HidEvent>();

        thread::Builder::new()
            .name("atlas-hid-worker".into())
            .spawn(move || worker_loop(cmd_rx, ev_tx))
            .expect("failed to spawn HID worker thread");

        Self { tx: cmd_tx, rx: ev_rx }
    }

    pub fn send(&self, cmd: HidCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn poll_events(&self) -> Vec<HidEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            events.push(ev);
        }
        events
    }
}

impl Drop for HidManager {
    fn drop(&mut self) {
        let _ = self.tx.send(HidCommand::Shutdown);
    }
}

fn worker_loop(cmd_rx: Receiver<HidCommand>, ev_tx: Sender<HidEvent>) {
    let mut device: Option<AtlasHidDevice> = None;
    let mut last_probe = Instant::now() - Duration::from_secs(10);

    loop {
        if device.is_none() && last_probe.elapsed() >= Duration::from_millis(1500) {
            last_probe = Instant::now();
            if let Some(dev) = AtlasHidDevice::open_any() {
                let _ = ev_tx.send(HidEvent::Log(format!(
                    "Connected {} (VID {:04X} PID {:04X})",
                    dev.mode.label(),
                    dev.vid,
                    dev.pid
                )));
                let _ = ev_tx.send(HidEvent::ConnectionChanged {
                    mode: dev.mode,
                    vid: dev.vid,
                    pid: dev.pid,
                });
                device = Some(dev);
            }
        }

        let mut latest_cmd: Option<HidCommand> = None;
        let wait_timeout = if device.is_some() {
            Duration::from_millis(20)
        } else {
            Duration::from_millis(120)
        };

        match cmd_rx.recv_timeout(wait_timeout) {
            Ok(cmd) => {
                if matches!(cmd, HidCommand::Shutdown) {
                    return;
                }
                latest_cmd = Some(cmd);
                while let Ok(next) = cmd_rx.try_recv() {
                    if matches!(next, HidCommand::Shutdown) {
                        return;
                    }
                    latest_cmd = Some(next);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Some(cmd) = latest_cmd {
            match cmd {
                HidCommand::Reconnect => {
                    device = None;
                    last_probe = Instant::now() - Duration::from_secs(10);
                    let _ = ev_tx.send(HidEvent::ConnectionChanged {
                        mode: ConnectionMode::Disconnected,
                        vid: 0,
                        pid: 0,
                    });
                    let _ = ev_tx.send(HidEvent::Log("Scanning USB HID bus...".into()));
                }
                HidCommand::Shutdown => return,
                other => {
                    if let Some(ref dev) = device {
                        let is_wireless = dev.mode == ConnectionMode::Wireless24G;
                        let packets = build_packets_for_cmd(&other, is_wireless);
                        let _ = ev_tx.send(HidEvent::SyncStarted);

                        let mut all_ok = true;
                        let mut err_msg = String::new();

                        for (label, pkt) in packets {
                            let _ = ev_tx.send(HidEvent::Log(format!(
                                "TX {label}: {}",
                                hex_preview(&pkt, 14)
                            )));
                            if let Err(e) = dev.set_feature(&pkt) {
                                all_ok = false;
                                err_msg = e.clone();
                                let _ = ev_tx.send(HidEvent::Log(format!("ERR {label}: {e}")));
                                break;
                            }
                            let pace = if is_wireless { 35 } else { 12 };
                            thread::sleep(Duration::from_millis(pace));
                        }

                        if all_ok {
                            let _ = ev_tx.send(HidEvent::SyncFinished {
                                ok: true,
                                message: "Synced to ATLAS mouse".into(),
                            });
                        } else {
                            let _ = ev_tx.send(HidEvent::SyncFinished {
                                ok: false,
                                message: err_msg,
                            });
                            device = None;
                            let _ = ev_tx.send(HidEvent::ConnectionChanged {
                                mode: ConnectionMode::Disconnected,
                                vid: 0,
                                pid: 0,
                            });
                        }
                    } else {
                        let _ = ev_tx.send(HidEvent::SyncFinished {
                            ok: false,
                            message: "Saved locally (mouse offline)".into(),
                        });
                    }
                }
            }
        }

        if let Some(ref dev) = device {
            match dev.read_monitor(40) {
                Ok(Some(raw)) => {
                    if let Some(mon) = parse_monitor_report(&raw) {
                        let _ = ev_tx.send(HidEvent::Log(format!(
                            "RX Monitor: {} -> {:?}",
                            hex_preview(&raw, 8),
                            mon
                        )));
                        let _ = ev_tx.send(HidEvent::Monitor(mon));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = ev_tx.send(HidEvent::Log(format!("Device disconnected ({e})")));
                    device = None;
                    let _ = ev_tx.send(HidEvent::ConnectionChanged {
                        mode: ConnectionMode::Disconnected,
                        vid: 0,
                        pid: 0,
                    });
                }
            }
        }
    }
}

fn build_packets_for_cmd(cmd: &HidCommand, is_wireless: bool) -> Vec<(String, Vec<u8>)> {
    match cmd {
        HidCommand::SyncFull { profile, macros } => {
            build_full_sync_packets(profile, macros, is_wireless)
        }
        HidCommand::SyncDpiAndSensor(profile) => {
            vec![("DPI & Sensor (0x04)".into(), build_dpi_sensor_report(profile))]
        }
        HidCommand::SyncPowerDebounce(profile) => {
            vec![(
                "Power & Debounce (0x05)".into(),
                build_power_debounce_report(profile),
            )]
        }
        HidCommand::SyncPollingRate(rate) => {
            vec![(
                format!("Polling Rate {}Hz (0x06)", rate.hz()),
                build_polling_rate_report(*rate),
            )]
        }
        HidCommand::SyncButtons { profile, macros } => {
            let mut out = vec![("Button Map (0x08)".into(), build_buttons_report(profile))];
            for (slot, action) in profile.buttons.iter().enumerate() {
                if let crate::model::ButtonAction::Macro { macro_id } = action {
                    if let Some(m) = macros.iter().find(|m| &m.id == macro_id) {
                        for (ci, pkt) in
                            crate::hid::protocol::build_macro_reports(slot, m, is_wireless)
                                .into_iter()
                                .enumerate()
                        {
                            out.push((
                                format!("Macro Slot {} Chunk {} (0x09)", slot + 1, ci),
                                pkt,
                            ));
                        }
                    }
                }
            }
            out
        }
        HidCommand::Reconnect | HidCommand::Shutdown => Vec::new(),
    }
}
