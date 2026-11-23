use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Frame, Margin, Pos2,
    Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::AccentTheme;

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_canvas: Color32,
    pub bg_topbar: Color32,
    pub bg_card: Color32,
    pub bg_elevated: Color32,
    pub bg_input: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

impl Palette {
    pub fn from_accent(theme: AccentTheme) -> Self {
        let [r, g, b] = theme.rgb();
        let accent = Color32::from_rgb(r, g, b);
        Self {
            bg_canvas: Color32::from_rgb(8, 9, 13),
            bg_topbar: Color32::from_rgb(12, 14, 20),
            bg_card: Color32::from_rgb(15, 18, 26),
            bg_elevated: Color32::from_rgb(22, 26, 37),
            bg_input: Color32::from_rgb(10, 12, 18),
            border_subtle: Color32::from_rgb(31, 37, 52),
            border_strong: Color32::from_rgb(54, 64, 88),
            text_primary: Color32::from_rgb(244, 247, 253),
            text_secondary: Color32::from_rgb(156, 166, 190),
            text_muted: Color32::from_rgb(94, 105, 130),
            accent,
            accent_soft: Color32::from_rgba_unmultiplied(r, g, b, 30),
            success: Color32::from_rgb(48, 226, 144),
            warning: Color32::from_rgb(246, 185, 59),
            danger: Color32::from_rgb(255, 76, 104),
        }
    }
}

/// Loads native Windows 11 Segoe UI + Cascadia Mono for ultra-crisp modern typography,
/// falling back cleanly to egui's built-in fonts for Unicode glyphs.
pub fn install_modern_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    if let Ok(segoe) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe_ui".into(), Arc::new(FontData::from_owned(segoe)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".into());
    }

    if let Ok(mono) = std::fs::read(r"C:\Windows\Fonts\CascadiaMono.ttf")
        .or_else(|_| std::fs::read(r"C:\Windows\Fonts\consola.ttf"))
    {
        fonts
            .font_data
            .insert("cascadia_mono".into(), Arc::new(FontData::from_owned(mono)));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "cascadia_mono".into());
    }

    ctx.set_fonts(fonts);
}

pub fn apply_luxury_theme(ctx: &egui::Context, p: Palette) {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.override_text_color = Some(p.text_primary);
    visuals.panel_fill = p.bg_canvas;
    visuals.window_fill = p.bg_card;
    visuals.extreme_bg_color = p.bg_input;
    visuals.faint_bg_color = p.bg_elevated;
    visuals.code_bg_color = p.bg_input;
    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.2_f32, p.accent);
    visuals.hyperlink_color = p.accent;
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.window_stroke = Stroke::new(1.0_f32, p.border_subtle);

    visuals.widgets.noninteractive.bg_fill = p.bg_card;
    visuals.widgets.noninteractive.weak_bg_fill = p.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_secondary);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    visuals.widgets.inactive.bg_fill = p.bg_elevated;
    visuals.widgets.inactive.weak_bg_fill = p.bg_elevated;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(10);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 36, 51);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(30, 36, 51);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.1_f32, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, p.text_primary);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(10);

    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.weak_bg_fill = p.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.4_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.text_primary);
    visuals.widgets.active.corner_radius = CornerRadius::same(10);

    visuals.widgets.open.bg_fill = p.bg_elevated;
    visuals.widgets.open.weak_bg_fill = p.bg_elevated;
    visuals.widgets.open.bg_stroke = Stroke::new(1.1_f32, p.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.open.corner_radius = CornerRadius::same(10);

    visuals.slider_trailing_fill = true;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = Vec2::new(10.0, 9.0);
    style.spacing.button_padding = Vec2::new(13.0, 7.0);
    style.spacing.slider_width = 180.0;
    style.spacing.scroll.bar_width = 5.0;
    style.spacing.scroll.floating = true;
    ctx.set_style(style);
}

pub fn luxury_card(p: Palette) -> Frame {
    Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(18))
}

pub fn elevated_card(p: Palette) -> Frame {
    Frame::new()
        .fill(p.bg_elevated)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(12))
}

pub fn section_eyebrow(ui: &mut Ui, p: Palette, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(10.2)
            .strong()
            .color(p.text_muted),
    );
}

pub fn card_title(ui: &mut Ui, p: Palette, eyebrow: &str, title: &str) {
    section_eyebrow(ui, p, eyebrow);
    ui.add_space(1.0);
    ui.label(
        egui::RichText::new(title)
            .size(16.5)
            .strong()
            .color(p.text_primary),
    );
}

/// Smooth spring-animated pill switch.
pub fn pill_toggle(ui: &mut Ui, p: Palette, on: &mut bool) -> Response {
    let desired_size = Vec2::new(44.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.14);
    if ui.is_rect_visible(rect) {
        let radius = rect.height() * 0.5;
        let bg = lerp_color(
            Color32::from_rgb(34, 40, 56),
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 230),
            how_on,
        );
        let border = if response.hovered() || *on {
            p.accent
        } else {
            p.border_strong
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(radius as u8),
            bg,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let knob_center = Pos2::new(knob_x, rect.center().y);
        ui.painter()
            .circle_filled(knob_center, radius - 3.4, Color32::WHITE);
    }
    response
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

pub fn badge_pill(ui: &mut Ui, bg: Color32, border: Color32, fg: Color32, text: &str) {
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(11.2).strong().color(fg));
        });
}

pub fn accent_button(ui: &mut Ui, p: Palette, label: &str) -> Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(12.5)
            .strong()
            .color(Color32::from_rgb(8, 10, 15)),
    )
    .fill(p.accent)
    .stroke(Stroke::new(1.0_f32, p.accent))
    .corner_radius(CornerRadius::same(10))
    .min_size(Vec2::new(0.0, 34.0));
    ui.add(btn)
}

pub fn ghost_button(ui: &mut Ui, p: Palette, label: &str) -> Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(12.2)
            .strong()
            .color(p.text_primary),
    )
    .fill(p.bg_elevated)
    .stroke(Stroke::new(1.0_f32, p.border_subtle))
    .corner_radius(CornerRadius::same(10))
    .min_size(Vec2::new(0.0, 32.0));
    ui.add(btn)
}

pub fn danger_button(ui: &mut Ui, p: Palette, label: &str) -> Response {
    let btn = egui::Button::new(egui::RichText::new(label).size(12.0).strong().color(p.danger))
        .fill(Color32::from_rgba_unmultiplied(255, 76, 104, 22))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(255, 76, 104, 110),
        ))
        .corner_radius(CornerRadius::same(10))
        .min_size(Vec2::new(0.0, 30.0));
    ui.add(btn)
}

/// Animated circular battery ring with subtle pulse when charging.
pub fn battery_ring_gauge(
    ui: &mut Ui,
    p: Palette,
    percent: Option<u8>,
    charging: bool,
    size: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.42;
    let stroke_w = (size * 0.08).clamp(6.0, 11.0);
    let time = ui.input(|i| i.time) as f32;

    painter.circle_stroke(center, radius, Stroke::new(stroke_w, p.bg_elevated));

    let pct_val = percent.unwrap_or(0) as f32;
    let anim_frac = ui
        .ctx()
        .animate_value_with_time(ui.id().with("batt_ring_anim"), (pct_val / 100.0).clamp(0.0, 1.0), 0.35);

    let ring_color = if charging {
        p.accent
    } else if pct_val <= 20.0 && percent.is_some() {
        p.danger
    } else if pct_val <= 45.0 && percent.is_some() {
        p.warning
    } else if percent.is_some() {
        p.success
    } else {
        p.border_strong
    };

    if anim_frac > 0.005 {
        let steps = 72;
        let max_step = ((steps as f32) * anim_frac).ceil() as usize;
        let start_angle = -std::f32::consts::FRAC_PI_2;
        let mut pts = Vec::with_capacity(max_step + 1);
        for i in 0..=max_step {
            let t = (i as f32 / steps as f32).min(anim_frac);
            let a = start_angle + t * std::f32::consts::TAU;
            pts.push(Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin()));
        }
        let pulse = if charging {
            28.0 + 22.0 * (time * 3.0).sin().abs()
        } else {
            34.0
        };
        painter.add(egui::Shape::line(
            pts.clone(),
            Stroke::new(
                stroke_w + 6.0,
                Color32::from_rgba_unmultiplied(
                    ring_color.r(),
                    ring_color.g(),
                    ring_color.b(),
                    pulse as u8,
                ),
            ),
        ));
        painter.add(egui::Shape::line(pts, Stroke::new(stroke_w, ring_color)));
    }

    let main_txt = match percent {
        Some(v) => format!("{v}%"),
        None => "--".into(),
    };
    painter.text(
        center - Vec2::new(0.0, 7.0),
        egui::Align2::CENTER_CENTER,
        main_txt,
        egui::FontId::monospace(size * 0.22),
        p.text_primary,
    );
    let sub_txt = if charging {
        "CHARGING"
    } else if percent.is_some() {
        "LI-PO CELL"
    } else {
        "STANDBY"
    };
    painter.text(
        center + Vec2::new(0.0, size * 0.15),
        egui::Align2::CENTER_CENTER,
        sub_txt,
        egui::FontId::proportional((size * 0.082).max(9.5)),
        if charging { p.accent } else { p.text_muted },
    );
}

pub fn toggle_row(
    ui: &mut Ui,
    p: Palette,
    title: &str,
    subtitle: &str,
    value: &mut bool,
) -> bool {
    let mut changed = false;
    elevated_card(p).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(13.5)
                        .strong()
                        .color(p.text_primary),
                );
                ui.label(
                    egui::RichText::new(subtitle)
                        .size(11.4)
                        .color(p.text_secondary),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if pill_toggle(ui, p, value).changed() {
                    changed = true;
                }
            });
        });
    });
    changed
}
