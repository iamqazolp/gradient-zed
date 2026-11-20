use std::f32::consts::TAU;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, ContentMask, Hsla, IntoElement,
    ParentElement as _, Pixels, Styled, Window, canvas, div, fill, linear_color_stop,
    linear_gradient, point, px, radial_gradient, size,
};
use theme::{ActiveTheme, Appearance};

use crate::palettes::{CURATED_PALETTES, FourStops, lerp_stops, rgb_to_hsla};
use crate::state::{BreatheConfig, GradientStudioState, PanelRegion};
use crate::theme_glass::canvas_background;

/// Exact 4-corner radial ellipse geometry matching VS Code Gradient Themes (`rainbow-gradient-*.css`):
/// `(stop_index, radius_x, radius_y, falloff, base_alpha)`
const RADIAL_LAYERS: [(usize, f32, f32, f32, f32); 4] = [
    (0, 0.75, 0.55, 0.70, 0.18),
    (1, 0.65, 0.75, 0.72, 0.18),
    (2, 0.75, 0.60, 0.72, 0.18),
    (3, 0.70, 0.70, 0.75, 0.22),
];

/// Exact 3-radial terminal aurora geometry matching VS Code `--g-default-terminal-panel`:
/// `(stop_index, cx, cy, radius_x, radius_y, falloff, base_alpha)`
const TERMINAL_RADIAL_LAYERS: [(usize, f32, f32, f32, f32, f32, f32); 3] = [
    (0, 0.12, 0.20, 0.65, 0.85, 0.70, 0.18),
    (1, 0.88, 0.80, 0.60, 0.80, 0.70, 0.18),
    (2, 0.50, 0.50, 0.55, 0.65, 0.75, 0.18),
];

#[derive(Clone, Copy)]
enum AuroraShape {
    /// Full 4-corner radial mesh + 135deg wash (Seamless window, Editor, wide Zoomed panel).
    FullMesh,
    /// Vertical 4-stop sidebar wash + subtle corner glow (VS Code `--g-explorer-side` / `--g-chat-side`).
    Sidebar,
    /// 3-radial + 105deg terminal aurora (VS Code `--g-default-terminal-panel`).
    Terminal,
    /// Crisp 90deg horizontal 4-stop ribbon (VS Code `--g-bars-bar` for Title Bar & Status Bar).
    HorizontalBar,
}

/// Renders the translucent 4-color aurora **underlay** as a single GPU `canvas()` element.
///
/// IMPORTANT: Mount this as the **FIRST** child of its `relative()` container (BEFORE text,
/// editors, buttons, and icons) so the aurora shines *behind* all UI at 100% text contrast —
/// identical to VS Code's `#prism-aurora-backdrop` (`z-index: -1`).
///
/// - `region == None` renders on the top-level `MultiWorkspace` root in Seamless mode.
/// - `region == Some(panel)` renders in Split Panels mode (`seamless == false`), OR in Seamless
///   mode whenever that specific component has a custom palette or has been toggled off (masking
///   the global seamless backdrop cleanly with `canvas_background`, identical to VS Code's
///   `.prism-custom-<comp>` rules).
pub fn render_aurora_overlay(
    region: Option<PanelRegion>,
    window: &mut Window,
    cx: &App,
) -> Option<AnyElement> {
    let state = cx.try_global::<GradientStudioState>()?;
    if !state.enabled {
        return None;
    }

    let canvas_bg = canvas_background(cx);
    let (opaque_base, shape) = match region {
        None => {
            if !state.seamless {
                return None;
            }
            // Root already paints solid `canvas_bg`; no duplicate opaque fill needed.
            (None, AuroraShape::FullMesh)
        }
        Some(panel_region) => {
            let panel_enabled = state.panels.is_enabled(panel_region);
            let has_custom = state.component_palettes.get(panel_region).is_some();

            if state.seamless {
                if panel_enabled && !has_custom {
                    // Transparent — let MultiWorkspace's unified seamless backdrop shine through!
                    return None;
                }
                if !panel_enabled {
                    // Component explicitly turned OFF in Seamless mode -> solid theme canvas mask.
                    return Some(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(canvas_bg)
                            .into_any_element(),
                    );
                }
                // Custom palette for this component inside Seamless mode -> opaque base + local mesh.
                (Some(canvas_bg), AuroraShape::FullMesh)
            } else {
                // Split Panels (Seamful) mode: each panel paints its own opaque base + signature shape.
                if !panel_enabled {
                    return Some(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(canvas_bg)
                            .into_any_element(),
                    );
                }
                let split_shape = match panel_region {
                    PanelRegion::Editor => AuroraShape::FullMesh,
                    PanelRegion::LeftDock | PanelRegion::RightDock => AuroraShape::Sidebar,
                    PanelRegion::BottomDock => AuroraShape::Terminal,
                    PanelRegion::Bars => AuroraShape::HorizontalBar,
                };
                (Some(canvas_bg), split_shape)
            }
        }
    };

    Some(render_aurora_element(
        state,
        region,
        shape,
        opaque_base,
        false,
        window,
        cx,
    ))
}

/// Renders the aurora **underlay** for a zoomed workspace panel (`self.zoomed`, such as a zoomed
/// Agent panel or zoomed Editor).
///
/// - Always sits behind `zoomed_element` on top of an opaque `canvas_bg` so underlying panes
///   never bleed through.
/// - In unified Seamless mode, aligns the radial mesh to `window.viewport_size()` clipped to the
///   zoomed panel's bounds (`window.with_content_mask`) so the gradient continues seamlessly
///   across the sidebar/titlebar seam with zero discontinuity!
/// - In Split Panels mode (or when customized), paints a rich full-panel 4-corner aurora mesh.
pub fn render_zoomed_aurora_backdrop(
    region: PanelRegion,
    window: &mut Window,
    cx: &App,
) -> Option<AnyElement> {
    let state = cx.try_global::<GradientStudioState>()?;
    if !state.enabled {
        return None;
    }

    if !state.panels.is_enabled(region) {
        return None;
    }

    let has_custom = state.component_palettes.get(region).is_some();
    let viewport_locked = state.seamless && !has_custom;
    let effective_region = if viewport_locked { None } else { Some(region) };

    Some(render_aurora_element(
        state,
        effective_region,
        AuroraShape::FullMesh,
        None, // `self.zoomed` wrapper already sets `.bg(canvas_background(cx))`
        viewport_locked,
        window,
        cx,
    ))
}

/// Edge along which a zero-layout-shift 4-color gradient hairline border is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientEdge {
    Top,
    Bottom,
    Left,
    Right,
}

fn render_four_stop_fill(stops: FourStops, alpha: f32, horizontal: bool) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, _cx| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            if width <= 0.0 || height <= 0.0 {
                return;
            }
            paint_four_stop_band(bounds, stops, [alpha, alpha, alpha, alpha], horizontal, window);
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

/// Paints a sleek 4-color rainbow gradient hairline along one edge of a `relative()` container
/// (matching VS Code's `--g-border` / `--g-border-v` panel dividers) with zero layout shift.
pub fn render_gradient_edge(
    region: Option<PanelRegion>,
    edge: GradientEdge,
    thickness_px: f32,
    alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    let state = cx.try_global::<GradientStudioState>()?;
    if !state.enabled {
        return None;
    }
    let stops = state.resolved_stops(region);
    let horizontal = matches!(edge, GradientEdge::Top | GradientEdge::Bottom);
    let mut strip = div().absolute().overflow_hidden();
    strip = match edge {
        GradientEdge::Top => strip.top_0().left_0().right_0().h(px(thickness_px)),
        GradientEdge::Bottom => strip.bottom_0().left_0().right_0().h(px(thickness_px)),
        GradientEdge::Left => strip.top_0().bottom_0().left_0().w(px(thickness_px)),
        GradientEdge::Right => strip.top_0().bottom_0().right_0().w(px(thickness_px)),
    };
    Some(
        strip
            .child(render_four_stop_fill(stops, alpha, horizontal))
            .into_any_element(),
    )
}

/// Paints an in-flow 4-color rainbow gradient divider line (e.g. between sidebar project groups).
pub fn render_gradient_divider(
    region: Option<PanelRegion>,
    horizontal: bool,
    thickness_px: f32,
    alpha: f32,
    cx: &App,
) -> Option<AnyElement> {
    let state = cx.try_global::<GradientStudioState>()?;
    if !state.enabled {
        return None;
    }
    let stops = state.resolved_stops(region);
    let mut strip = div().relative().flex_shrink_0().overflow_hidden();
    strip = if horizontal {
        strip.w_full().h(px(thickness_px))
    } else {
        strip.h_full().w(px(thickness_px))
    };
    Some(
        strip
            .child(render_four_stop_fill(stops, alpha, horizontal))
            .into_any_element(),
    )
}

/// Wraps `content` in VS Code's signature rounded 4-color gradient border frame (`1.5px` prism
/// perimeter around a frosted translucent glass surface with crisp content on top).
pub fn wrap_gradient_frame(
    region: Option<PanelRegion>,
    radius_px: f32,
    border_px: f32,
    fill_height: bool,
    content: AnyElement,
    cx: &App,
) -> AnyElement {
    let Some(state) = cx.try_global::<GradientStudioState>().filter(|s| s.enabled) else {
        return content;
    };
    let stops = state.resolved_stops(region);
    let is_light = cx.theme().appearance() == Appearance::Light;
    let canvas_bg = canvas_background(cx);
    let inner_bg = canvas_bg.opacity(if is_light { 0.80 } else { 0.78 });
    let inner_wash_alpha = if is_light { 0.09 } else { 0.14 };
    let border_alpha = if is_light { 0.86 } else { 0.82 };

    let mut inner = div()
        .relative()
        .w_full()
        .rounded(px((radius_px - border_px).max(2.0)))
        .overflow_hidden()
        .bg(inner_bg)
        .child(render_four_stop_fill(stops, inner_wash_alpha, true))
        .child(content);
    if fill_height {
        inner = inner.flex_1().h_full().flex().flex_col();
    }

    let mut outer = div()
        .relative()
        .w_full()
        .p(px(border_px))
        .rounded(px(radius_px))
        .overflow_hidden()
        .shadow_sm()
        .child(render_four_stop_fill(stops, border_alpha, true))
        .child(inner);
    if fill_height {
        outer = outer.flex_1().h_full().flex().flex_col();
    }

    outer.into_any_element()
}

/// Renders a decorative 4-corner radial aurora backdrop inside the Gradient Studio card itself.
pub fn render_studio_card_backdrop(stops: FourStops, is_light: bool) -> AnyElement {
    let base_scale = if is_light { 0.85 } else { 1.05 };
    let focal = [(0.14, 0.14), (0.86, 0.22), (0.20, 0.86), (0.86, 0.86)];
    build_aurora_canvas(
        stops,
        135.0,
        base_scale,
        focal,
        AuroraShape::FullMesh,
        None,
        false,
    )
    .into_any_element()
}

fn render_aurora_element(
    state: &GradientStudioState,
    region: Option<PanelRegion>,
    shape: AuroraShape,
    opaque_base: Option<Hsla>,
    viewport_locked: bool,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let is_light = cx.theme().appearance() == Appearance::Light;
    // Because the aurora renders BEHIND text & buttons, we can match VS Code's full lush vibrancy
    // without ever washing out code or UI contrast.
    let base_scale = if is_light { 0.90 } else { 1.00 } * state.intensity;
    let stops = state.resolved_stops(region);
    let base_angle = state.base_angle;
    let cfg = state.breathe_cfg;
    let any_motion = cfg.full_wave || cfg.pulse || cfg.shimmer || cfg.auto_morph;
    let breathe_active = state.breathe && any_motion && window.is_window_active();

    if breathe_active {
        let anim_id = match (region, viewport_locked) {
            (_, true) => "aurora-breathe-zoomed-seamless",
            (None, false) => "aurora-breathe-seamless",
            (Some(PanelRegion::Editor), false) => "aurora-breathe-editor",
            (Some(PanelRegion::LeftDock), false) => "aurora-breathe-left",
            (Some(PanelRegion::RightDock), false) => "aurora-breathe-right",
            (Some(PanelRegion::BottomDock), false) => "aurora-breathe-bottom",
            (Some(PanelRegion::Bars), false) => "aurora-breathe-bars",
        };
        let speed_mult = (cfg.speed_pct / 100.0).clamp(0.25, 2.5);
        let cycle_secs = (18.0 / speed_mult).clamp(5.0, 72.0);
        let focal_static = compute_focal_points(None, cfg);
        build_aurora_canvas(
            stops,
            base_angle,
            base_scale,
            focal_static,
            shape,
            opaque_base,
            viewport_locked,
        )
        .with_animation(
            anim_id,
            Animation::new(Duration::from_secs_f32(cycle_secs))
                .repeat_synced()
                .with_max_fps(24.0),
            move |_prev, delta| {
                let phase = delta * TAU;
                let focal = compute_focal_points(Some(phase), cfg);
                let pulse_mult = if cfg.pulse {
                    1.0 + 0.18 * (phase * 2.0).sin()
                } else {
                    1.0
                };
                let angle = if cfg.shimmer {
                    (base_angle + 28.0 * phase.sin()).rem_euclid(360.0)
                } else {
                    base_angle
                };
                let active_stops = if cfg.auto_morph && !CURATED_PALETTES.is_empty() {
                    let count = CURATED_PALETTES.len().min(12);
                    let pos = delta * count as f32;
                    let idx_a = (pos.floor() as usize) % count;
                    let idx_b = (idx_a + 1) % count;
                    let frac = pos - pos.floor();
                    lerp_stops(
                        CURATED_PALETTES[idx_a].stops,
                        CURATED_PALETTES[idx_b].stops,
                        frac,
                    )
                } else {
                    stops
                };
                build_aurora_canvas(
                    active_stops,
                    angle,
                    base_scale * pulse_mult,
                    focal,
                    shape,
                    opaque_base,
                    viewport_locked,
                )
            },
        )
        .into_any_element()
    } else {
        let focal = compute_focal_points(None, cfg);
        build_aurora_canvas(
            stops,
            base_angle,
            base_scale,
            focal,
            shape,
            opaque_base,
            viewport_locked,
        )
        .into_any_element()
    }
}

fn compute_focal_points(phase_radians: Option<f32>, cfg: BreatheConfig) -> [(f32, f32); 4] {
    let base = [(0.15, 0.15), (0.85, 0.25), (0.25, 0.85), (0.85, 0.85)];
    let Some(t) = phase_radians else {
        return base;
    };
    if !cfg.full_wave {
        return base;
    }
    [
        (base[0].0 + 0.08 * t.sin(), base[0].1 + 0.06 * (t * 0.8).cos()),
        (
            base[1].0 + 0.07 * (t * 0.9 + 1.4).cos(),
            base[1].1 + 0.08 * (t * 0.7 + 0.6).sin(),
        ),
        (
            base[2].0 + 0.08 * (t * 0.75 + 2.5).sin(),
            base[2].1 + 0.06 * (t * 1.05 + 1.1).cos(),
        ),
        (
            base[3].0 + 0.07 * (t * 0.85 + 3.8).cos(),
            base[3].1 + 0.07 * (t * 0.95 + 2.2).sin(),
        ),
    ]
}

fn paint_four_stop_band(
    bounds: Bounds<Pixels>,
    stops: FourStops,
    alphas: [f32; 4],
    horizontal: bool,
    window: &mut Window,
) {
    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let angle = if horizontal { 90.0 } else { 180.0 };

    for seg in 0..3 {
        let t0 = seg as f32 / 3.0;
        let t1 = (seg + 1) as f32 / 3.0;
        let seg_bounds = if horizontal {
            Bounds {
                origin: point(px(origin_x + t0 * width), px(origin_y)),
                size: size(px((t1 - t0) * width + 0.5), px(height)),
            }
        } else {
            Bounds {
                origin: point(px(origin_x), px(origin_y + t0 * height)),
                size: size(px(width), px((t1 - t0) * height + 0.5)),
            }
        };
        let from = rgb_to_hsla(stops[seg], alphas[seg]);
        let to = rgb_to_hsla(stops[seg + 1], alphas[seg + 1]);
        window.paint_quad(fill(
            seg_bounds,
            linear_gradient(
                angle,
                linear_color_stop(from, 0.0),
                linear_color_stop(to, 1.0),
            ),
        ));
    }
}

fn paint_clipped_radial(
    bounds: Bounds<Pixels>,
    cx_uv: f32,
    cy_uv: f32,
    rx: f32,
    ry: f32,
    falloff: f32,
    color: Hsla,
    window: &mut Window,
) {
    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);

    let half_u = rx * falloff;
    let half_v = ry * falloff;

    let u0 = (cx_uv - half_u).clamp(0.0, 1.0);
    let u1 = (cx_uv + half_u).clamp(0.0, 1.0);
    let v0 = (cy_uv - half_v).clamp(0.0, 1.0);
    let v1 = (cy_uv + half_v).clamp(0.0, 1.0);

    let du = u1 - u0;
    let dv = v1 - v0;
    if du <= 0.001 || dv <= 0.001 {
        return;
    }

    let clipped_bounds = Bounds {
        origin: point(px(origin_x + u0 * width), px(origin_y + v0 * height)),
        size: size(px(du * width), px(dv * height)),
    };
    let local_cx = (cx_uv - u0) / du;
    let local_cy = (cy_uv - v0) / dv;
    let local_rx = rx / du;
    let local_ry = ry / dv;

    window.paint_quad(fill(
        clipped_bounds,
        radial_gradient(local_cx, local_cy, local_rx, local_ry, falloff, color),
    ));
}

fn paint_aurora_shape(
    paint_bounds: Bounds<Pixels>,
    stops: FourStops,
    base_angle: f32,
    base_scale: f32,
    focal: [(f32, f32); 4],
    shape: AuroraShape,
    window: &mut Window,
) {
    let clamp_alpha = move |raw: f32| (raw * base_scale).clamp(0.02, 0.56);

    match shape {
        AuroraShape::FullMesh => {
            // 1. Subtle diagonal linear wash matching VS Code `--g-base`
            let base_from = rgb_to_hsla(stops[0], clamp_alpha(0.13));
            let base_to = rgb_to_hsla(stops[3], clamp_alpha(0.15));
            window.paint_quad(fill(
                paint_bounds,
                linear_gradient(
                    base_angle,
                    linear_color_stop(base_from, 0.0),
                    linear_color_stop(base_to, 1.0),
                ),
            ));

            // 2. 4 tight-clipped elliptical radial glows matching VS Code `--g-default-mesh`
            for (idx, rx, ry, falloff, alpha) in RADIAL_LAYERS {
                let (cx_uv, cy_uv) = focal[idx];
                let color = rgb_to_hsla(stops[idx], clamp_alpha(alpha));
                paint_clipped_radial(
                    paint_bounds,
                    cx_uv,
                    cy_uv,
                    rx,
                    ry,
                    falloff,
                    color,
                    window,
                );
            }
        }
        AuroraShape::Sidebar => {
            // VS Code `--g-explorer-side` / `--g-chat-side`: vertical 4-stop wash + soft corner glow
            let alphas = [
                clamp_alpha(0.16),
                clamp_alpha(0.13),
                clamp_alpha(0.13),
                clamp_alpha(0.16),
            ];
            paint_four_stop_band(paint_bounds, stops, alphas, false, window);
            paint_clipped_radial(
                paint_bounds,
                0.20,
                0.12,
                0.90,
                0.45,
                0.75,
                rgb_to_hsla(stops[0], clamp_alpha(0.11)),
                window,
            );
            paint_clipped_radial(
                paint_bounds,
                0.80,
                0.88,
                0.90,
                0.45,
                0.75,
                rgb_to_hsla(stops[3], clamp_alpha(0.13)),
                window,
            );
        }
        AuroraShape::Terminal => {
            // VS Code `--g-default-terminal-panel`: 105deg base + 3-radial aurora
            let base_from = rgb_to_hsla(stops[1], clamp_alpha(0.08));
            let base_to = rgb_to_hsla(stops[3], clamp_alpha(0.18));
            window.paint_quad(fill(
                paint_bounds,
                linear_gradient(
                    105.0,
                    linear_color_stop(base_from, 0.0),
                    linear_color_stop(base_to, 1.0),
                ),
            ));
            for (idx, cx_uv, cy_uv, rx, ry, falloff, alpha) in TERMINAL_RADIAL_LAYERS {
                let color = rgb_to_hsla(stops[idx], clamp_alpha(alpha));
                paint_clipped_radial(
                    paint_bounds,
                    cx_uv,
                    cy_uv,
                    rx,
                    ry,
                    falloff,
                    color,
                    window,
                );
            }
        }
        AuroraShape::HorizontalBar => {
            // VS Code `--g-bars-bar`: crisp 90deg 4-stop horizontal ribbon
            let alphas = [
                clamp_alpha(0.20),
                clamp_alpha(0.15),
                clamp_alpha(0.15),
                clamp_alpha(0.22),
            ];
            paint_four_stop_band(paint_bounds, stops, alphas, true, window);
        }
    }
}

fn build_aurora_canvas(
    stops: FourStops,
    base_angle: f32,
    base_scale: f32,
    focal: [(f32, f32); 4],
    shape: AuroraShape,
    opaque_base: Option<Hsla>,
    viewport_locked: bool,
) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, _cx| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            if width <= 0.0 || height <= 0.0 {
                return;
            }

            if let Some(bg) = opaque_base {
                window.paint_quad(fill(bounds, bg));
            }

            if viewport_locked {
                let viewport_bounds = Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: window.viewport_size(),
                };
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    paint_aurora_shape(
                        viewport_bounds,
                        stops,
                        base_angle,
                        base_scale,
                        focal,
                        shape,
                        window,
                    );
                });
            } else {
                paint_aurora_shape(
                    bounds,
                    stops,
                    base_angle,
                    base_scale,
                    focal,
                    shape,
                    window,
                );
            }
        },
    )
    .absolute()
    .inset_0()
}
