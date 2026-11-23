use std::collections::VecDeque;
use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Key, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::{
    format_shortcut, hid_keycode_name, snap_dpi, AccentTheme, AppConfig, BatteryState,
    ButtonAction, ButtonCategory, ConnectionMode, DeviceStatus, LiftOffDistance, MacroDef,
    MacroEvent, MacroLoopMode, MouseProfile, PollingRate, PHYSICAL_BUTTONS, SHORTCUT_KEY_CHOICES,
};
use crate::storage;
use crate::ui::mouse_viz::{draw_atlas_mouse_studio, LiveMouseFeedback};
use crate::ui::theme::{
    accent_button, badge_pill, battery_ring_gauge, card_title, danger_button, elevated_card,
    ghost_button, lerp_color, luxury_card, section_eyebrow, toggle_row, Palette,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncHint {
    None,
    DpiSensor,
    PowerDebounce,
    PollingRate,
    Buttons,
    Full,
}

impl SyncHint {
    pub fn merge(&mut self, other: SyncHint) {
        if other == SyncHint::None {
            return;
        }
        if *self == SyncHint::None {
            *self = other;
        } else if *self != other {
            *self = SyncHint::Full;
        }
    }
}

/// Interactive live sensor telemetry tracker (measures real pointer movement Hz & draws fading trail).
pub struct LiveSensorPad {
    pub move_timestamps: VecDeque<Instant>,
    pub trail: VecDeque<(Pos2, Instant)>,
    pub hz_history: VecDeque<f32>,
    pub peak_hz: u32,
    pub last_pos: Option<Pos2>,
    pub last_sample_push: Instant,
}

impl Default for LiveSensorPad {
    fn default() -> Self {
        Self {
            move_timestamps: VecDeque::with_capacity(512),
            trail: VecDeque::with_capacity(64),
            hz_history: VecDeque::from(vec![0.0; 48]),
            peak_hz: 0,
            last_pos: None,
            last_sample_push: Instant::now(),
        }
    }
}

impl LiveSensorPad {
    pub fn update(&mut self, hover_pos: Option<Pos2>) -> u32 {
        let now = Instant::now();
        if let Some(pos) = hover_pos {
            let moved = self
                .last_pos
                .map(|p| (p - pos).length_sq() > 0.25)
                .unwrap_or(true);
            if moved {
                self.last_pos = Some(pos);
                self.move_timestamps.push_back(now);
                self.trail.push_back((pos, now));
                if self.trail.len() > 42 {
                    self.trail.pop_front();
                }
            }
        } else {
            self.last_pos = None;
        }

        while let Some(&front) = self.move_timestamps.front() {
            if now.duration_since(front).as_millis() > 400 {
                self.move_timestamps.pop_front();
            } else {
                break;
            }
        }
        while let Some(&(_, t)) = self.trail.front() {
            if now.duration_since(t).as_secs_f32() > 0.45 {
                self.trail.pop_front();
            } else {
                break;
            }
        }

        let current_hz = ((self.move_timestamps.len() as f32) / 0.4).round() as u32;
        if current_hz > self.peak_hz {
            self.peak_hz = current_hz;
        }
        if self.last_sample_push.elapsed().as_millis() >= 60 {
            self.last_sample_push = now;
            self.hz_history.push_back(current_hz as f32);
            if self.hz_history.len() > 48 {
                self.hz_history.pop_front();
            }
        }
        current_hz
    }

    pub fn reset_peak(&mut self) {
        self.peak_hz = 0;
    }
}

pub struct UiTransientState {
    pub selected_btn_idx: usize,
    pub btn_category: ButtonCategory,
    pub shortcut_mods: u8,
    pub shortcut_key: u8,
    pub show_all_8_dpi: bool,
    pub selected_macro_idx: usize,
    pub recording_macro: bool,
    pub last_record_time: Option<Instant>,
    pub manual_macro_key: u8,
    pub manual_macro_delay: u16,
    pub macro_all_delay_ms: u16,
    pub new_profile_name: String,
    pub rename_profile_buf: String,
    pub renaming_profile: bool,
    pub live_mouse: LiveMouseFeedback,
    pub sensor_pad: LiveSensorPad,
    pub toast: Option<(String, bool, Instant)>,
}

impl Default for UiTransientState {
    fn default() -> Self {
        Self {
            selected_btn_idx: 3, // Side Forward M4 selected by default
            btn_category: ButtonCategory::Mouse,
            shortcut_mods: 0x01,
            shortcut_key: 0x06,
            show_all_8_dpi: false,
            selected_macro_idx: 0,
            recording_macro: false,
            last_record_time: None,
            manual_macro_key: 0x04,
            manual_macro_delay: 25,
            macro_all_delay_ms: 20,
            new_profile_name: String::new(),
            rename_profile_buf: String::new(),
            renaming_profile: false,
            live_mouse: LiveMouseFeedback::default(),
            sensor_pad: LiveSensorPad::default(),
            toast: None,
        }
    }
}

impl UiTransientState {
    pub fn show_toast(&mut self, msg: impl Into<String>, is_err: bool) {
        self.toast = Some((msg.into(), is_err, Instant::now()));
    }
}

pub fn draw_buttons_view(
    ui: &mut Ui,
    p: Palette,
    cfg: &mut AppConfig,
    status: &DeviceStatus,
    state: &mut UiTransientState,
) -> SyncHint {
    let mut sync = SyncHint::None;
    let total_w = ui.available_width();
    let left_w = (total_w * 0.62).clamp(500.0, 820.0);
    let right_w = (total_w - left_w - 14.0).max(320.0);

    ui.horizontal_top(|ui| {
        // Kiri: Peta Tombol Mouse Interaktif
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, ui.available_height()),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                luxury_card(p).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            card_title(ui, p, "PETA TOMBOL INTERAKTIF", "Press Play ATLAS");
                        });
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            let prof = cfg.active_profile();
                            badge_pill(
                                ui,
                                p.bg_elevated,
                                p.border_subtle,
                                p.text_secondary,
                                &format!("Debounce {}ms", prof.debounce_ms),
                            );
                            badge_pill(
                                ui,
                                p.bg_elevated,
                                p.border_subtle,
                                p.text_secondary,
                                &format!("{} Hz", prof.polling_rate.hz()),
                            );
                            badge_pill(
                                ui,
                                p.accent_soft,
                                p.accent,
                                p.accent,
                                &format!("{} DPI", prof.active_dpi()),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    let prof = cfg.active_profile();
                    if let Some(clicked) = draw_atlas_mouse_studio(
                        ui,
                        p,
                        prof,
                        &cfg.macros,
                        state.selected_btn_idx,
                        status.mode != ConnectionMode::Disconnected,
                        &mut state.live_mouse,
                    ) {
                        state.selected_btn_idx = clicked;
                    }
                });
            },
        );

        ui.add_space(4.0);

        // Kanan: Pengaturan Fungsi Tombol
        ui.allocate_ui_with_layout(
            Vec2::new(right_w, ui.available_height()),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                luxury_card(p).show(ui, |ui| {
                    let info =
                        PHYSICAL_BUTTONS[state.selected_btn_idx.min(PHYSICAL_BUTTONS.len() - 1)];
                    let current_label =
                        cfg.active_profile().buttons[info.slot].display_label(&cfg.macros);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            section_eyebrow(ui, p, "PENGATURAN TOMBOL");
                            ui.label(
                                RichText::new(format!("{} · {}", info.short_name, info.full_name))
                                    .size(16.5)
                                    .strong()
                                    .color(p.text_primary),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            badge_pill(
                                ui,
                                p.accent_soft,
                                p.accent,
                                p.accent,
                                &format!("Tombol #{}", info.id),
                            );
                        });
                    });

                    ui.add_space(8.0);
                    elevated_card(p).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fungsi Aktif:").size(12.0).color(p.text_muted));
                            ui.label(
                                RichText::new(current_label)
                                    .size(13.5)
                                    .strong()
                                    .color(p.accent),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    section_eyebrow(ui, p, "PILIH KATEGORI FUNGSI");
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                        for (cat, label) in [
                            (ButtonCategory::Mouse, "Mouse"),
                            (ButtonCategory::Dpi, "DPI"),
                            (ButtonCategory::Multimedia, "Media"),
                            (ButtonCategory::Browser, "Browser"),
                            (ButtonCategory::Shortcuts, "Pintasan"),
                            (ButtonCategory::Custom, "Keyboard / Makro"),
                        ] {
                            let active = state.btn_category == cat;
                            let btn = egui::Button::new(
                                RichText::new(label).size(11.5).strong().color(if active {
                                    Color32::from_rgb(8, 10, 15)
                                } else {
                                    p.text_secondary
                                }),
                            )
                            .fill(if active { p.accent } else { p.bg_elevated })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if active { p.accent } else { p.border_subtle },
                            ))
                            .corner_radius(CornerRadius::same(8));
                            if ui.add(btn).clicked() {
                                state.btn_category = cat;
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ScrollArea::vertical()
                        .id_salt("btn_bind_scroll")
                        .max_height((ui.available_height() - 12.0).max(200.0))
                        .show(ui, |ui| {
                            if state.btn_category == ButtonCategory::Custom {
                                elevated_card(p).show(ui, |ui| {
                                    section_eyebrow(ui, p, "KOMBINASI TOMBOL KEYBOARD");
                                    ui.add_space(6.0);
                                    ui.horizontal_wrapped(|ui| {
                                        for (bit, name) in [
                                            (0x01u8, "Ctrl"),
                                            (0x02, "Shift"),
                                            (0x04, "Alt"),
                                            (0x08, "Win"),
                                        ] {
                                            let on = state.shortcut_mods & bit != 0;
                                            let b = egui::Button::new(
                                                RichText::new(name).size(11.5).strong().color(
                                                    if on {
                                                        Color32::from_rgb(8, 10, 15)
                                                    } else {
                                                        p.text_primary
                                                    },
                                                ),
                                            )
                                            .fill(if on { p.accent } else { p.bg_card })
                                            .stroke(Stroke::new(
                                                1.0_f32,
                                                if on { p.accent } else { p.border_subtle },
                                            ))
                                            .corner_radius(CornerRadius::same(7));
                                            if ui.add(b).clicked() {
                                                state.shortcut_mods ^= bit;
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Tombol:").color(p.text_secondary));
                                        egui::ComboBox::from_id_salt("shortcut_key_combo")
                                            .selected_text(hid_keycode_name(state.shortcut_key))
                                            .width(110.0)
                                            .show_ui(ui, |ui| {
                                                for &(code, name) in SHORTCUT_KEY_CHOICES {
                                                    ui.selectable_value(
                                                        &mut state.shortcut_key,
                                                        code,
                                                        name,
                                                    );
                                                }
                                            });
                                    });
                                    ui.add_space(6.0);
                                    let preview =
                                        format_shortcut(state.shortcut_mods, state.shortcut_key);
                                    if accent_button(ui, p, &format!("Pasang  {preview}")).clicked()
                                    {
                                        if assign_button_safe(
                                            cfg,
                                            info.slot,
                                            ButtonAction::Shortcut {
                                                modifiers: state.shortcut_mods,
                                                hid_key: state.shortcut_key,
                                            },
                                            state,
                                        ) {
                                            sync.merge(SyncHint::Buttons);
                                        }
                                    }
                                });

                                ui.add_space(10.0);
                                elevated_card(p).show(ui, |ui| {
                                    section_eyebrow(ui, p, "PASANG MAKRO TERSIMPAN");
                                    ui.add_space(6.0);
                                    if cfg.macros.is_empty() {
                                        ui.label(
                                            RichText::new(
                                                "Buat makro dulu di tab Makro.",
                                            )
                                            .size(12.0)
                                            .color(p.text_muted),
                                        );
                                    } else {
                                        let macro_list: Vec<(String, String, usize)> = cfg
                                            .macros
                                            .iter()
                                            .map(|m| (m.id.clone(), m.name.clone(), m.events.len()))
                                            .collect();
                                        for (mid, mname, count) in macro_list {
                                            let is_cur = matches!(
                                                &cfg.active_profile().buttons[info.slot],
                                                ButtonAction::Macro { macro_id } if macro_id == &mid
                                            );
                                            let btn = egui::Button::new(
                                                RichText::new(format!("{mname}  ({count} langkah)"))
                                                    .size(12.5)
                                                    .color(if is_cur {
                                                        p.accent
                                                    } else {
                                                        p.text_primary
                                                    }),
                                            )
                                            .fill(if is_cur { p.accent_soft } else { p.bg_card })
                                            .stroke(Stroke::new(
                                                1.0_f32,
                                                if is_cur { p.accent } else { p.border_subtle },
                                            ))
                                            .min_size(Vec2::new(ui.available_width(), 32.0));
                                            if ui.add(btn).clicked() {
                                                if assign_button_safe(
                                                    cfg,
                                                    info.slot,
                                                    ButtonAction::Macro { macro_id: mid },
                                                    state,
                                                ) {
                                                    sync.merge(SyncHint::Buttons);
                                                }
                                            }
                                        }
                                    }
                                });
                            } else {
                                let actions = actions_for_category(state.btn_category);
                                let col_w = ((ui.available_width() - 8.0) * 0.5).max(120.0);
                                egui::Grid::new("btn_action_grid")
                                    .num_columns(2)
                                    .spacing(Vec2::new(8.0, 8.0))
                                    .show(ui, |ui| {
                                        for (i, act) in actions.into_iter().enumerate() {
                                            let is_cur =
                                                cfg.active_profile().buttons[info.slot] == act;
                                            let label = act.display_label(&cfg.macros);
                                            let b = egui::Button::new(
                                                RichText::new(label)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(if is_cur {
                                                        p.accent
                                                    } else {
                                                        p.text_primary
                                                    }),
                                            )
                                            .fill(if is_cur { p.accent_soft } else { p.bg_elevated })
                                            .stroke(Stroke::new(
                                                if is_cur { 1.4_f32 } else { 1.0_f32 },
                                                if is_cur { p.accent } else { p.border_subtle },
                                            ))
                                            .corner_radius(CornerRadius::same(9))
                                            .min_size(Vec2::new(col_w, 34.0));
                                            if ui.add(b).clicked() {
                                                if assign_button_safe(cfg, info.slot, act, state) {
                                                    sync.merge(SyncHint::Buttons);
                                                }
                                            }
                                            if i % 2 == 1 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                            }
                        });
                });
            },
        );
    });

    sync
}

fn assign_button_safe(
    cfg: &mut AppConfig,
    slot: usize,
    action: ButtonAction,
    state: &mut UiTransientState,
) -> bool {
    let prof = cfg.active_profile_mut();
    if prof.buttons[slot] == ButtonAction::LeftClick
        && action != ButtonAction::LeftClick
        && prof.count_left_clicks() <= 1
    {
        state.show_toast("Minimal satu tombol harus tetap jadi Klik Kiri!", true);
        return false;
    }
    prof.buttons[slot] = action;
    true
}

fn actions_for_category(cat: ButtonCategory) -> Vec<ButtonAction> {
    match cat {
        ButtonCategory::Mouse => vec![
            ButtonAction::LeftClick,
            ButtonAction::RightClick,
            ButtonAction::MiddleClick,
            ButtonAction::Forward,
            ButtonAction::Backward,
            ButtonAction::DoubleClick,
            ButtonAction::FireButton,
            ButtonAction::EasyAim,
            ButtonAction::ScrollUp,
            ButtonAction::ScrollDown,
            ButtonAction::TiltLeft,
            ButtonAction::TiltRight,
            ButtonAction::Disabled,
        ],
        ButtonCategory::Dpi => vec![
            ButtonAction::DpiCycle,
            ButtonAction::DpiUp,
            ButtonAction::DpiDown,
            ButtonAction::EasyAim,
            ButtonAction::ProfileCycle,
        ],
        ButtonCategory::Multimedia => vec![
            ButtonAction::PlayPause,
            ButtonAction::MediaStop,
            ButtonAction::PrevTrack,
            ButtonAction::NextTrack,
            ButtonAction::VolumeUp,
            ButtonAction::VolumeDown,
            ButtonAction::Mute,
            ButtonAction::MediaPlayer,
        ],
        ButtonCategory::Browser => vec![
            ButtonAction::BrowserBackward,
            ButtonAction::BrowserForward,
            ButtonAction::BrowserRefresh,
            ButtonAction::BrowserHome,
            ButtonAction::BrowserSearch,
            ButtonAction::BrowserFavorites,
            ButtonAction::BrowserStop,
            ButtonAction::Calculator,
            ButtonAction::Email,
            ButtonAction::MyComputer,
        ],
        ButtonCategory::Shortcuts => vec![
            ButtonAction::Copy,
            ButtonAction::Paste,
            ButtonAction::Cut,
            ButtonAction::Undo,
            ButtonAction::Redo,
            ButtonAction::SelectAll,
            ButtonAction::Save,
            ButtonAction::Find,
            ButtonAction::SwapWindows,
            ButtonAction::ShowDesktop,
            ButtonAction::ScreenCapture,
            ButtonAction::CloseWindow,
            ButtonAction::RunCommand,
            ButtonAction::LockPc,
        ],
        ButtonCategory::Custom => vec![],
    }
}

pub fn draw_performance_view(
    ui: &mut Ui,
    p: Palette,
    cfg: &mut AppConfig,
    state: &mut UiTransientState,
) -> SyncHint {
    let mut sync = SyncHint::None;
    let prof = cfg.active_profile_mut();

    ScrollArea::vertical().id_salt("perf_scroll").show(ui, |ui| {
        // 1. Pengaturan DPI Interaktif
        luxury_card(p).show(ui, |ui| {
            let active_idx = prof.active_dpi_stage.min(7);
            let target_dpi = prof.dpi_stages[active_idx].dpi as f32;
            let smooth_dpi = ui
                .ctx()
                .animate_value_with_time(ui.id().with("hero_dpi_val"), target_dpi, 0.16)
                .round() as u32;

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    section_eyebrow(ui, p, "SENSOR OPTIK PAW3311 · SENSITIVITAS AKTIF");
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{smooth_dpi}"))
                                .monospace()
                                .size(36.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.label(RichText::new("DPI").size(15.0).strong().color(p.accent));
                        ui.add_space(10.0);
                        badge_pill(
                            ui,
                            p.accent_soft,
                            p.accent,
                            p.accent,
                            &format!("Level {}", active_idx + 1),
                        );
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let toggle_lbl = if state.show_all_8_dpi {
                        "Tampilkan 6 Level"
                    } else {
                        "Tampilkan 8 Level"
                    };
                    if ghost_button(ui, p, toggle_lbl).clicked() {
                        state.show_all_8_dpi = !state.show_all_8_dpi;
                    }
                    ui.add_space(6.0);
                    for preset in [3200u32, 1600, 800, 400] {
                        let is_match = prof.dpi_stages[active_idx].dpi == preset;
                        let b = egui::Button::new(
                            RichText::new(format!("{preset}"))
                                .monospace()
                                .size(11.8)
                                .strong()
                                .color(if is_match {
                                    Color32::from_rgb(8, 10, 15)
                                } else {
                                    p.text_secondary
                                }),
                        )
                        .fill(if is_match { p.accent } else { p.bg_elevated })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if is_match { p.accent } else { p.border_subtle },
                        ))
                        .corner_radius(CornerRadius::same(8));
                        if ui.add(b).clicked() {
                            prof.dpi_stages[active_idx].dpi = preset;
                            prof.dpi_stages[active_idx].enabled = true;
                            sync.merge(SyncHint::DpiSensor);
                        }
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Seret batang ke atas/bawah atau geser angkanya untuk ubah DPI tiap level (50 – 24.000 DPI).",
                )
                .size(11.5)
                .color(p.text_muted),
            );
            ui.add_space(10.0);

            let visible_count = if state.show_all_8_dpi { 8 } else { 6 };
            let card_gap = 10.0;
            let card_w = ((ui.available_width() - card_gap * (visible_count as f32 - 1.0))
                / visible_count as f32)
                .max(110.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = card_gap;
                for i in 0..visible_count {
                    let is_active = prof.active_dpi_stage == i;
                    let stage_en = prof.dpi_stages[i].enabled;
                    let anim_active = ui.ctx().animate_bool_with_time(
                        ui.id().with(("dpi_card_act", i)),
                        is_active,
                        0.16,
                    );

                    let border = lerp_color(
                        if stage_en {
                            p.border_subtle
                        } else {
                            Color32::from_rgb(22, 26, 36)
                        },
                        p.accent,
                        anim_active,
                    );
                    let card_bg =
                        lerp_color(p.bg_elevated, Color32::from_rgb(20, 26, 40), anim_active);

                    Frame::new()
                        .fill(card_bg)
                        .stroke(Stroke::new(if is_active { 1.6_f32 } else { 1.0_f32 }, border))
                        .corner_radius(CornerRadius::same(13))
                        .inner_margin(Margin::same(11))
                        .show(ui, |ui| {
                            ui.set_width(card_w - 24.0);
                            ui.vertical_centered(|ui| {
                                ui.horizontal(|ui| {
                                    let active_btn = egui::Button::new(
                                        RichText::new(format!("L{}", i + 1))
                                            .monospace()
                                            .size(11.0)
                                            .strong()
                                            .color(if is_active {
                                                Color32::from_rgb(8, 10, 15)
                                            } else {
                                                p.text_secondary
                                            }),
                                    )
                                    .fill(if is_active { p.accent } else { p.bg_card })
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(30.0, 22.0));
                                    if ui.add(active_btn).clicked() {
                                        prof.active_dpi_stage = i;
                                        prof.dpi_stages[i].enabled = true;
                                        sync.merge(SyncHint::DpiSensor);
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        let enabled_count =
                                            prof.dpi_stages.iter().filter(|s| s.enabled).count();
                                        let mut en = prof.dpi_stages[i].enabled;
                                        if ui.checkbox(&mut en, "").changed() {
                                            if en || enabled_count > 1 {
                                                prof.dpi_stages[i].enabled = en;
                                                sync.merge(SyncHint::DpiSensor);
                                            }
                                        }
                                    });
                                });

                                ui.add_space(8.0);

                                let bar_h = 86.0;
                                let bar_w = 28.0;
                                let (bar_rect, bar_resp) = ui.allocate_exact_size(
                                    Vec2::new(bar_w, bar_h),
                                    Sense::click_and_drag(),
                                );
                                ui.painter().rect_filled(
                                    bar_rect,
                                    CornerRadius::same(8),
                                    p.bg_input,
                                );
                                for t in [0.25_f32, 0.5, 0.75] {
                                    let ty = egui::lerp(bar_rect.top()..=bar_rect.bottom(), t);
                                    ui.painter().line_segment(
                                        [
                                            Pos2::new(bar_rect.left() + 4.0, ty),
                                            Pos2::new(bar_rect.right() - 4.0, ty),
                                        ],
                                        Stroke::new(
                                            1.0_f32,
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 14),
                                        ),
                                    );
                                }

                                let target_frac = dpi_to_visual_frac(prof.dpi_stages[i].dpi);
                                let smooth_frac = ui.ctx().animate_value_with_time(
                                    ui.id().with(("dpi_bar_h", i)),
                                    target_frac,
                                    0.12,
                                );
                                let fill_h = (bar_h * smooth_frac).clamp(8.0, bar_h - 4.0);
                                let fill_rect = Rect::from_min_max(
                                    Pos2::new(
                                        bar_rect.left() + 3.0,
                                        bar_rect.bottom() - fill_h - 2.0,
                                    ),
                                    Pos2::new(bar_rect.right() - 3.0, bar_rect.bottom() - 2.0),
                                );
                                let bar_col = if !stage_en {
                                    Color32::from_rgb(44, 50, 66)
                                } else if is_active || bar_resp.hovered() {
                                    p.accent
                                } else {
                                    Color32::from_rgba_unmultiplied(
                                        p.accent.r(),
                                        p.accent.g(),
                                        p.accent.b(),
                                        125,
                                    )
                                };
                                ui.painter().rect_filled(fill_rect, CornerRadius::same(5), bar_col);
                                if stage_en {
                                    let cap = Rect::from_min_size(
                                        fill_rect.left_top(),
                                        Vec2::new(fill_rect.width(), 3.0),
                                    );
                                    ui.painter().rect_filled(cap, CornerRadius::same(2), Color32::WHITE);
                                }

                                if bar_resp.dragged() || bar_resp.clicked() {
                                    if let Some(pos) = bar_resp.interact_pointer_pos() {
                                        let f =
                                            ((bar_rect.bottom() - pos.y) / bar_h).clamp(0.0, 1.0);
                                        prof.dpi_stages[i].dpi = visual_frac_to_dpi(f);
                                        prof.active_dpi_stage = i;
                                        prof.dpi_stages[i].enabled = true;
                                        sync.merge(SyncHint::DpiSensor);
                                    }
                                }

                                ui.add_space(7.0);

                                let mut dpi_val = prof.dpi_stages[i].dpi;
                                let step = if dpi_val <= 10000 { 50.0 } else { 100.0 };
                                let dv = egui::DragValue::new(&mut dpi_val)
                                    .range(50..=24000)
                                    .speed(step)
                                    .suffix(" DPI");
                                if ui.add(dv).changed() {
                                    prof.dpi_stages[i].dpi = snap_dpi(dpi_val);
                                    prof.active_dpi_stage = i;
                                    sync.merge(SyncHint::DpiSensor);
                                }
                            });
                        });
                }
            });
        });

        ui.add_space(10.0);

        // 2. Bawah: Polling Rate + Pad Tes Sensor (Kiri) | Kalibrasi Sensor & Switch (Kanan)
        let half_w = ((ui.available_width() - 14.0) * 0.5).max(320.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(half_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    luxury_card(p).show(ui, |ui| {
                        card_title(ui, p, "FREKUENSI LAPORAN USB", "Polling Rate");
                        ui.add_space(10.0);

                        let cell_w = ((ui.available_width() - 10.0) * 0.5).max(120.0);
                        egui::Grid::new("polling_grid")
                            .num_columns(2)
                            .spacing(Vec2::new(10.0, 10.0))
                            .show(ui, |ui| {
                                for (i, rate) in PollingRate::ALL.into_iter().enumerate() {
                                    let active = prof.polling_rate == rate;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(cell_w, 64.0),
                                        Sense::click(),
                                    );
                                    let anim = ui.ctx().animate_bool_with_time(
                                        ui.id().with(("poll_anim", i)),
                                        active || resp.hovered(),
                                        0.12,
                                    );
                                    let fill = if active {
                                        p.accent_soft
                                    } else {
                                        lerp_color(p.bg_elevated, Color32::from_rgb(28, 34, 50), anim)
                                    };
                                    let border = lerp_color(p.border_subtle, p.accent, anim);
                                    ui.painter().rect(
                                        rect,
                                        CornerRadius::same(11),
                                        fill,
                                        Stroke::new(if active { 1.6_f32 } else { 1.0_f32 }, border),
                                        StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        rect.left_top() + Vec2::new(12.0, 18.0),
                                        Align2::LEFT_CENTER,
                                        format!("{} Hz", rate.hz()),
                                        FontId::monospace(17.0),
                                        if active { p.accent } else { p.text_primary },
                                    );
                                    ui.painter().text(
                                        rect.right_top() + Vec2::new(-12.0, 18.0),
                                        Align2::RIGHT_CENTER,
                                        format!("{:.1} ms", rate.latency_ms()),
                                        FontId::monospace(11.5),
                                        p.text_secondary,
                                    );
                                    ui.painter().text(
                                        rect.left_bottom() + Vec2::new(12.0, -15.0),
                                        Align2::LEFT_CENTER,
                                        rate.tier_label(),
                                        FontId::proportional(11.2),
                                        if active { p.text_primary } else { p.text_muted },
                                    );
                                    if resp.clicked() && !active {
                                        prof.polling_rate = rate;
                                        sync.merge(SyncHint::PollingRate);
                                    }
                                    if i % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                            });

                        ui.add_space(12.0);
                        draw_live_sensor_pad(ui, p, &mut state.sensor_pad);
                    });
                },
            );

            ui.add_space(4.0);

            ui.allocate_ui_with_layout(
                Vec2::new(half_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    luxury_card(p).show(ui, |ui| {
                        card_title(ui, p, "KALIBRASI PRESISI", "Sensor & Kecepatan Klik");
                        ui.add_space(10.0);

                        // LOD Visual Diagram
                        elevated_card(p).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("Jarak Angkat Sensor (LOD)")
                                            .size(13.5)
                                            .strong()
                                            .color(p.text_primary),
                                    );
                                    ui.label(
                                        RichText::new("Batas tinggi saat mouse diangkat dari mousepad")
                                            .size(11.2)
                                            .color(p.text_secondary),
                                    );
                                });
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    for lod in [LiftOffDistance::Mm2, LiftOffDistance::Mm1] {
                                        let sel = prof.lod == lod;
                                        let b = egui::Button::new(
                                            RichText::new(lod.label())
                                                .monospace()
                                                .size(11.8)
                                                .strong()
                                                .color(if sel {
                                                    Color32::from_rgb(8, 10, 15)
                                                } else {
                                                    p.text_primary
                                                }),
                                        )
                                        .fill(if sel { p.accent } else { p.bg_card })
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            if sel { p.accent } else { p.border_subtle },
                                        ))
                                        .corner_radius(CornerRadius::same(8));
                                        if ui.add(b).clicked() {
                                            prof.lod = lod;
                                            sync.merge(SyncHint::DpiSensor);
                                        }
                                    }
                                });
                            });
                            ui.add_space(8.0);
                            draw_animated_lod_diagram(ui, p, prof.lod);
                        });

                        ui.add_space(8.0);

                        // Debounce
                        elevated_card(p).show(ui, |ui| {
                            let tier = if prof.debounce_ms <= 4 {
                                ("Turnamen (Ultra Cepat)", p.accent)
                            } else if prof.debounce_ms <= 10 {
                                ("Esports", p.success)
                            } else if prof.debounce_ms <= 20 {
                                ("Seimbang", p.warning)
                            } else {
                                ("Anti Double-Click", p.text_secondary)
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Respon Klik (Debounce)")
                                        .size(13.5)
                                        .strong()
                                        .color(p.text_primary),
                                );
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    badge_pill(
                                        ui,
                                        p.accent_soft,
                                        p.accent,
                                        p.accent,
                                        &format!("{} ms", prof.debounce_ms),
                                    );
                                    ui.label(
                                        RichText::new(tier.0).size(11.0).strong().color(tier.1),
                                    );
                                });
                            });
                            ui.add_space(5.0);
                            let mut step_val = (prof.debounce_ms / 2).clamp(1, 25);
                            ui.spacing_mut().slider_width = (ui.available_width() - 12.0).max(160.0);
                            if ui
                                .add(egui::Slider::new(&mut step_val, 1..=25).show_value(false))
                                .changed()
                            {
                                prof.debounce_ms = step_val * 2;
                                sync.merge(SyncHint::PowerDebounce);
                            }
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("2 ms · Paling Responsif")
                                        .size(10.5)
                                        .color(p.text_muted),
                                );
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("50 ms · Filter Maksimal")
                                            .size(10.5)
                                            .color(p.text_muted),
                                    );
                                });
                            });
                        });

                        ui.add_space(8.0);
                        if toggle_row(
                            ui,
                            p,
                            "Ripple Control (Peredam Getaran)",
                            "Mengurangi getaran kursor halus pada DPI tinggi (>3200)",
                            &mut prof.ripple_control,
                        ) {
                            sync.merge(SyncHint::DpiSensor);
                        }

                        ui.add_space(8.0);
                        if toggle_row(
                            ui,
                            p,
                            "Angle Snapping (Pelurus Garis)",
                            "Meluruskan gerakan garis lurus (matikan untuk aim FPS murni)",
                            &mut prof.angle_snap,
                        ) {
                            sync.merge(SyncHint::DpiSensor);
                        }

                        ui.add_space(10.0);
                        if ghost_button(ui, p, "Buka Pengaturan Mouse Windows").clicked() {
                            storage::open_windows_mouse_properties();
                        }
                    });
                },
            );
        });
    });

    sync
}

fn draw_live_sensor_pad(ui: &mut Ui, p: Palette, pad: &mut LiveSensorPad) {
    elevated_card(p).show(ui, |ui| {
        ui.horizontal(|ui| {
            section_eyebrow(ui, p, "PAD TES SENSOR & HZ LANGSUNG");
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("Reset Puncak").clicked() {
                    pad.reset_peak();
                }
            });
        });
        ui.add_space(6.0);

        let pad_w = ui.available_width();
        let pad_h = 112.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(pad_w, pad_h), Sense::hover());
        let hz = pad.update(resp.hover_pos());
        if resp.hovered() || !pad.trail.is_empty() {
            ui.ctx().request_repaint();
        }

        let painter = ui.painter_at(rect);
        painter.rect(
            rect,
            CornerRadius::same(10),
            p.bg_input,
            Stroke::new(
                1.0_f32,
                if resp.hovered() { p.accent } else { p.border_subtle },
            ),
            StrokeKind::Inside,
        );

        let c = rect.center();
        painter.line_segment(
            [Pos2::new(rect.left() + 8.0, c.y), Pos2::new(rect.right() - 8.0, c.y)],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(120, 140, 180, 18)),
        );
        painter.line_segment(
            [Pos2::new(c.x, rect.top() + 8.0), Pos2::new(c.x, rect.bottom() - 8.0)],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(120, 140, 180, 18)),
        );

        let max_hist = pad.hz_history.iter().copied().fold(250.0_f32, f32::max);
        let n = pad.hz_history.len();
        if n >= 2 {
            let mut pts = Vec::with_capacity(n);
            for (idx, &val) in pad.hz_history.iter().enumerate() {
                let x = egui::lerp(
                    (rect.left() + 8.0)..=(rect.right() - 8.0),
                    idx as f32 / (n - 1) as f32,
                );
                let y = rect.bottom() - 8.0 - (val / max_hist).clamp(0.0, 1.0) * (pad_h * 0.48);
                pts.push(Pos2::new(x, y));
            }
            painter.add(Shape::line(
                pts,
                Stroke::new(
                    1.6_f32,
                    Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 110),
                ),
            ));
        }

        let now = Instant::now();
        let trail_vec: Vec<(Pos2, Instant)> = pad.trail.iter().copied().collect();
        for w in trail_vec.windows(2) {
            let age = now.duration_since(w[1].1).as_secs_f32();
            let alpha = ((1.0 - (age / 0.45)).clamp(0.0, 1.0) * 235.0) as u8;
            let width = 1.4 + (1.0 - (age / 0.45)).clamp(0.0, 1.0) * 2.4;
            painter.line_segment(
                [w[0].0, w[1].0],
                Stroke::new(
                    width,
                    Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
                ),
            );
        }
        if let Some(pos) = resp.hover_pos() {
            painter.circle_filled(pos, 4.0, p.accent);
        }

        painter.text(
            rect.left_top() + Vec2::new(10.0, 14.0),
            Align2::LEFT_CENTER,
            format!("LIVE  {hz} Hz"),
            FontId::monospace(12.0),
            if hz > 0 { p.accent } else { p.text_muted },
        );
        painter.text(
            rect.right_top() + Vec2::new(-10.0, 14.0),
            Align2::RIGHT_CENTER,
            format!("PUNCAK  {} Hz", pad.peak_hz),
            FontId::monospace(11.2),
            p.text_secondary,
        );
        if !resp.hovered() && pad.trail.is_empty() {
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "Gerakkan mouse di area ini untuk tes kecepatan sensor",
                FontId::proportional(11.5),
                p.text_muted,
            );
        }
    });
}

fn draw_animated_lod_diagram(ui: &mut Ui, p: Palette, lod: LiftOffDistance) {
    let target_mm = match lod {
        LiftOffDistance::Mm1 => 1.0_f32,
        LiftOffDistance::Mm2 => 2.0_f32,
    };
    let anim_mm = ui
        .ctx()
        .animate_value_with_time(ui.id().with("lod_height_mm"), target_mm, 0.2);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 66.0), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, CornerRadius::same(9), p.bg_input);

    let pad_y = rect.bottom() - 12.0;
    painter.line_segment(
        [Pos2::new(rect.left() + 14.0, pad_y), Pos2::new(rect.right() - 14.0, pad_y)],
        Stroke::new(2.0_f32, p.border_strong),
    );

    let lift_px = 5.0 + (anim_mm - 1.0) * 11.0;
    let base_y = pad_y - lift_px;
    let cx = rect.center().x - 16.0;

    let profile_pts = vec![
        Pos2::new(cx - 48.0, base_y),
        Pos2::new(cx - 42.0, base_y - 11.0),
        Pos2::new(cx - 18.0, base_y - 21.0),
        Pos2::new(cx + 12.0, base_y - 22.0),
        Pos2::new(cx + 38.0, base_y - 14.0),
        Pos2::new(cx + 50.0, base_y - 4.0),
        Pos2::new(cx + 48.0, base_y),
    ];

    let sensor_origin = Pos2::new(cx + 2.0, base_y);
    let cone_half = 4.0 + lift_px * 0.45;
    let cone_pts = vec![
        sensor_origin,
        Pos2::new(sensor_origin.x - cone_half, pad_y),
        Pos2::new(sensor_origin.x + cone_half, pad_y),
    ];
    painter.add(Shape::convex_polygon(
        cone_pts,
        Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 48),
        Stroke::NONE,
    ));
    painter.line_segment(
        [sensor_origin, Pos2::new(sensor_origin.x, pad_y)],
        Stroke::new(1.4_f32, p.accent),
    );

    painter.add(Shape::convex_polygon(
        profile_pts,
        Color32::from_rgb(36, 41, 54),
        Stroke::new(1.2_f32, Color32::from_rgb(92, 102, 128)),
    ));

    let dim_x = cx + 76.0;
    painter.line_segment(
        [Pos2::new(dim_x, base_y), Pos2::new(dim_x, pad_y)],
        Stroke::new(1.2_f32, p.accent),
    );
    painter.text(
        Pos2::new(dim_x + 8.0, (base_y + pad_y) * 0.5),
        Align2::LEFT_CENTER,
        format!("{anim_mm:.1} mm"),
        FontId::monospace(11.5),
        p.accent,
    );
}

fn dpi_to_visual_frac(dpi: u32) -> f32 {
    let min_l = (50.0_f32).ln();
    let max_l = (24000.0_f32).ln();
    (((dpi.clamp(50, 24000) as f32).ln() - min_l) / (max_l - min_l)).clamp(0.0, 1.0)
}

fn visual_frac_to_dpi(frac: f32) -> u32 {
    let min_l = (50.0_f32).ln();
    let max_l = (24000.0_f32).ln();
    let raw = (min_l + frac.clamp(0.0, 1.0) * (max_l - min_l)).exp() as u32;
    snap_dpi(raw)
}

pub fn draw_macros_view(
    ui: &mut Ui,
    p: Palette,
    cfg: &mut AppConfig,
    state: &mut UiTransientState,
) -> SyncHint {
    let mut sync = SyncHint::None;

    if state.recording_macro {
        if let Some(m) = cfg.macros.get_mut(state.selected_macro_idx) {
            let events = ui.input(|i| i.events.clone());
            for ev in events {
                if let egui::Event::Key {
                    key,
                    pressed,
                    repeat: false,
                    ..
                } = ev
                {
                    if let Some(hid) = egui_key_to_hid(key) {
                        let now = Instant::now();
                        let delay = state
                            .last_record_time
                            .map(|t| t.elapsed().as_millis().clamp(10, 1200) as u16)
                            .unwrap_or(20);
                        state.last_record_time = Some(now);
                        if m.events.len() < 50 {
                            m.events.push(MacroEvent {
                                hid_key: hid,
                                pressed,
                                delay_ms: delay,
                            });
                        }
                    }
                }
            }
        }
    }

    let left_w = 246.0;
    let right_w = (ui.available_width() - left_w - 14.0).max(360.0);

    ui.horizontal_top(|ui| {
        // Kiri: Daftar Makro
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, ui.available_height()),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                luxury_card(p).show(ui, |ui| {
                    card_title(ui, p, "OTOMATISASI", "Daftar Makro");
                    ui.add_space(8.0);
                    if accent_button(ui, p, "+  Buat Makro Baru").clicked() {
                        let n = cfg.macros.len() + 1;
                        let id = format!(
                            "macro_{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis())
                                .unwrap_or(n as u128)
                        );
                        cfg.macros.push(MacroDef::new(id, format!("Makro {n}")));
                        state.selected_macro_idx = cfg.macros.len() - 1;
                    }
                    ui.add_space(10.0);

                    ScrollArea::vertical()
                        .id_salt("macro_list_scroll")
                        .max_height((ui.available_height() - 16.0).max(240.0))
                        .show(ui, |ui| {
                            let mut delete_idx: Option<usize> = None;
                            for (i, m) in cfg.macros.iter().enumerate() {
                                let sel = state.selected_macro_idx == i;
                                Frame::new()
                                    .fill(if sel { p.accent_soft } else { p.bg_elevated })
                                    .stroke(Stroke::new(
                                        if sel { 1.4_f32 } else { 1.0_f32 },
                                        if sel { p.accent } else { p.border_subtle },
                                    ))
                                    .corner_radius(CornerRadius::same(10))
                                    .inner_margin(Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .selectable_label(
                                                    sel,
                                                    RichText::new(&m.name).size(13.0).strong().color(
                                                        if sel {
                                                            p.accent
                                                        } else {
                                                            p.text_primary
                                                        },
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                state.selected_macro_idx = i;
                                                state.recording_macro = false;
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui
                                                        .small_button(
                                                            RichText::new("x").color(p.danger),
                                                        )
                                                        .clicked()
                                                    {
                                                        delete_idx = Some(i);
                                                    }
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} step",
                                                            m.events.len()
                                                        ))
                                                        .monospace()
                                                        .size(10.5)
                                                        .color(p.text_muted),
                                                    );
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                            if let Some(di) = delete_idx {
                                cfg.macros.remove(di);
                                if state.selected_macro_idx >= cfg.macros.len() {
                                    state.selected_macro_idx = cfg.macros.len().saturating_sub(1);
                                }
                            }
                        });
                });
            },
        );

        ui.add_space(4.0);

        // Kanan: Editor Makro + Fitur ALL DRAG Delay ms
        ui.allocate_ui_with_layout(
            Vec2::new(right_w, ui.available_height()),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                luxury_card(p).show(ui, |ui| {
                    if cfg.macros.is_empty() {
                        ui.label(
                            RichText::new(
                                "Belum ada makro. Klik '+ Buat Makro Baru' di sebelah kiri.",
                            )
                            .color(p.text_secondary),
                        );
                        return;
                    }
                    let idx = state.selected_macro_idx.min(cfg.macros.len() - 1);
                    state.selected_macro_idx = idx;
                    let macro_id = cfg.macros[idx].id.clone();

                    // Header: Nama + Tombol Rekam
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Nama Makro:").color(p.text_muted));
                        ui.add(
                            egui::TextEdit::singleline(&mut cfg.macros[idx].name)
                                .desired_width(190.0),
                        );

                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if state.recording_macro {
                                if danger_button(ui, p, "■  Stop Rekam").clicked() {
                                    state.recording_macro = false;
                                }
                            } else if accent_button(ui, p, "●  Rekam Keyboard").clicked() {
                                state.recording_macro = true;
                                state.last_record_time = None;
                            }
                        });
                    });

                    if state.recording_macro {
                        ui.add_space(6.0);
                        badge_pill(
                            ui,
                            Color32::from_rgba_unmultiplied(255, 76, 104, 34),
                            p.danger,
                            p.danger,
                            "SEDANG MEREKAM — Tekan tombol keyboard kamu (maksimal 50 langkah)...",
                        );
                    }

                    ui.add_space(8.0);

                    // Pengaturan Loop + Pasang Cepat ke M4 / M5
                    elevated_card(p).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Mode Jalan:").color(p.text_secondary));
                            egui::ComboBox::from_id_salt("macro_loop_combo")
                                .selected_text(cfg.macros[idx].loop_mode.label())
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    for mode in MacroLoopMode::ALL {
                                        ui.selectable_value(
                                            &mut cfg.macros[idx].loop_mode,
                                            mode,
                                            mode.label(),
                                        );
                                    }
                                });

                            if cfg.macros[idx].loop_mode == MacroLoopMode::RepeatCount {
                                ui.label(RichText::new("Ulangi:").color(p.text_secondary));
                                ui.add(
                                    egui::DragValue::new(&mut cfg.macros[idx].loop_count)
                                        .range(1..=99)
                                        .suffix("x"),
                                );
                            }

                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ghost_button(ui, p, "Pasang ke M5").clicked() {
                                    cfg.active_profile_mut().buttons[4] = ButtonAction::Macro {
                                        macro_id: macro_id.clone(),
                                    };
                                    state.show_toast("Makro dipasang ke tombol M5 (Mundur)", false);
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ghost_button(ui, p, "Pasang ke M4").clicked() {
                                    cfg.active_profile_mut().buttons[5] = ButtonAction::Macro {
                                        macro_id: macro_id.clone(),
                                    };
                                    state.show_toast("Makro dipasang ke tombol M4 (Maju)", false);
                                    sync.merge(SyncHint::Buttons);
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);

                    // FITUR ALL DRAG DELAY: Atur / turunkan semua delay ms sekaligus!
                    Frame::new()
                        .fill(Color32::from_rgb(19, 24, 36))
                        .stroke(Stroke::new(1.2_f32, p.accent))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            let total_ms: u32 = cfg.macros[idx]
                                .events
                                .iter()
                                .map(|e| e.delay_ms as u32)
                                .sum();

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("ALL DRAG DELAY — Atur Semua Delay Sekaligus")
                                        .size(11.5)
                                        .strong()
                                        .color(p.accent),
                                );
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    badge_pill(
                                        ui,
                                        p.accent_soft,
                                        p.accent,
                                        p.accent,
                                        &format!("Total Durasi: {total_ms} ms"),
                                    );
                                });
                            });
                            ui.add_space(6.0);

                            // Slider All-Drag: geser langsung ubah SEMUA step delay sekaligus!
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Geser Semua:")
                                        .size(12.0)
                                        .color(p.text_secondary),
                                );
                                ui.spacing_mut().slider_width =
                                    (ui.available_width() - 185.0).max(160.0);
                                let slider_resp = ui.add(
                                    egui::Slider::new(&mut state.macro_all_delay_ms, 10..=500)
                                        .suffix(" ms"),
                                );
                                if slider_resp.changed() {
                                    let val = state.macro_all_delay_ms;
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = val;
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                            });

                            ui.add_space(6.0);

                            // Tombol Cepat Turunin / Skala Semua Delay
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                                ui.label(
                                    RichText::new("Ubah Cepat:")
                                        .size(11.5)
                                        .color(p.text_muted),
                                );
                                if ui.small_button("-20ms Semua").clicked() {
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = ev.delay_ms.saturating_sub(20).max(10);
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ui.small_button("-10ms Semua").clicked() {
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = ev.delay_ms.saturating_sub(10).max(10);
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ui.small_button("-5ms Semua").clicked() {
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = ev.delay_ms.saturating_sub(5).max(10);
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ui.small_button("+5ms Semua").clicked() {
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = (ev.delay_ms + 5).min(1270);
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ui.small_button("÷2 (50% Lebih Cepat)").clicked() {
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = (ev.delay_ms / 2).max(10);
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                                if ui
                                    .small_button(RichText::new("Min (10ms)").color(p.accent))
                                    .clicked()
                                {
                                    state.macro_all_delay_ms = 10;
                                    for ev in &mut cfg.macros[idx].events {
                                        ev.delay_ms = 10;
                                    }
                                    sync.merge(SyncHint::Buttons);
                                }
                            });
                        });

                    ui.add_space(8.0);

                    // Tambah Langkah Manual
                    elevated_card(p).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Tambah Manual:").color(p.text_secondary));
                            egui::ComboBox::from_id_salt("manual_macro_key")
                                .selected_text(hid_keycode_name(state.manual_macro_key))
                                .width(105.0)
                                .show_ui(ui, |ui| {
                                    for &(code, name) in SHORTCUT_KEY_CHOICES {
                                        ui.selectable_value(&mut state.manual_macro_key, code, name);
                                    }
                                    ui.separator();
                                    for (code, name) in [
                                        (0xF0u8, "Klik Kiri"),
                                        (0xF1, "Klik Kanan"),
                                        (0xF2, "Klik Tengah"),
                                    ] {
                                        ui.selectable_value(&mut state.manual_macro_key, code, name);
                                    }
                                });
                            ui.label(RichText::new("Delay:").color(p.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut state.manual_macro_delay)
                                    .range(10..=1200)
                                    .suffix(" ms"),
                            );
                            if ghost_button(ui, p, "+ Tambah Tekan & Lepas").clicked()
                                && cfg.macros[idx].events.len() + 2 <= 50
                            {
                                cfg.macros[idx].events.push(MacroEvent {
                                    hid_key: state.manual_macro_key,
                                    pressed: true,
                                    delay_ms: state.manual_macro_delay,
                                });
                                cfg.macros[idx].events.push(MacroEvent {
                                    hid_key: state.manual_macro_key,
                                    pressed: false,
                                    delay_ms: state.manual_macro_delay,
                                });
                                sync.merge(SyncHint::Buttons);
                            }
                            if danger_button(ui, p, "Hapus Semua").clicked() {
                                cfg.macros[idx].events.clear();
                                sync.merge(SyncHint::Buttons);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    section_eyebrow(
                        ui,
                        p,
                        &format!(
                            "URUTAN LANGKAH MAKRO ({} / 50 LANGKAH)",
                            cfg.macros[idx].events.len()
                        ),
                    );
                    ui.add_space(4.0);

                    ScrollArea::vertical()
                        .id_salt("macro_events_scroll")
                        .max_height((ui.available_height() - 10.0).max(140.0))
                        .show(ui, |ui| {
                            let mut action_swap: Option<(usize, usize)> = None;
                            let mut action_del: Option<usize> = None;
                            let len = cfg.macros[idx].events.len();

                            for (ei, ev) in cfg.macros[idx].events.iter_mut().enumerate() {
                                Frame::new()
                                    .fill(p.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(Margin::symmetric(10, 5))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("#{:02}", ei + 1))
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(p.text_muted),
                                            );
                                            let badge_col =
                                                if ev.pressed { p.success } else { p.warning };
                                            badge_pill(
                                                ui,
                                                Color32::from_rgba_unmultiplied(
                                                    badge_col.r(),
                                                    badge_col.g(),
                                                    badge_col.b(),
                                                    26,
                                                ),
                                                badge_col,
                                                badge_col,
                                                if ev.pressed { "TEKAN" } else { "LEPAS" },
                                            );
                                            ui.label(
                                                RichText::new(hid_keycode_name(ev.hid_key))
                                                    .size(13.0)
                                                    .strong()
                                                    .color(p.text_primary),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui
                                                        .small_button(
                                                            RichText::new("x").color(p.danger),
                                                        )
                                                        .clicked()
                                                    {
                                                        action_del = Some(ei);
                                                    }
                                                    if ei + 1 < len && ui.small_button("v").clicked()
                                                    {
                                                        action_swap = Some((ei, ei + 1));
                                                    }
                                                    if ei > 0 && ui.small_button("^").clicked() {
                                                        action_swap = Some((ei - 1, ei));
                                                    }
                                                    if ui
                                                        .add(
                                                            egui::DragValue::new(&mut ev.delay_ms)
                                                                .range(10..=1270)
                                                                .suffix(" ms"),
                                                        )
                                                        .changed()
                                                    {
                                                        sync.merge(SyncHint::Buttons);
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(3.0);
                            }

                            if let Some((a, b)) = action_swap {
                                cfg.macros[idx].events.swap(a, b);
                                sync.merge(SyncHint::Buttons);
                            }
                            if let Some(d) = action_del {
                                cfg.macros[idx].events.remove(d);
                                sync.merge(SyncHint::Buttons);
                            }
                        });
                });
            },
        );
    });

    sync
}

fn egui_key_to_hid(key: Key) -> Option<u8> {
    Some(match key {
        Key::A => 0x04, Key::B => 0x05, Key::C => 0x06, Key::D => 0x07,
        Key::E => 0x08, Key::F => 0x09, Key::G => 0x0A, Key::H => 0x0B,
        Key::I => 0x0C, Key::J => 0x0D, Key::K => 0x0E, Key::L => 0x0F,
        Key::M => 0x10, Key::N => 0x11, Key::O => 0x12, Key::P => 0x13,
        Key::Q => 0x14, Key::R => 0x15, Key::S => 0x16, Key::T => 0x17,
        Key::U => 0x18, Key::V => 0x19, Key::W => 0x1A, Key::X => 0x1B,
        Key::Y => 0x1C, Key::Z => 0x1D,
        Key::Num1 => 0x1E, Key::Num2 => 0x1F, Key::Num3 => 0x20, Key::Num4 => 0x21,
        Key::Num5 => 0x22, Key::Num6 => 0x23, Key::Num7 => 0x24, Key::Num8 => 0x25,
        Key::Num9 => 0x26, Key::Num0 => 0x27,
        Key::Enter => 0x28, Key::Escape => 0x29, Key::Backspace => 0x2A,
        Key::Tab => 0x2B, Key::Space => 0x2C,
        Key::F1 => 0x3A, Key::F2 => 0x3B, Key::F3 => 0x3C, Key::F4 => 0x3D,
        Key::F5 => 0x3E, Key::F6 => 0x3F, Key::F7 => 0x40, Key::F8 => 0x41,
        Key::F9 => 0x42, Key::F10 => 0x43, Key::F11 => 0x44, Key::F12 => 0x45,
        Key::ArrowRight => 0x4F, Key::ArrowLeft => 0x50,
        Key::ArrowDown => 0x51, Key::ArrowUp => 0x52,
        _ => return None,
    })
}

pub struct PowerProfilesResult {
    pub sync: SyncHint,
    pub reconnect: bool,
    pub clear_log: bool,
}

pub fn draw_power_profiles_view(
    ui: &mut Ui,
    p: Palette,
    cfg: &mut AppConfig,
    status: &DeviceStatus,
    state: &mut UiTransientState,
) -> PowerProfilesResult {
    let mut res = PowerProfilesResult {
        sync: SyncHint::None,
        reconnect: false,
        clear_log: false,
    };

    ScrollArea::vertical().id_salt("power_prof_scroll").show(ui, |ui| {
        let half_w = ((ui.available_width() - 14.0) * 0.5).max(320.0);

        ui.horizontal_top(|ui| {
            // Kiri: Baterai & Mode Tidur
            ui.allocate_ui_with_layout(
                Vec2::new(half_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    luxury_card(p).show(ui, |ui| {
                        card_title(ui, p, "MANAJEMEN DAYA", "Baterai & Mode Tidur");
                        ui.add_space(12.0);

                        let charging = matches!(status.battery, BatteryState::Charging(_));
                        let pct = status.battery.percent();

                        ui.horizontal(|ui| {
                            battery_ring_gauge(ui, p, pct, charging, 128.0);
                            ui.add_space(14.0);
                            ui.vertical(|ui| {
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("Status Koneksi")
                                        .size(11.0)
                                        .color(p.text_muted),
                                );
                                ui.label(
                                    RichText::new(status.mode.label())
                                        .size(15.5)
                                        .strong()
                                        .color(p.text_primary),
                                );
                                ui.add_space(8.0);
                                let batt_desc = match status.battery {
                                    BatteryState::Full => "Baterai Penuh (100%)",
                                    BatteryState::Charging(_) => "Sedang Mengisi Daya (USB-C)",
                                    BatteryState::Discharging(v) if v <= 20 => {
                                        "Baterai Lemah — Segera Isi Daya"
                                    }
                                    BatteryState::Discharging(v) => {
                                        let hrs = (v as f32 * 0.55).round() as u32;
                                        ui.label(
                                            RichText::new(format!("Perkiraan ~{hrs} jam tersisa"))
                                                .monospace()
                                                .size(12.0)
                                                .color(p.accent),
                                        );
                                        "Menggunakan baterai internal"
                                    }
                                    BatteryState::Unknown => {
                                        "Gerakkan mouse atau colok dongle untuk membaca baterai"
                                    }
                                };
                                ui.label(
                                    RichText::new(batt_desc)
                                        .size(11.5)
                                        .color(p.text_secondary),
                                );
                            });
                        });

                        ui.add_space(12.0);
                        let prof = cfg.active_profile_mut();

                        elevated_card(p).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Waktu Tidur Ringan (Sleep)")
                                        .size(13.0)
                                        .strong()
                                        .color(p.text_primary),
                                );
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    badge_pill(
                                        ui,
                                        p.accent_soft,
                                        p.accent,
                                        p.accent,
                                        &format!("{} menit", prof.power.sleep_minutes),
                                    );
                                });
                            });
                            ui.spacing_mut().slider_width = (ui.available_width() - 12.0).max(160.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut prof.power.sleep_minutes, 1..=60)
                                        .show_value(false),
                                )
                                .changed()
                            {
                                res.sync.merge(SyncHint::PowerDebounce);
                            }
                        });

                        ui.add_space(8.0);

                        elevated_card(p).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Waktu Tidur Dalam (Deep Sleep)")
                                        .size(13.0)
                                        .strong()
                                        .color(p.text_primary),
                                );
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    badge_pill(
                                        ui,
                                        p.accent_soft,
                                        p.accent,
                                        p.accent,
                                        &format!("{} menit", prof.power.deep_sleep_minutes),
                                    );
                                });
                            });
                            ui.spacing_mut().slider_width = (ui.available_width() - 12.0).max(160.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut prof.power.deep_sleep_minutes, 1..=60)
                                        .show_value(false),
                                )
                                .changed()
                            {
                                res.sync.merge(SyncHint::PowerDebounce);
                            }
                        });
                    });
                },
            );

            ui.add_space(4.0);

            // Kanan: Tema Tampilan & Diagnostik USB HID
            ui.allocate_ui_with_layout(
                Vec2::new(half_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    luxury_card(p).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                card_title(ui, p, "TAMPILAN & KONEKSI", "Tema UI & USB HID");
                            });
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ghost_button(ui, p, "Cari Ulang USB").clicked() {
                                    res.reconnect = true;
                                }
                            });
                        });

                        ui.add_space(8.0);
                        section_eyebrow(ui, p, "WARNA AKSEN APLIKASI");
                        ui.add_space(5.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(7.0, 7.0);
                            for theme in AccentTheme::ALL {
                                let sel = cfg.accent == theme;
                                let [r, g, b] = theme.rgb();
                                let col = Color32::from_rgb(r, g, b);
                                let btn = egui::Button::new(
                                    RichText::new(theme.label()).size(11.5).strong().color(
                                        if sel {
                                            Color32::from_rgb(8, 10, 15)
                                        } else {
                                            p.text_primary
                                        },
                                    ),
                                )
                                .fill(if sel { col } else { p.bg_elevated })
                                .stroke(Stroke::new(1.2_f32, col))
                                .corner_radius(CornerRadius::same(9));
                                if ui.add(btn).clicked() {
                                    cfg.accent = theme;
                                }
                            }
                        });

                        ui.add_space(10.0);
                        toggle_row(
                            ui,
                            p,
                            "Sinkron Otomatis",
                            "Langsung kirim perubahan ke mouse tanpa perlu klik tombol simpan",
                            &mut cfg.auto_sync,
                        );

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            section_eyebrow(ui, p, "LOG KOMUNIKASI USB HID");
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Bersihkan").clicked() {
                                    res.clear_log = true;
                                }
                            });
                        });

                        Frame::new()
                            .fill(p.bg_input)
                            .stroke(Stroke::new(1.0_f32, p.border_subtle))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::same(9))
                            .show(ui, |ui| {
                                ScrollArea::vertical()
                                    .id_salt("hid_log_scroll")
                                    .max_height(108.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        if status.packet_log.is_empty() {
                                            ui.label(
                                                RichText::new(
                                                    "Menunggu Press Play ATLAS (1D57:2112 / 1D57:FA60)...",
                                                )
                                                .monospace()
                                                .size(10.8)
                                                .color(p.text_muted),
                                            );
                                        } else {
                                            for line in &status.packet_log {
                                                ui.label(
                                                    RichText::new(line)
                                                        .monospace()
                                                        .size(10.5)
                                                        .color(p.text_secondary),
                                                );
                                            }
                                        }
                                    });
                            });
                    });
                },
            );
        });

        ui.add_space(10.0);

        // Bawah: Pengelola Profil
        luxury_card(p).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    card_title(ui, p, "MEMORI PROFIL", "Profil Mouse");
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button(ui, p, "Impor JSON").clicked() {
                        match storage::import_profile_dialog() {
                            Ok(Some(prof)) => {
                                cfg.profiles.push(prof);
                                cfg.active_profile_idx = cfg.profiles.len() - 1;
                                state.show_toast("Profil berhasil diimpor", false);
                                res.sync.merge(SyncHint::Full);
                            }
                            Ok(None) => {}
                            Err(e) => state.show_toast(e, true),
                        }
                    }
                    if ghost_button(ui, p, "Ekspor Profil").clicked() {
                        match storage::export_profile_dialog(cfg.active_profile()) {
                            Ok(Some(_)) => state.show_toast("Profil berhasil diekspor", false),
                            Ok(None) => {}
                            Err(e) => state.show_toast(e, true),
                        }
                    }
                    if danger_button(ui, p, "Reset Bawaan Pabrik").clicked() {
                        let name = cfg.active_profile().name.clone();
                        *cfg.active_profile_mut() = MouseProfile::factory_default(name);
                        state.show_toast("Pengaturan dikembalikan ke bawaan pabrik ATLAS", false);
                        res.sync.merge(SyncHint::Full);
                    }
                });
            });

            ui.add_space(10.0);

            let mut switch_to: Option<usize> = None;
            let mut delete_idx: Option<usize> = None;
            let mut dup_idx: Option<usize> = None;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                for (i, prof) in cfg.profiles.iter().enumerate() {
                    let active = i == cfg.active_profile_idx;
                    Frame::new()
                        .fill(if active { p.accent_soft } else { p.bg_elevated })
                        .stroke(Stroke::new(
                            if active { 1.6_f32 } else { 1.0_f32 },
                            if active { p.accent } else { p.border_subtle },
                        ))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_min_width(220.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&prof.name)
                                        .size(14.0)
                                        .strong()
                                        .color(if active { p.accent } else { p.text_primary }),
                                );
                                if active {
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                        badge_pill(
                                            ui,
                                            p.accent,
                                            p.accent,
                                            Color32::from_rgb(8, 10, 15),
                                            "AKTIF",
                                        );
                                    });
                                }
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} DPI  ·  {} Hz  ·  {}ms",
                                    prof.active_dpi(),
                                    prof.polling_rate.hz(),
                                    prof.debounce_ms
                                ))
                                .monospace()
                                .size(11.2)
                                .color(p.text_secondary),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if !active && ui.small_button("Pakai").clicked() {
                                    switch_to = Some(i);
                                }
                                if ui.small_button("Duplikat").clicked() {
                                    dup_idx = Some(i);
                                }
                                if cfg.profiles.len() > 1
                                    && ui
                                        .small_button(RichText::new("Hapus").color(p.danger))
                                        .clicked()
                                {
                                    delete_idx = Some(i);
                                }
                            });
                        });
                }
            });

            if let Some(i) = switch_to {
                cfg.active_profile_idx = i;
                res.sync.merge(SyncHint::Full);
            }
            if let Some(i) = dup_idx {
                let mut copy = cfg.profiles[i].clone();
                copy.name = format!("{} Salinan", copy.name);
                cfg.profiles.push(copy);
                cfg.active_profile_idx = cfg.profiles.len() - 1;
                res.sync.merge(SyncHint::Full);
            }
            if let Some(i) = delete_idx {
                if cfg.profiles.len() > 1 {
                    cfg.profiles.remove(i);
                    if cfg.active_profile_idx >= cfg.profiles.len() {
                        cfg.active_profile_idx = cfg.profiles.len() - 1;
                    }
                    res.sync.merge(SyncHint::Full);
                }
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_profile_name)
                        .hint_text("Nama profil baru...")
                        .desired_width(210.0),
                );
                if accent_button(ui, p, "+ Buat Profil").clicked() {
                    let trimmed = state.new_profile_name.trim();
                    let name = if trimmed.is_empty() {
                        format!("Profil {}", cfg.profiles.len() + 1)
                    } else {
                        trimmed.to_string()
                    };
                    cfg.profiles.push(MouseProfile::factory_default(name));
                    cfg.active_profile_idx = cfg.profiles.len() - 1;
                    state.new_profile_name.clear();
                    res.sync.merge(SyncHint::Full);
                }

                ui.add_space(10.0);
                if state.renaming_profile {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.rename_profile_buf)
                            .desired_width(165.0),
                    );
                    if ghost_button(ui, p, "Simpan Nama").clicked() {
                        let t = state.rename_profile_buf.trim();
                        if !t.is_empty() {
                            cfg.active_profile_mut().name = t.to_string();
                        }
                        state.renaming_profile = false;
                    }
                } else if ghost_button(ui, p, "Ganti Nama Profil").clicked() {
                    state.rename_profile_buf = cfg.active_profile().name.clone();
                    state.renaming_profile = true;
                }
            });
        });
    });

    res
}
