use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    AnyElement, App, Background, BorderStyle, Bounds, Corners, Edges, Global, Hsla, IntoElement,
    Path, Pixels, Styled, Window, canvas, linear_color_stop, linear_gradient, outline, point, px,
    quad, size, transparent_black,
};
use theme::{ActiveTheme as _, Appearance};

pub const GLASS_THEME_SUFFIX: &str = "#gradient-glass";

pub type Rgb = [u8; 3];
pub type FourStops = [Rgb; 4];

const DEFAULT_STOPS: FourStops = [
    [234, 67, 53],
    [251, 188, 4],
    [52, 168, 83],
    [66, 133, 244],
];

/// Shared lightweight snapshot of Gradient Zed's live palette and synchronized Breathing Aurora
/// clock, accessible from `ui`, `markdown`, `editor`, `project_panel`, `sidebar`, and `agent_ui`
/// without introducing circular dependencies on `gradient_studio`.
#[derive(Clone)]
pub struct GradientChromeState {
    pub enabled: bool,
    pub stops: FourStops,
    pub breathe: bool,
    pub breathe_origin: Instant,
    pub breathe_paused_delta: Option<f32>,
    pub breathe_color_offset: f32,
    pub breathe_cycle_secs: f32,
    pub breathe_pulse: bool,
    pub breathe_shimmer: bool,
    pub breathe_auto_morph: bool,
    pub intensity: f32,
    pub canvas_bg: Hsla,
    pub is_light: bool,
    pub morph_palettes: Vec<FourStops>,
    pub raw_theme: Option<Arc<theme::Theme>>,
}

impl Global for GradientChromeState {}

impl Default for GradientChromeState {
    fn default() -> Self {
        Self {
            enabled: false,
            stops: DEFAULT_STOPS,
            breathe: false,
            breathe_origin: Instant::now(),
            breathe_paused_delta: None,
            breathe_color_offset: 0.0,
            breathe_cycle_secs: 18.0,
            breathe_pulse: true,
            breathe_shimmer: true,
            breathe_auto_morph: false,
            intensity: 1.0,
            canvas_bg: gpui::rgb(0x111722).into(),
            is_light: false,
            morph_palettes: Vec::new(),
            raw_theme: None,
        }
    }
}

#[inline]
pub fn rgb_to_hsla([r, g, b]: Rgb, alpha: f32) -> Hsla {
    let mut c: Hsla = gpui::Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
    .into();
    c.a = alpha.clamp(0.0, 1.0);
    c
}

#[inline]
fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t).round() as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t).round() as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t).round() as u8,
    ]
}

#[inline]
pub fn lerp_stops(a: FourStops, b: FourStops, t: f32) -> FourStops {
    [
        lerp_rgb(a[0], b[0], t),
        lerp_rgb(a[1], b[1], t),
        lerp_rgb(a[2], b[2], t),
        lerp_rgb(a[3], b[3], t),
    ]
}

/// Continuously shifts 4 rainbow stops around the cycle by `shift` in `[0.0..4.0)` so frames,
/// borders, buttons, tabs, and selections smoothly shimmer in lockstep with Breathing Aurora.
#[inline]
pub fn shimmer_shift_stops(stops: FourStops, shift: f32) -> FourStops {
    let s = shift.rem_euclid(4.0);
    let base = s.floor() as usize;
    let frac = s - s.floor();
    let mut out = stops;
    for i in 0..4 {
        let a = stops[(i + base) % 4];
        let b = stops[(i + base + 1) % 4];
        out[i] = lerp_rgb(a, b, frac);
    }
    out
}

/// Samples the Auto-Morph palette ring starting seamlessly from `base` at `delta == 0.0`
/// so enabling Breathe or Auto-Morph never jumps to `CURATED_PALETTES[0]` (prism rainbow).
pub fn sample_morph_ring(base: FourStops, ring: &[FourStops], delta: f32) -> FourStops {
    if ring.is_empty() {
        return base;
    }
    let d = delta.rem_euclid(1.0);
    if let Some(start_idx) = ring.iter().position(|&p| p == base) {
        let count = ring.len();
        let pos = d * count as f32;
        let step = (pos.floor() as usize) % count;
        let idx_a = (start_idx + step) % count;
        let idx_b = (idx_a + 1) % count;
        let frac = pos - pos.floor();
        lerp_stops(ring[idx_a], ring[idx_b], frac)
    } else {
        let total = ring.len() + 1;
        let pos = d * total as f32;
        let slot_a = (pos.floor() as usize) % total;
        let slot_b = (slot_a + 1) % total;
        let frac = pos - pos.floor();
        let stops_a = if slot_a == 0 { base } else { ring[slot_a - 1] };
        let stops_b = if slot_b == 0 { base } else { ring[slot_b - 1] };
        lerp_stops(stops_a, stops_b, frac)
    }
}

impl GradientChromeState {
    pub fn live_phase_delta(&self, _window_active: bool) -> Option<f32> {
        let any_motion = self.breathe_pulse || self.breathe_shimmer || self.breathe_auto_morph;
        if !any_motion {
            return None;
        }
        if self.breathe {
            let cycle = self.breathe_cycle_secs.clamp(5.0, 72.0);
            let base = self.breathe_paused_delta.unwrap_or(0.0);
            let elapsed = self.breathe_origin.elapsed().as_secs_f32() / cycle;
            Some((base + elapsed).rem_euclid(1.0))
        } else {
            self.breathe_paused_delta
        }
    }

    #[inline]
    pub fn color_phase_delta(&self, motion_delta: f32) -> f32 {
        (motion_delta + self.breathe_color_offset).rem_euclid(1.0)
    }

    pub fn live_stops_and_pulse(
        &self,
        override_stops: Option<FourStops>,
        window_active: bool,
    ) -> (FourStops, f32) {
        let base = override_stops.unwrap_or(self.stops);
        let Some(delta) = self.live_phase_delta(window_active) else {
            return (base, 1.0);
        };
        let phase = delta * TAU;
        let color_delta = self.color_phase_delta(delta);

        let mut stops = if self.breathe_auto_morph && !self.morph_palettes.is_empty() {
            sample_morph_ring(base, &self.morph_palettes, color_delta)
        } else {
            base
        };

        if self.breathe_shimmer {
            stops = shimmer_shift_stops(stops, color_delta * 4.0);
        }

        let pulse = if self.breathe_pulse {
            1.0 + 0.16 * (phase * 2.0).sin()
        } else {
            1.0
        };

        (stops, pulse)
    }
}

pub fn is_enabled(cx: &App) -> bool {
    cx.try_global::<GradientChromeState>()
        .is_some_and(|s| s.enabled)
}

/// Evaluates the current live breathing `(stops, pulse)` snapshot for a window.
pub fn live_stops_and_pulse(
    override_stops: Option<FourStops>,
    window: &Window,
    cx: &App,
) -> Option<(FourStops, f32)> {
    let state = cx.try_global::<GradientChromeState>()?;
    if !state.enabled {
        return None;
    }
    Some(state.live_stops_and_pulse(override_stops, window.is_window_active()))
}

pub fn canvas_background(cx: &App) -> Hsla {
    if let Some(state) = cx.try_global::<GradientChromeState>() {
        state.canvas_bg
    } else {
        let fallback: Hsla = match cx.theme().appearance() {
            Appearance::Light => gpui::rgb(0xffffff).into(),
            Appearance::Dark => gpui::rgb(0x111722).into(),
        };
        let mut c = fallback.blend(cx.theme().colors().editor_background);
        c.a = 1.0;
        c
    }
}

/// Returns the unmodified pre-glass theme colors when Gradient Zed's glass theme is active,
/// falling back to `cx.theme().colors()` otherwise.
///
/// Rasterized SVG diagrams (such as Mermaid) blend node fills over `editor_background` and draw
/// edges with `border`; reading glass-transparent structural backgrounds (`transparent_black()`)
/// or translucent glass borders turns light-mode diagram nodes near-black and fades out arrows.
pub fn raw_theme_colors<'a>(cx: &'a App) -> &'a theme::ThemeColors {
    if cx.theme().id.ends_with(GLASS_THEME_SUFFIX)
        && let Some(state) = cx.try_global::<GradientChromeState>()
        && state.enabled
        && let Some(raw) = &state.raw_theme
    {
        return raw.colors();
    }
    cx.theme().colors()
}

/// Computes a crisp opaque connector/arrow stroke color for Mermaid diagrams rendered over
/// Gradient Zed's frosted aurora cards by strengthening the theme's raw border toward `text_muted`.
pub fn mermaid_line_color(border: Hsla, text_muted: Hsla) -> Hsla {
    let b = gpui::Rgba::from(border);
    let m = gpui::Rgba::from(text_muted);
    let t = 0.45;
    gpui::Rgba {
        r: b.r * (1.0 - t) + m.r * t,
        g: b.g * (1.0 - t) + m.g * t,
        b: b.b * (1.0 - t) + m.b * t,
        a: 1.0,
    }
    .into()
}

/// Solid opaque background color for fenced code blocks inside Markdown / Agent responses,
/// keeping code boxes crisp and solid while framed by a rounded gradient border.
pub fn solid_code_block_background(cx: &App) -> Hsla {
    let mut base = canvas_background(cx);
    base.a = 1.0;
    if cx.theme().appearance() == Appearance::Light {
        // Subtle cool-alabaster contrast tint on pure white themes so code cards pop cleanly
        if base.l > 0.96 {
            base.l = (base.l - 0.025).max(0.0);
        }
    } else if base.l < 0.12 {
        base.l = (base.l + 0.025).min(1.0);
    }
    base
}

/// Returns a live-breathing horizontal gradient `Background` for Editor text selections
/// (matching VS Code's `--g-selection`).
pub fn selection_background(
    window: &Window,
    opacity_scale: f32,
    cx: &App,
) -> Option<Background> {
    let state = cx.try_global::<GradientChromeState>()?;
    if !state.enabled {
        return None;
    }
    let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
    let base_a = if state.is_light { 0.28 } else { 0.34 };
    let a0 = (base_a * opacity_scale * pulse).clamp(0.08, 0.52);
    let a1 = ((base_a + 0.04) * opacity_scale * pulse).clamp(0.08, 0.56);
    Some(linear_gradient(
        90.0,
        linear_color_stop(rgb_to_hsla(stops[0], a0), 0.0),
        linear_color_stop(rgb_to_hsla(stops[3], a1), 1.0),
    ))
}

/// Returns a harmonious palette-tinted `Hsla` for Markdown text selections.
pub fn markdown_selection_tint(cx: &App) -> Option<Hsla> {
    let state = cx.try_global::<GradientChromeState>()?;
    if !state.enabled {
        return None;
    }
    let alpha = if state.is_light { 0.26 } else { 0.34 };
    Some(rgb_to_hsla(state.stops[3], alpha))
}

/// Returns a live-breathing palette-tinted `Hsla` for Markdown text selections during layout/paint.
pub fn markdown_selection_tint_live(window: &Window, cx: &App) -> Option<Hsla> {
    let state = cx.try_global::<GradientChromeState>()?;
    if !state.enabled {
        return None;
    }
    let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
    let base_a = if state.is_light { 0.26 } else { 0.34 };
    let alpha = (base_a * pulse).clamp(0.14, 0.50);
    Some(rgb_to_hsla(stops[3], alpha))
}

/// Paints a live-breathing 4-stop horizontal gradient selection highlight for Markdown text spans.
pub fn paint_markdown_selection_quad(
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &App,
) -> bool {
    if !is_enabled(cx) {
        return false;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let Some(state) = cx.try_global::<GradientChromeState>() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
        let base_a = if state.is_light { 0.28 } else { 0.34 };
        let a0 = (base_a * pulse).clamp(0.14, 0.50);
        let a1 = ((base_a + 0.03) * pulse).clamp(0.14, 0.54);
        paint_rounded_four_stop_band(bounds, px(3.0), stops, [a0, a1, a1, a0], true, window);
    });
    true
}

/// Paints a live-breathing gradient path for Editor text selections while keeping the enclosing
/// `Pane`/`Editor` view cached across Breathe ticks.
pub fn paint_selection_path(
    path: Path<Pixels>,
    opacity_scale: f32,
    fallback: Hsla,
    window: &mut Window,
    cx: &App,
) {
    if !is_enabled(cx) {
        window.paint_path(path, fallback);
        return;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let bg = selection_background(window, opacity_scale, cx)
            .unwrap_or_else(|| fallback.into());
        window.paint_path(path.clone(), bg);
    });
}

/// Paints a live-breathing gradient path for animated text cursors while keeping the enclosing
/// `Pane`/`Editor` view cached across Breathe ticks.
pub fn paint_cursor_path(
    path: Path<Pixels>,
    is_underline: bool,
    fallback: Hsla,
    window: &mut Window,
    cx: &App,
) {
    if !is_enabled(cx) {
        window.paint_path(path, fallback);
        return;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let bg = cursor_path_background(is_underline, fallback.a, window, cx)
            .unwrap_or_else(|| fallback.into());
        window.paint_path(path.clone(), bg);
    });
}

/// Paints a pixel-snapped, zero-seam 4-stop gradient rectangle with **native GPU SDF rounded
/// corners** (`gpui::quad` + `Corners`), eliminating both rectangular corner poke-outs and
/// overlapping 1/3 vertical seam lines.
pub fn paint_rounded_four_stop_band(
    bounds: Bounds<Pixels>,
    radius: Pixels,
    stops: FourStops,
    alphas: [f32; 4],
    horizontal: bool,
    window: &mut Window,
) {
    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let r_f32 = f32::from(radius).max(0.0).min(width * 0.5).min(height * 0.5);
    let r = px(r_f32);
    let primary_extent = if horizontal { width } else { height };

    // On compact pills/buttons/circles where `primary_extent < 3 * r`, use a single seamless
    // SDF rounded quad so corners are always mathematically smooth with zero pinching.
    if primary_extent < r_f32 * 3.0 + 3.0 {
        let angle = if horizontal { 125.0 } else { 155.0 };
        window.paint_quad(quad(
            bounds,
            Corners::all(r),
            linear_gradient(
                angle,
                linear_color_stop(rgb_to_hsla(stops[0], alphas[0]), 0.0),
                linear_color_stop(rgb_to_hsla(stops[3], alphas[3]), 1.0),
            ),
            Edges::default(),
            transparent_black(),
            BorderStyle::Solid,
        ));
        return;
    }

    let angle = if horizontal { 90.0 } else { 180.0 };
    let p0 = 0.0;
    let p1 = (primary_extent / 3.0).round();
    let p2 = (2.0 * primary_extent / 3.0).round();
    let p3 = primary_extent;
    let splits = [(p0, p1), (p1, p2), (p2, p3)];

    for (seg, (s0, s1)) in splits.into_iter().enumerate() {
        let len = s1 - s0;
        if len <= 0.0 {
            continue;
        }
        let (seg_bounds, seg_corners) = if horizontal {
            let b = Bounds {
                origin: point(px(origin_x + s0), px(origin_y)),
                size: size(px(len), px(height)),
            };
            let c = match seg {
                0 => Corners {
                    top_left: r,
                    top_right: px(0.0),
                    bottom_right: px(0.0),
                    bottom_left: r,
                },
                1 => Corners::all(px(0.0)),
                _ => Corners {
                    top_left: px(0.0),
                    top_right: r,
                    bottom_right: r,
                    bottom_left: px(0.0),
                },
            };
            (b, c)
        } else {
            let b = Bounds {
                origin: point(px(origin_x), px(origin_y + s0)),
                size: size(px(width), px(len)),
            };
            let c = match seg {
                0 => Corners {
                    top_left: r,
                    top_right: r,
                    bottom_right: px(0.0),
                    bottom_left: px(0.0),
                },
                1 => Corners::all(px(0.0)),
                _ => Corners {
                    top_left: px(0.0),
                    top_right: px(0.0),
                    bottom_right: r,
                    bottom_left: r,
                },
            };
            (b, c)
        };

        window.paint_quad(quad(
            seg_bounds,
            seg_corners,
            linear_gradient(
                angle,
                linear_color_stop(rgb_to_hsla(stops[seg], alphas[seg]), 0.0),
                linear_color_stop(rgb_to_hsla(stops[seg + 1], alphas[seg + 1]), 1.0),
            ),
            Edges::default(),
            transparent_black(),
            BorderStyle::Solid,
        ));
    }
}

/// Paints a concentric GPU-SDF rounded 4-color gradient border frame + solid or frosted inner card.
pub fn paint_rounded_gradient_frame(
    bounds: Bounds<Pixels>,
    radius_px: f32,
    border_px: f32,
    stops: FourStops,
    border_alpha: f32,
    inner_bg: Hsla,
    inner_wash_alpha: f32,
    window: &mut Window,
) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= border_px * 2.0 || height <= border_px * 2.0 {
        return;
    }

    // 1. Outer 4-color gradient perimeter with smooth GPU SDF rounded corners
    let outer_r = px(radius_px);
    paint_rounded_four_stop_band(
        bounds,
        outer_r,
        stops,
        [border_alpha, border_alpha, border_alpha, border_alpha],
        true,
        window,
    );

    // 2. Concentric inset inner card (solid opaque for code boxes, frosted glass for UI pills/cards)
    let inner_bounds = Bounds {
        origin: point(
            bounds.origin.x + px(border_px),
            bounds.origin.y + px(border_px),
        ),
        size: size(
            px((width - border_px * 2.0).max(0.0)),
            px((height - border_px * 2.0).max(0.0)),
        ),
    };
    let inner_r = px((radius_px - border_px).max(0.0));
    window.paint_quad(quad(
        inner_bounds,
        Corners::all(inner_r),
        inner_bg,
        Edges::default(),
        transparent_black(),
        BorderStyle::Solid,
    ));

    // 3. Optional subtle inner 4-color bar wash matching VS Code `--g-bar` / `--g-chat-bar`
    if inner_wash_alpha > 0.001 {
        paint_rounded_four_stop_band(
            inner_bounds,
            inner_r,
            stops,
            [
                inner_wash_alpha,
                inner_wash_alpha * 0.80,
                inner_wash_alpha * 0.80,
                inner_wash_alpha * 1.10,
            ],
            true,
            window,
        );
    }
}

/// Paints a scrollbar thumb with a directional 4-stop gradient (180° vertical for vertical
/// scrollbars, 90° horizontal for horizontal scrollbars) when Gradient Chrome is active.
pub fn paint_scrollbar_thumb(
    bounds: Bounds<Pixels>,
    horizontal: bool,
    opacity_mult: f32,
    radius_px: f32,
    window: &mut Window,
    cx: &App,
) -> bool {
    if !is_enabled(cx) {
        return false;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let Some(state) = cx.try_global::<GradientChromeState>() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
        let a = (opacity_mult * pulse).clamp(0.0, 0.96);
        if a <= 0.001 {
            return;
        }
        paint_rounded_four_stop_band(
            bounds,
            px(radius_px),
            stops,
            [a, a, a, a],
            horizontal,
            window,
        );
    });
    true
}

/// Returns a directional gradient `Background` for animated text cursor paths (`Bar`/`Block` =
/// vertical, `Underline` = horizontal), preserving `color_alpha` for blinking and dimming.
pub fn cursor_path_background(
    is_underline: bool,
    color_alpha: f32,
    window: &Window,
    cx: &App,
) -> Option<Background> {
    let state = cx.try_global::<GradientChromeState>()?;
    if !state.enabled {
        return None;
    }
    let (stops, _) = state.live_stops_and_pulse(None, window.is_window_active());
    let a = color_alpha.clamp(0.0, 1.0);
    let angle = if is_underline { 90.0 } else { 180.0 };
    Some(linear_gradient(
        angle,
        linear_color_stop(rgb_to_hsla(stops[0], a), 0.0),
        linear_color_stop(rgb_to_hsla(stops[3], a), 1.0),
    ))
}

/// Paints a static text cursor (`Bar`/`Block` = vertical 4-color gradient, `Underline` = horizontal
/// 4-color gradient, `Hollow` = palette accent outline), preserving `color_alpha` for blinking.
pub fn paint_text_cursor_quad(
    bounds: Bounds<Pixels>,
    is_underline: bool,
    is_hollow: bool,
    color_alpha: f32,
    radius_px: f32,
    window: &mut Window,
    cx: &App,
) -> bool {
    if !is_enabled(cx) {
        return false;
    }
    let a = color_alpha.clamp(0.0, 1.0);
    if a <= 0.001 {
        return true;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let Some(state) = cx.try_global::<GradientChromeState>() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let (stops, _) = state.live_stops_and_pulse(None, window.is_window_active());
        if is_hollow {
            window.paint_quad(outline(bounds, rgb_to_hsla(stops[0], a), BorderStyle::Solid));
        } else {
            paint_rounded_four_stop_band(
                bounds,
                px(radius_px),
                stops,
                [a, a, a, a],
                is_underline,
                window,
            );
        }
    });
    true
}

/// Paints a frosted-opaque 4-color gradient frame for sticky headers and elevated overlays so
/// scrolling content behind them is cleanly masked while preserving the live gradient theme.
pub fn paint_sticky_top_frame(
    bounds: Bounds<Pixels>,
    radius_px: f32,
    border_px: f32,
    inner_wash_alpha: f32,
    border_alpha: f32,
    window: &mut Window,
    cx: &App,
) -> bool {
    if !is_enabled(cx) {
        return false;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let Some(state) = cx.try_global::<GradientChromeState>() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
        let inner_bg = state
            .canvas_bg
            .opacity(if state.is_light { 0.90 } else { 0.92 });
        let b_alpha = (border_alpha * pulse).clamp(0.15, 0.96);
        let w_alpha = (inner_wash_alpha * pulse).clamp(0.0, 0.45);
        paint_rounded_gradient_frame(
            bounds,
            radius_px,
            border_px,
            stops,
            b_alpha,
            inner_bg,
            w_alpha,
            window,
        );
    });
    true
}

/// Paints a soft directional 4-stop gradient hover highlight over a row or sticky header line.
pub fn paint_gradient_hover_band(
    bounds: Bounds<Pixels>,
    radius_px: f32,
    alpha: f32,
    window: &mut Window,
    cx: &App,
) -> bool {
    if !is_enabled(cx) {
        return false;
    }
    window.paint_dynamic(cx, move |window, cx| {
        let Some(state) = cx.try_global::<GradientChromeState>() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let (stops, pulse) = state.live_stops_and_pulse(None, window.is_window_active());
        let a = (alpha * pulse).clamp(0.0, 0.45);
        paint_rounded_four_stop_band(
            bounds,
            px(radius_px),
            stops,
            [a, a * 0.85, a * 0.85, a],
            true,
            window,
        );
    });
    true
}

fn render_gradient_frame_canvas_inner(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    solid_inner: bool,
    opaque_solid_wash: bool,
    sticky_opaque: bool,
    breathing: bool,
    inner_wash_alpha: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    if !is_enabled(cx) {
        return None;
    }

    Some(
        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, (), window, cx| {
                let paint_frame = move |window: &mut Window, cx: &App| {
                    let Some(state) = cx.try_global::<GradientChromeState>() else {
                        return;
                    };
                    if !state.enabled {
                        return;
                    }
                    let inset_bounds = Bounds {
                        origin: point(
                            bounds.origin.x + px(inset_x_px),
                            bounds.origin.y + px(inset_y_px),
                        ),
                        size: size(
                            px((f32::from(bounds.size.width) - inset_x_px * 2.0).max(0.0)),
                            px((f32::from(bounds.size.height) - inset_y_px * 2.0).max(0.0)),
                        ),
                    };
                    let (stops, pulse) = if breathing {
                        state.live_stops_and_pulse(override_stops, window.is_window_active())
                    } else {
                        (override_stops.unwrap_or(state.stops), 1.0)
                    };
                    let (inner_bg, base_wash) = if opaque_solid_wash || (solid_inner && !breathing)
                    {
                        let mut solid = state.canvas_bg;
                        solid.a = 1.0;
                        (solid, inner_wash_alpha)
                    } else if sticky_opaque {
                        let masked = state
                            .canvas_bg
                            .opacity(if state.is_light { 0.90 } else { 0.92 });
                        (masked, inner_wash_alpha)
                    } else if solid_inner {
                        // Frosted translucent card for code/mermaid blocks so they don't look like
                        // glaring opaque white slabs while preserving crisp syntax contrast.
                        let frosted = state
                            .canvas_bg
                            .opacity(if state.is_light { 0.74 } else { 0.78 });
                        let wash = if inner_wash_alpha > 0.0 {
                            inner_wash_alpha
                        } else {
                            0.06
                        };
                        (frosted, wash)
                    } else {
                        (
                            state
                                .canvas_bg
                                .opacity(if state.is_light { 0.78 } else { 0.78 }),
                            inner_wash_alpha,
                        )
                    };
                    let b_alpha = (border_alpha * pulse).clamp(0.15, 0.96);
                    let w_alpha = if base_wash > 0.0 {
                        (base_wash * pulse).clamp(0.0, 0.45)
                    } else {
                        0.0
                    };
                    paint_rounded_gradient_frame(
                        inset_bounds,
                        radius_px,
                        border_px,
                        stops,
                        b_alpha,
                        inner_bg,
                        w_alpha,
                        window,
                    );
                };

                if breathing {
                    window.paint_dynamic(cx, paint_frame);
                } else {
                    paint_frame(window, cx);
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element(),
    )
}

/// Mounts an absolute `inset_0()` GPU-SDF rounded breathing gradient frame canvas as the FIRST
/// child of any `relative()` container (Code Box, Opened Thread, Opened File, Active Tab, Buttons).
/// Automatically breathes in lockstep via cached dynamic scene repaints!
pub fn render_gradient_frame_canvas(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    solid_inner: bool,
    inner_wash_alpha: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    render_gradient_frame_canvas_inner(
        override_stops,
        radius_px,
        border_px,
        inset_x_px,
        inset_y_px,
        solid_inner,
        false,
        false,
        true,
        inner_wash_alpha,
        border_alpha,
        cx,
    )
}

/// Mounts a **breathing** rounded 4-color gradient frame with a solid opaque `canvas_bg` (`a = 1.0`)
/// backing plus a live-breathing 4-color inner wash for chatboxes and composers.
pub fn render_breathing_solid_gradient_frame_canvas(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    inner_wash_alpha: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    render_gradient_frame_canvas_inner(
        override_stops,
        radius_px,
        border_px,
        inset_x_px,
        inset_y_px,
        true,
        true,
        false,
        true,
        inner_wash_alpha,
        border_alpha,
        cx,
    )
}

/// Mounts a frosted-opaque, live-breathing rounded 4-color gradient frame for sticky top headers
/// and elevated tooltips so underlying scrolled text stays cleanly masked.
pub fn render_sticky_gradient_frame_canvas(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    inner_wash_alpha: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    render_gradient_frame_canvas_inner(
        override_stops,
        radius_px,
        border_px,
        inset_x_px,
        inset_y_px,
        false,
        false,
        true,
        true,
        inner_wash_alpha,
        border_alpha,
        cx,
    )
}

/// Mounts a **non-breathing**, rounded 4-color gradient frame with a **solid** inner backing
/// (`canvas_bg`, alpha 1.0) plus a static inner 4-color gradient wash.
pub fn render_static_gradient_frame_canvas(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    inner_wash_alpha: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    render_gradient_frame_canvas_inner(
        override_stops,
        radius_px,
        border_px,
        inset_x_px,
        inset_y_px,
        true,
        true,
        false,
        false,
        inner_wash_alpha,
        border_alpha,
        cx,
    )
}

/// Mounts a **non-breathing**, rounded 4-color gradient frame with a **solid** inner background
/// (`canvas_bg`, alpha 1.0, zero inner wash) for compact textboxes and search inputs.
pub fn render_static_solid_gradient_frame_canvas(
    override_stops: Option<FourStops>,
    radius_px: f32,
    border_px: f32,
    inset_x_px: f32,
    inset_y_px: f32,
    border_alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    render_static_gradient_frame_canvas(
        override_stops,
        radius_px,
        border_px,
        inset_x_px,
        inset_y_px,
        0.0,
        border_alpha,
        cx,
    )
}

/// Mounts an absolute `inset_0()` live-breathing 4-stop gradient strip for panel dividers & edges.
pub fn render_gradient_edge_canvas(
    override_stops: Option<FourStops>,
    horizontal: bool,
    alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    if !is_enabled(cx) {
        return None;
    }

    Some(
        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, (), window, cx| {
                window.paint_dynamic(cx, move |window, cx| {
                    let Some(state) = cx.try_global::<GradientChromeState>() else {
                        return;
                    };
                    if !state.enabled {
                        return;
                    }
                    let (stops, pulse) =
                        state.live_stops_and_pulse(override_stops, window.is_window_active());
                    let a = (alpha * pulse).clamp(0.12, 0.96);
                    paint_rounded_four_stop_band(
                        bounds,
                        px(0.0),
                        stops,
                        [a, a, a, a],
                        horizontal,
                        window,
                    );
                });
            },
        )
        .absolute()
        .inset_0()
        .into_any_element(),
    )
}
