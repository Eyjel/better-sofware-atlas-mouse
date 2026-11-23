use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, PointerButton, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};

use crate::model::{MacroDef, MouseProfile, PHYSICAL_BUTTONS};
use crate::ui::theme::{lerp_color, Palette};

/// Tracks live physical mouse button presses, click ripples, and scroll wheel motion
/// so the on-screen ATLAS mouse reacts instantaneously to your real hardware!
pub struct LiveMouseFeedback {
    pub pressed_phys: [bool; 6],
    pub last_click_at: [Option<Instant>; 6],
    pub scroll_phase: f32,
    pub last_scroll_at: Option<Instant>,
    pub scroll_dir: f32,
}

impl Default for LiveMouseFeedback {
    fn default() -> Self {
        Self {
            pressed_phys: [false; 6],
            last_click_at: [None; 6],
            scroll_phase: 0.0,
            last_scroll_at: None,
            scroll_dir: 0.0,
        }
    }
}

impl LiveMouseFeedback {
    pub fn sample_input(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        ctx.input(|i| {
            let mappings = [
                (0usize, PointerButton::Primary),
                (1usize, PointerButton::Secondary),
                (2usize, PointerButton::Middle),
                (3usize, PointerButton::Extra2), // Side Forward
                (4usize, PointerButton::Extra1), // Side Backward
            ];
            for (idx, btn) in mappings {
                let down = i.pointer.button_down(btn);
                if down && !self.pressed_phys[idx] {
                    self.last_click_at[idx] = Some(now);
                }
                self.pressed_phys[idx] = down;
            }
            let scroll_y = i.raw_scroll_delta.y;
            if scroll_y.abs() > 0.1 {
                self.scroll_phase = (self.scroll_phase + scroll_y * 0.18).rem_euclid(100.0);
                self.scroll_dir = scroll_y.signum();
                self.last_scroll_at = Some(now);
            }
        });
    }

    pub fn ripple_progress(&self, idx: usize) -> Option<f32> {
        let t = self.last_click_at[idx]?.elapsed().as_secs_f32();
        if t < 0.42 {
            Some(t / 0.42)
        } else {
            None
        }
    }

    pub fn is_animating(&self) -> bool {
        self.pressed_phys.iter().any(|&b| b)
            || (0..6).any(|i| self.ripple_progress(i).is_some())
            || self
                .last_scroll_at
                .map(|t| t.elapsed().as_secs_f32() < 0.3)
                .unwrap_or(false)
    }
}

/// Renders a stealth, studio-lit, interactive Press Play ATLAS ergonomic mouse.
/// - No fake RGB bar (pure stealth matte black matching the real ATLAS!)
/// - Interactive 3D studio light reflection tracking cursor movement
/// - Live click depression + expanding ripple rings + animated scroll wheel ridges!
pub fn draw_atlas_mouse_studio(
    ui: &mut Ui,
    p: Palette,
    profile: &MouseProfile,
    macros: &[MacroDef],
    selected_btn_idx: usize,
    is_connected: bool,
    live: &mut LiveMouseFeedback,
) -> Option<usize> {
    live.sample_input(ui.ctx());
    if live.is_animating() {
        ui.ctx().request_repaint();
    }

    let mut clicked_btn: Option<usize> = None;

    let avail = ui.available_size();
    let canvas_w = avail.x.max(480.0);
    let canvas_h = avail.y.clamp(450.0, 580.0);
    let (canvas, canvas_resp) =
        ui.allocate_exact_size(Vec2::new(canvas_w, canvas_h), Sense::hover());

    let painter = ui.painter_at(canvas);
    let center = Pos2::new(canvas.center().x, canvas.center().y - 2.0);

    // Subtle cursor-driven studio light parallax (gives physical matte-shell depth!)
    let mut light_offset = Vec2::ZERO;
    if let Some(pos) = canvas_resp.hover_pos().or_else(|| ui.input(|i| i.pointer.latest_pos())) {
        let dx = ((pos.x - center.x) / (canvas_w * 0.5)).clamp(-1.0, 1.0);
        let dy = ((pos.y - center.y) / (canvas_h * 0.5)).clamp(-1.0, 1.0);
        light_offset = Vec2::new(dx * 16.0, dy * 12.0);
    }

    // Precision dark studio blueprint grid
    let dot_col = Color32::from_rgba_unmultiplied(130, 148, 186, 15);
    let step = 22.0;
    let mut gx = canvas.left() + 14.0;
    while gx < canvas.right() - 10.0 {
        let mut gy = canvas.top() + 14.0;
        while gy < canvas.bottom() - 28.0 {
            painter.circle_filled(Pos2::new(gx, gy), 0.95, dot_col);
            gy += step;
        }
        gx += step;
    }

    // Subtle concentric studio radar rings behind mouse
    for (rad, alpha) in [(165.0_f32, 16u8), (118.0_f32, 22u8)] {
        painter.circle_stroke(
            center,
            rad,
            Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
            ),
        );
    }

    // Soft monochrome studio spotlight (moves gently with cursor)
    for (r_mult, alpha) in [(1.0_f32, 10u8), (0.68_f32, 16u8), (0.42_f32, 22u8)] {
        painter.circle_filled(
            center + light_offset * 0.45,
            178.0 * r_mult,
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
        );
    }

    // Mouse geometry scaling (Press Play ATLAS ergonomic right-handed silhouette)
    let scale = (canvas_h / 475.0).clamp(0.84, 1.10);
    let mw = 178.0 * scale;
    let mh = 328.0 * scale;
    let mouse_rect = Rect::from_center_size(center, Vec2::new(mw, mh));

    let pt = |nx: f32, ny: f32| -> Pos2 {
        Pos2::new(center.x + nx * (mw * 0.5), center.y + ny * (mh * 0.5))
    };

    let shell_norm: &[(f32, f32)] = &[
        (-0.56, -0.95),
        (-0.32, -0.995),
        (-0.14, -1.00),
        (0.14, -1.00),
        (0.36, -0.99),
        (0.58, -0.93),
        (0.76, -0.80),
        (0.86, -0.60),
        (0.90, -0.34),
        (0.87, -0.05),
        (0.91, 0.25),
        (0.95, 0.52),
        (0.91, 0.74),
        (0.76, 0.89),
        (0.48, 0.97),
        (0.15, 1.00),
        (-0.22, 0.99),
        (-0.56, 0.92),
        (-0.80, 0.78),
        (-0.92, 0.56),
        (-0.94, 0.30),
        (-0.86, 0.02),
        (-0.82, -0.24),
        (-0.86, -0.52),
        (-0.80, -0.76),
        (-0.69, -0.89),
    ];

    let shell_pts: Vec<Pos2> = shell_norm.iter().map(|&(x, y)| pt(x, y)).collect();

    // Multi-layered soft cast desk shadow
    for (spread, dy, alpha) in [(1.07_f32, 16.0_f32, 60u8), (1.035_f32, 9.0_f32, 115u8)] {
        let shadow_pts: Vec<Pos2> = shell_norm
            .iter()
            .map(|&(x, y)| {
                pt(x * spread, y * (spread - 0.015))
                    + Vec2::new(-light_offset.x * 0.2, dy * scale - light_offset.y * 0.15)
            })
            .collect();
        painter.add(Shape::convex_polygon(
            shadow_pts,
            Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
            Stroke::NONE,
        ));
    }

    // Main stealth matte-obsidian body
    painter.add(Shape::convex_polygon(
        shell_pts,
        Color32::from_rgb(24, 27, 34),
        Stroke::new(1.8_f32, Color32::from_rgb(66, 74, 94)),
    ));

    // Inner sculpted shell layer
    let inner_pts: Vec<Pos2> = shell_norm
        .iter()
        .map(|&(x, y)| pt(x * 0.91, y * 0.935))
        .collect();
    painter.add(Shape::convex_polygon(
        inner_pts,
        Color32::from_rgb(31, 35, 44),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 13)),
    ));

    // Dynamic cursor-tracking studio specular reflection across matte palm dome
    let spec_center = pt(-0.02, 0.38) + light_offset * 0.55;
    let crest_rect = Rect::from_center_size(spec_center, Vec2::new(mw * 0.58, mh * 0.34));
    painter.rect_filled(
        crest_rect,
        CornerRadius::same((mw * 0.28) as u8),
        Color32::from_rgba_unmultiplied(255, 255, 255, 9),
    );

    // Minimalist stealth laser-etched branding on palm rest
    painter.text(
        pt(0.0, 0.55),
        Align2::CENTER_CENTER,
        "A T L A S",
        FontId::proportional(11.5 * scale),
        Color32::from_rgba_unmultiplied(188, 198, 224, 48),
    );
    painter.text(
        pt(0.0, 0.65),
        Align2::CENTER_CENTER,
        "ULTRALIGHT · PAW3311",
        FontId::monospace(7.8 * scale),
        Color32::from_rgba_unmultiplied(150, 165, 200, 34),
    );

    // Hit rectangles
    let lmb_rect = Rect::from_min_max(pt(-0.80, -0.94), pt(-0.18, -0.10));
    let rmb_rect = Rect::from_min_max(pt(0.18, -0.94), pt(0.81, -0.10));
    let channel_rect = Rect::from_min_max(pt(-0.175, -0.97), pt(0.175, -0.34));
    let wheel_rect = Rect::from_min_max(pt(-0.115, -0.80), pt(0.115, -0.45));
    let m4_rect = Rect::from_min_max(pt(-0.99, -0.40), pt(-0.80, -0.11));
    let m5_rect = Rect::from_min_max(pt(-0.99, -0.05), pt(-0.80, 0.25));
    let dpi_pill_rect = Rect::from_center_size(
        Pos2::new(center.x, mouse_rect.bottom() + 26.0),
        Vec2::new(168.0, 28.0),
    );

    let hit_rects = [lmb_rect, rmb_rect, wheel_rect, m4_rect, m5_rect, dpi_pill_rect];
    let mut hovered_idx: Option<usize> = None;

    for (idx, &hr) in hit_rects.iter().enumerate() {
        let resp = ui.interact(hr, ui.id().with(("mouse_zone", idx)), Sense::click());
        if resp.hovered() {
            hovered_idx = Some(idx);
        }
        if resp.clicked() {
            clicked_btn = Some(idx);
        }
    }

    // Crisp separated paddle seam lines
    let seam_col = Color32::from_rgb(11, 13, 18);
    painter.line_segment([pt(-0.84, -0.09), pt(0.86, -0.09)], Stroke::new(2.1_f32, seam_col));
    painter.line_segment([pt(0.0, -0.34), pt(0.0, -0.09)], Stroke::new(2.1_f32, seam_col));

    // LMB & RMB interactive surfaces (physically react to live clicks + smooth hover)
    for (idx, rect) in [(0usize, lmb_rect), (1usize, rmb_rect)] {
        let is_sel = selected_btn_idx == idx;
        let is_hov = hovered_idx == Some(idx);
        let is_pressed = live.pressed_phys[idx];
        let anim = ui
            .ctx()
            .animate_bool_with_time(ui.id().with(("pad_anim", idx)), is_sel || is_hov || is_pressed, 0.12);
        if anim > 0.01 {
            let draw_r = if is_pressed { rect.shrink(1.5) } else { rect };
            let alpha = if is_pressed {
                88
            } else if is_sel {
                (44.0 * anim) as u8
            } else {
                (26.0 * anim) as u8
            };
            painter.rect_filled(
                draw_r,
                CornerRadius::same((16.0 * scale) as u8),
                Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
            );
            painter.rect_stroke(
                draw_r,
                CornerRadius::same((16.0 * scale) as u8),
                Stroke::new(if is_sel || is_pressed { 1.6_f32 } else { 1.0_f32 }, p.accent),
                StrokeKind::Inside,
            );
        }
    }

    // Glossy stealth center scroll channel (no RGB bar!)
    painter.rect(
        channel_rect,
        CornerRadius::same((16.0 * scale) as u8),
        Color32::from_rgb(13, 15, 21),
        Stroke::new(1.0_f32, Color32::from_rgb(52, 58, 76)),
        StrokeKind::Inside,
    );

    // Subtle matte hardware connection micro-pinhole (clean minimalist dot, no RGB)
    let pin_pos = pt(0.0, -0.89);
    let pin_col = if is_connected {
        Color32::from_rgba_unmultiplied(p.success.r(), p.success.g(), p.success.b(), 190)
    } else {
        Color32::from_rgb(42, 48, 62)
    };
    painter.circle_filled(pin_pos, 2.6 * scale, pin_col);

    // Animated ribbed rubber scroll wheel (Button 3)
    let wheel_active =
        selected_btn_idx == 2 || hovered_idx == Some(2) || live.pressed_phys[2];
    let scrolling_now = live
        .last_scroll_at
        .map(|t| t.elapsed().as_secs_f32() < 0.25)
        .unwrap_or(false);
    let wheel_draw = if live.pressed_phys[2] {
        wheel_rect.shrink(1.2)
    } else {
        wheel_rect
    };
    painter.rect(
        wheel_draw,
        CornerRadius::same((9.0 * scale) as u8),
        Color32::from_rgb(22, 25, 33),
        Stroke::new(
            if wheel_active || scrolling_now { 1.6_f32 } else { 1.1_f32 },
            if wheel_active || scrolling_now {
                p.accent
            } else {
                Color32::from_rgb(74, 82, 104)
            },
        ),
        StrokeKind::Inside,
    );

    // Moving rubber scroll notches driven by live.scroll_phase!
    let notch_count = 8;
    let phase_frac = (live.scroll_phase * 0.22).fract();
    for r in 0..notch_count {
        let mut t = (r as f32 + phase_frac) / (notch_count as f32);
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 0.08 || t > 0.92 {
            continue;
        }
        let y = egui::lerp(wheel_draw.top()..=wheel_draw.bottom(), t);
        let notch_col = if scrolling_now {
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 180)
        } else {
            Color32::from_rgb(54, 62, 78)
        };
        painter.line_segment(
            [
                Pos2::new(wheel_draw.left() + 3.0, y),
                Pos2::new(wheel_draw.right() - 3.0, y),
            ],
            Stroke::new(1.5_f32, notch_col),
        );
    }

    // Side thumb buttons (M4 = idx 3, M5 = idx 4) with live press depression
    for (idx, rect) in [(3usize, m4_rect), (4usize, m5_rect)] {
        let is_sel = selected_btn_idx == idx;
        let is_hov = hovered_idx == Some(idx);
        let is_down = live.pressed_phys[idx];
        let draw_r = if is_down {
            rect.translate(Vec2::new(2.0, 0.0))
        } else {
            rect
        };
        let fill = if is_down || is_sel {
            p.accent
        } else if is_hov {
            Color32::from_rgb(80, 92, 118)
        } else {
            Color32::from_rgb(46, 52, 66)
        };
        painter.rect(
            draw_r,
            CornerRadius::same((6.0 * scale) as u8),
            fill,
            Stroke::new(
                1.2_f32,
                if is_sel || is_hov || is_down {
                    p.accent
                } else {
                    Color32::from_rgb(78, 86, 108)
                },
            ),
            StrokeKind::Inside,
        );
    }

    // Floating glass callout pills + smooth animated leader lines
    let anchors: [(usize, Pos2, Pos2, bool); 5] = [
        (0, pt(-0.48, -0.56), Pos2::new(canvas.left() + 14.0, pt(0.0, -0.64).y), true),
        (3, pt(-0.90, -0.25), Pos2::new(canvas.left() + 14.0, pt(0.0, -0.18).y), true),
        (4, pt(-0.90, 0.10),  Pos2::new(canvas.left() + 14.0, pt(0.0, 0.28).y),  true),
        (2, pt(0.0, -0.62),   Pos2::new(canvas.right() - 14.0, pt(0.0, -0.64).y), false),
        (1, pt(0.50, -0.45),  Pos2::new(canvas.right() - 14.0, pt(0.0, -0.18).y), false),
    ];

    let pill_w = ((canvas_w - mw) * 0.5 - 38.0).clamp(128.0, 186.0);
    let pill_h = 50.0;

    for (phys_idx, node_pos, edge_pos, is_left) in anchors {
        let info = PHYSICAL_BUTTONS[phys_idx];
        let binding_text = profile.buttons[info.slot].display_label(macros);
        let is_sel = selected_btn_idx == phys_idx;
        let is_pressed = live.pressed_phys[phys_idx];

        let pill_rect = if is_left {
            Rect::from_min_size(
                Pos2::new(edge_pos.x, edge_pos.y - pill_h * 0.5),
                Vec2::new(pill_w, pill_h),
            )
        } else {
            Rect::from_min_size(
                Pos2::new(edge_pos.x - pill_w, edge_pos.y - pill_h * 0.5),
                Vec2::new(pill_w, pill_h),
            )
        };

        let pill_resp =
            ui.interact(pill_rect, ui.id().with(("callout_pill", phys_idx)), Sense::click());
        let active = is_sel || hovered_idx == Some(phys_idx) || pill_resp.hovered() || is_pressed;
        if pill_resp.clicked() {
            clicked_btn = Some(phys_idx);
        }

        let anim = ui
            .ctx()
            .animate_bool_with_time(ui.id().with(("pill_anim", phys_idx)), active, 0.13);

        // Expanding click shockwave ripple on the hardware node!
        if let Some(rp) = live.ripple_progress(phys_idx) {
            let ripple_r = 8.0 + rp * 26.0;
            let ripple_a = ((1.0 - rp) * 190.0) as u8;
            painter.circle_stroke(
                node_pos,
                ripple_r,
                Stroke::new(
                    2.0_f32,
                    Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), ripple_a),
                ),
            );
        }

        // Leader line
        let line_end = if is_left {
            Pos2::new(pill_rect.right(), pill_rect.center().y)
        } else {
            Pos2::new(pill_rect.left(), pill_rect.center().y)
        };
        let elbow = Pos2::new(
            if is_left { line_end.x + 15.0 } else { line_end.x - 15.0 },
            line_end.y,
        );
        let line_col = lerp_color(
            Color32::from_rgba_unmultiplied(100, 114, 146, 110),
            p.accent,
            anim,
        );
        let line_w = 1.0 + 0.8 * anim;
        painter.line_segment([line_end, elbow], Stroke::new(line_w, line_col));
        painter.line_segment([elbow, node_pos], Stroke::new(line_w, line_col));

        // Node dot on the mouse
        if anim > 0.05 {
            painter.circle_filled(
                node_pos,
                11.5,
                Color32::from_rgba_unmultiplied(
                    p.accent.r(),
                    p.accent.g(),
                    p.accent.b(),
                    (58.0 * anim) as u8,
                ),
            );
        }
        painter.circle_filled(
            node_pos,
            7.2,
            lerp_color(Color32::from_rgb(18, 22, 32), p.accent, anim),
        );
        painter.circle_stroke(
            node_pos,
            7.2,
            Stroke::new(1.4_f32, lerp_color(p.border_strong, Color32::WHITE, anim)),
        );
        painter.text(
            node_pos,
            Align2::CENTER_CENTER,
            format!("{}", info.id),
            FontId::monospace(9.5),
            if anim > 0.5 {
                Color32::from_rgb(8, 10, 15)
            } else {
                p.text_primary
            },
        );

        // Glass callout pill with animated elevation & accent bar
        let card_fill = lerp_color(
            Color32::from_rgba_unmultiplied(15, 18, 27, 238),
            Color32::from_rgb(23, 29, 44),
            anim,
        );
        let card_border = lerp_color(p.border_subtle, p.accent, anim);
        painter.rect(
            pill_rect,
            CornerRadius::same(11),
            card_fill,
            Stroke::new(if is_sel || is_pressed { 1.6_f32 } else { 1.0_f32 }, card_border),
            StrokeKind::Inside,
        );

        if is_sel || is_pressed {
            let accent_bar = Rect::from_min_size(
                Pos2::new(
                    if is_left { pill_rect.left() } else { pill_rect.right() - 3.0 },
                    pill_rect.top() + 9.0,
                ),
                Vec2::new(3.0, pill_rect.height() - 18.0),
            );
            painter.rect_filled(accent_bar, CornerRadius::same(2), p.accent);
        }

        let title_pos = Pos2::new(pill_rect.left() + 12.0, pill_rect.top() + 15.0);
        let val_pos = Pos2::new(pill_rect.left() + 12.0, pill_rect.top() + 34.0);
        painter.text(
            title_pos,
            Align2::LEFT_CENTER,
            format!("{} · {}", info.short_name, info.full_name),
            FontId::proportional(10.0),
            if is_sel || is_pressed { p.accent } else { p.text_muted },
        );
        let truncated = if binding_text.chars().count() > 21 {
            let prefix: String = binding_text.chars().take(19).collect();
            format!("{prefix}...")
        } else {
            binding_text
        };
        painter.text(
            val_pos,
            Align2::LEFT_CENTER,
            truncated,
            FontId::proportional(12.5),
            p.text_primary,
        );
    }

    // Bottom DPI Switch pill (Slot 3)
    let dpi_sel = selected_btn_idx == 5 || hovered_idx == Some(5);
    let dpi_label = profile.buttons[PHYSICAL_BUTTONS[5].slot].display_label(macros);
    painter.rect(
        dpi_pill_rect,
        CornerRadius::same(14),
        if dpi_sel { p.accent_soft } else { p.bg_elevated },
        Stroke::new(
            if dpi_sel { 1.5_f32 } else { 1.0_f32 },
            if dpi_sel { p.accent } else { p.border_subtle },
        ),
        StrokeKind::Inside,
    );
    painter.text(
        dpi_pill_rect.center(),
        Align2::CENTER_CENTER,
        format!("Tombol Bawah · {dpi_label}"),
        FontId::proportional(11.2),
        if dpi_sel { p.text_primary } else { p.text_secondary },
    );

    // Interactive live test hint at bottom of canvas
    let any_live = live.pressed_phys.iter().any(|&b| b) || scrolling_now;
    let hint_txt = if scrolling_now {
        if live.scroll_dir > 0.0 {
            "TES LANGSUNG · Scroll Ke Atas Terdeteksi"
        } else {
            "TES LANGSUNG · Scroll Ke Bawah Terdeteksi"
        }
    } else if let Some(pi) = live.pressed_phys.iter().position(|&b| b) {
        match pi {
            0 => "TES LANGSUNG · Klik Kiri (LMB) Ditekan",
            1 => "TES LANGSUNG · Klik Kanan (RMB) Ditekan",
            2 => "TES LANGSUNG · Klik Tengah (MMB) Ditekan",
            3 => "TES LANGSUNG · Tombol Samping Maju (M4) Ditekan",
            4 => "TES LANGSUNG · Tombol Samping Mundur (M5) Ditekan",
            _ => "TES LANGSUNG · Aktif",
        }
    } else {
        "INTERAKTIF · Klik tombol pada gambar untuk atur, atau tekan mouse fisik untuk tes"
    };
    painter.text(
        Pos2::new(canvas.center().x, canvas.bottom() - 8.0),
        Align2::CENTER_CENTER,
        hint_txt,
        FontId::proportional(10.5),
        if any_live { p.accent } else { p.text_muted },
    );

    clicked_btn
}
