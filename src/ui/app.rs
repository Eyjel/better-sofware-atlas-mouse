use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, ColorImage, CornerRadius, FontId, Frame, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
};

use crate::hid::manager::{HidCommand, HidEvent, HidManager};
use crate::hid::protocol::MonitorEvent;
use crate::model::{AppConfig, BatteryState, ConnectionMode, DeviceStatus};
use crate::storage;
use crate::ui::theme::{
    accent_button, apply_luxury_theme, badge_pill, install_modern_fonts, Palette,
};
use crate::ui::views::{
    draw_buttons_view, draw_macros_view, draw_performance_view, draw_power_profiles_view,
    SyncHint, UiTransientState,
};

/// Immutable compile-time watermark signature baked into the binary (not configurable).
const WATERMARK_SIGNATURE: &str = "Eyjel";
const _: () = {
    let b = WATERMARK_SIGNATURE.as_bytes();
    assert!(b.len() == 5 && b[0] == b'E' && b[1] == b'y' && b[2] == b'j' && b[3] == b'e' && b[4] == b'l');
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NavTab {
    Buttons = 0,
    Performance = 1,
    Macros = 2,
    PowerProfiles = 3,
}

impl NavTab {
    const ALL: [NavTab; 4] = [
        NavTab::Buttons,
        NavTab::Performance,
        NavTab::Macros,
        NavTab::PowerProfiles,
    ];

    fn label(self) -> &'static str {
        match self {
            NavTab::Buttons => "Tombol",
            NavTab::Performance => "Sensor & DPI",
            NavTab::Macros => "Makro",
            NavTab::PowerProfiles => "Daya & Profil",
        }
    }
}

pub struct AtlasApp {
    cfg: AppConfig,
    hid: HidManager,
    status: DeviceStatus,
    tab: NavTab,
    state: UiTransientState,
    dirty_config: bool,
    pending_sync: SyncHint,
    last_edit_at: Option<Instant>,
    last_theme: Option<crate::model::AccentTheme>,
    logo_tex: Option<TextureHandle>,
}

impl AtlasApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_modern_fonts(&cc.egui_ctx);
        let cfg = storage::load_config();
        let palette = Palette::from_accent(cfg.accent);
        apply_luxury_theme(&cc.egui_ctx, palette);

        // Load official Press Play ATLAS icon as crisp GPU texture for the app header
        let logo_tex = crate::decode_ico_rgba(crate::ATLAS_ICO_BYTES, 64).map(|(rgba, w, h)| {
            let img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            cc.egui_ctx
                .load_texture("atlas_brand_logo", img, TextureOptions::LINEAR)
        });

        let hid = HidManager::spawn();
        let last_theme = Some(cfg.accent);

        Self {
            cfg,
            hid,
            status: DeviceStatus::default(),
            tab: NavTab::Buttons,
            state: UiTransientState::default(),
            dirty_config: false,
            pending_sync: SyncHint::None,
            last_edit_at: None,
            last_theme,
            logo_tex,
        }
    }

    fn process_hid_events(&mut self) {
        for ev in self.hid.poll_events() {
            match ev {
                HidEvent::ConnectionChanged { mode, vid, pid } => {
                    let was_off = self.status.mode == ConnectionMode::Disconnected;
                    self.status.mode = mode;
                    self.status.vid = vid;
                    self.status.pid = pid;
                    if mode == ConnectionMode::Disconnected {
                        self.status.battery = BatteryState::Unknown;
                    } else if was_off {
                        self.state.show_toast(
                            format!("Press Play ATLAS tersambung ({})", mode.label()),
                            false,
                        );
                    }
                }
                HidEvent::Monitor(mon) => {
                    self.status.sleeping = false;
                    match mon {
                        MonitorEvent::ActiveDpiStage(stage) => {
                            if stage < 8 {
                                self.cfg.active_profile_mut().active_dpi_stage = stage;
                                let dpi = self.cfg.active_profile().active_dpi();
                                self.state.show_toast(
                                    format!("DPI Level {} · {dpi} DPI", stage + 1),
                                    false,
                                );
                            }
                        }
                        MonitorEvent::PollingRateChanged(rate) => {
                            self.cfg.active_profile_mut().polling_rate = rate;
                        }
                        MonitorEvent::Battery(b) => {
                            self.status.battery = b;
                        }
                        MonitorEvent::BatteryBar(bars) => {
                            if matches!(self.status.battery, BatteryState::Unknown) {
                                self.status.battery =
                                    BatteryState::Discharging((bars * 10).min(100));
                            }
                        }
                        MonitorEvent::Ack(_) => {}
                    }
                }
                HidEvent::SyncStarted => {
                    self.status.sync_in_flight = true;
                }
                HidEvent::SyncFinished { ok, message } => {
                    self.status.sync_in_flight = false;
                    self.status.last_sync_ok = Some(ok);
                    if !ok && self.status.mode != ConnectionMode::Disconnected {
                        self.state.show_toast(message, true);
                    }
                }
                HidEvent::Log(line) => {
                    self.status.packet_log.push(line);
                    if self.status.packet_log.len() > 80 {
                        let overflow = self.status.packet_log.len() - 80;
                        self.status.packet_log.drain(0..overflow);
                    }
                }
            }
        }
    }

    fn dispatch_sync(&mut self, hint: SyncHint) {
        if !hint.needs_hardware_sync() {
            return;
        }
        let prof = self.cfg.active_profile().clone();
        let macros = self.cfg.macros.clone();
        match hint {
            SyncHint::None | SyncHint::SaveOnly => {}
            SyncHint::DpiSensor => self.hid.send(HidCommand::SyncDpiAndSensor(prof)),
            SyncHint::PowerDebounce => self.hid.send(HidCommand::SyncPowerDebounce(prof)),
            SyncHint::PollingRate => self.hid.send(HidCommand::SyncPollingRate(prof.polling_rate)),
            SyncHint::Buttons => self.hid.send(HidCommand::SyncButtons {
                profile: prof,
                macros,
            }),
            SyncHint::Full => self.hid.send(HidCommand::SyncFull {
                profile: prof,
                macros,
            }),
        }
    }

    fn flush_debounced_updates(&mut self) {
        if let Some(t) = self.last_edit_at {
            if t.elapsed() >= Duration::from_millis(120) {
                self.last_edit_at = None;
                if self.dirty_config {
                    let _ = storage::save_config(&self.cfg);
                    self.dirty_config = false;
                }
                if self.pending_sync != SyncHint::None {
                    let h = self.pending_sync;
                    self.pending_sync = SyncHint::None;
                    if self.cfg.auto_sync && h.needs_hardware_sync() {
                        self.dispatch_sync(h);
                    }
                }
            }
        }
    }
}

impl eframe::App for AtlasApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_hid_events();
        self.flush_debounced_updates();

        ctx.request_repaint_after(Duration::from_millis(120));

        if self.last_theme != Some(self.cfg.accent) {
            let p = Palette::from_accent(self.cfg.accent);
            apply_luxury_theme(ctx, p);
            self.last_theme = Some(self.cfg.accent);
        }
        let p = Palette::from_accent(self.cfg.accent);

        // Header Modern dengan Logo Asli Press Play ATLAS + Navigasi Pill Geser
        egui::TopBottomPanel::top("top_header")
            .exact_height(66.0)
            .frame(
                Frame::new()
                    .fill(p.bg_topbar)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .inner_margin(Margin::symmetric(20, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Icon Asli Press Play ATLAS
                    if let Some(ref tex) = self.logo_tex {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(Vec2::splat(32.0))
                                .corner_radius(CornerRadius::same(7)),
                        );
                    } else {
                        let (mark_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                        ui.painter()
                            .rect_filled(mark_rect, CornerRadius::same(8), p.accent_soft);
                    }

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = -1.0;
                        ui.label(
                            RichText::new("Press Play ATLAS")
                                .size(15.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        let (conn_col, conn_txt) = match self.status.mode {
                            ConnectionMode::Wired => (p.success, "● Kabel USB-C"),
                            ConnectionMode::Wireless24G => (p.accent, "● Wireless 2.4GHz"),
                            ConnectionMode::Disconnected => (p.warning, "○ Mode Offline"),
                        };
                        ui.label(RichText::new(conn_txt).size(10.2).strong().color(conn_col));
                    });

                    if let Some(pct) = self.status.battery.percent() {
                        ui.add_space(6.0);
                        let chg = matches!(self.status.battery, BatteryState::Charging(_));
                        let bcol = if chg {
                            p.accent
                        } else if pct <= 20 {
                            p.danger
                        } else {
                            p.success
                        };
                        let btxt = if chg {
                            format!("⚡ {pct}%")
                        } else {
                            format!("{pct}%")
                        };
                        badge_pill(
                            ui,
                            Color32::from_rgba_unmultiplied(bcol.r(), bcol.g(), bcol.b(), 24),
                            bcol,
                            bcol,
                            &btxt,
                        );
                    }

                    ui.add_space(16.0);

                    // Navigasi Pill Melayang dengan Animasi Geser Halus
                    let nav_count = NavTab::ALL.len();
                    let tab_w = 118.0_f32;
                    let total_nav_w = tab_w * (nav_count as f32) + 8.0;
                    let (nav_rect, _) =
                        ui.allocate_exact_size(Vec2::new(total_nav_w, 40.0), Sense::hover());

                    ui.painter().rect(
                        nav_rect,
                        CornerRadius::same(13),
                        p.bg_input,
                        Stroke::new(1.0_f32, p.border_subtle),
                        StrokeKind::Inside,
                    );

                    let target_slot = self.tab as usize as f32;
                    let anim_slot = ctx.animate_value_with_time(
                        egui::Id::new("nav_pill_slide"),
                        target_slot,
                        0.18,
                    );
                    let pill_x = nav_rect.left() + 4.0 + anim_slot * tab_w;
                    let pill_rect = Rect::from_min_size(
                        Pos2::new(pill_x, nav_rect.top() + 4.0),
                        Vec2::new(tab_w, nav_rect.height() - 8.0),
                    );
                    ui.painter().rect(
                        pill_rect,
                        CornerRadius::same(9),
                        p.accent_soft,
                        Stroke::new(1.2_f32, p.accent),
                        StrokeKind::Inside,
                    );

                    for (idx, nav) in NavTab::ALL.into_iter().enumerate() {
                        let cell = Rect::from_min_size(
                            Pos2::new(
                                nav_rect.left() + 4.0 + (idx as f32) * tab_w,
                                nav_rect.top() + 4.0,
                            ),
                            Vec2::new(tab_w, nav_rect.height() - 8.0),
                        );
                        let resp =
                            ui.interact(cell, egui::Id::new(("nav_tab", idx)), Sense::click());
                        let is_sel = self.tab == nav;
                        let col = if is_sel {
                            p.text_primary
                        } else if resp.hovered() {
                            p.accent
                        } else {
                            p.text_secondary
                        };
                        ui.painter().text(
                            cell.center(),
                            Align2::CENTER_CENTER,
                            nav.label(),
                            FontId::proportional(12.8),
                            col,
                        );
                        if resp.clicked() {
                            self.tab = nav;
                            if nav != NavTab::Macros {
                                self.state.recording_macro = false;
                            }
                        }
                    }

                    // Kanan: Pilih Profil Cepat + Tombol Sinkronkan
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let sync_lbl = if self.status.sync_in_flight {
                            "Mengirim..."
                        } else {
                            "Sinkronkan"
                        };
                        if accent_button(ui, p, sync_lbl).clicked() {
                            let _ = storage::save_config(&self.cfg);
                            self.dispatch_sync(SyncHint::Full);
                            self.state.show_toast(
                                if self.status.mode == ConnectionMode::Disconnected {
                                    "Tersimpan di PC (colok mouse untuk sinkron)"
                                } else {
                                    "Mengirim seluruh pengaturan ke mouse ATLAS..."
                                },
                                false,
                            );
                        }

                        ui.add_space(6.0);

                        let current_name = self.cfg.active_profile().name.clone();
                        let mut new_idx = self.cfg.active_profile_idx;
                        egui::ComboBox::from_id_salt("header_profile_combo")
                            .selected_text(current_name)
                            .width(142.0)
                            .show_ui(ui, |ui| {
                                for (i, prof) in self.cfg.profiles.iter().enumerate() {
                                    ui.selectable_value(&mut new_idx, i, &prof.name);
                                }
                            });
                        if new_idx != self.cfg.active_profile_idx {
                            self.cfg.active_profile_idx = new_idx;
                            self.dirty_config = true;
                            self.pending_sync.merge(SyncHint::Full);
                            self.last_edit_at = Some(Instant::now());
                        }
                    });
                });
            });

        // Area Konten Utama
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg_canvas).inner_margin(Margin::same(18)))
            .show(ctx, |ui| {
                let sync_hint = match self.tab {
                    NavTab::Buttons => draw_buttons_view(
                        ui,
                        p,
                        &mut self.cfg,
                        &self.status,
                        &mut self.state,
                    ),
                    NavTab::Performance => {
                        draw_performance_view(ui, p, &mut self.cfg, &mut self.state)
                    }
                    NavTab::Macros => draw_macros_view(ui, p, &mut self.cfg, &mut self.state),
                    NavTab::PowerProfiles => {
                        let r = draw_power_profiles_view(
                            ui,
                            p,
                            &mut self.cfg,
                            &self.status,
                            &mut self.state,
                        );
                        if r.reconnect {
                            self.hid.send(HidCommand::Reconnect);
                        }
                        if r.clear_log {
                            self.status.packet_log.clear();
                        }
                        r.sync
                    }
                };

                if sync_hint != SyncHint::None {
                    self.dirty_config = true;
                    self.pending_sync.merge(sync_hint);
                    self.last_edit_at = Some(Instant::now());
                }
            });

        // Notifikasi Toast Melayang
        if let Some((msg, is_err, created)) = self.state.toast.clone() {
            if created.elapsed() < Duration::from_millis(3200) {
                egui::Area::new(egui::Id::new("toast_overlay"))
                    .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-22.0, -34.0))
                    .show(ctx, |ui| {
                        let col = if is_err { p.danger } else { p.accent };
                        Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(14, 18, 28, 248))
                            .stroke(Stroke::new(1.3_f32, col))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(15, 10))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(msg).size(12.5).strong().color(p.text_primary),
                                );
                            });
                    });
            } else {
                self.state.toast = None;
            }
        }

        // Watermark kecil permanen "Eyjel" di pojok kanan bawah aplikasi (non-blocking & immutable)
        egui::Area::new(egui::Id::new("eyjel_watermark"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-14.0, -8.0))
            .interactable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(10, 12, 18, 195))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 85),
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            ui.label(RichText::new("●").size(7.5).color(p.accent));
                            ui.label(
                                RichText::new(WATERMARK_SIGNATURE)
                                    .monospace()
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgba_unmultiplied(230, 238, 252, 210)),
                            );
                        });
                    });
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.cfg.auto_sync && self.pending_sync.needs_hardware_sync() {
            let h = self.pending_sync;
            self.pending_sync = SyncHint::None;
            self.dispatch_sync(h);
        }
        let _ = storage::save_config(&self.cfg);
    }
}
