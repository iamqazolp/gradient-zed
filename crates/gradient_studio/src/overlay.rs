use std::cell::RefCell;
use std::collections::HashSet;
use std::f32::consts::TAU;
use std::time::Duration;

use gpui::{
    AnyElement, App, Bounds, ContentMask, Hsla, IntoElement, ParentElement as _, Pixels, Styled,
    Window, WindowId, canvas, div, fill, linear_color_stop, linear_gradient, point, px,
    radial_gradient, size,
};

thread_local! {
    static PENDING_BREATHE_REFRESHES: RefCell<HashSet<WindowId>> = RefCell::new(HashSet::new());
}
use theme::{ActiveTheme, Appearance};

use crate::palettes::{CURATED_PALETTES, FourStops, rgb_to_hsla};
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

/// Paints a sleek, live-breathing 4-color rainbow gradient hairline along one edge of a
/// `relative()` container (matching VS Code's `--g-border` / `--g-border-v` panel dividers)
/// with zero layout shift.
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
    let custom_override =
        region.and_then(|panel| state.component_palettes.get(panel).map(|p| p.stops));
    let horizontal = matches!(edge, GradientEdge::Top | GradientEdge::Bottom);
    let edge_canvas =
        ui::gradient_chrome::render_gradient_edge_canvas(custom_override, horizontal, alpha, cx)?;
    let mut strip = div().absolute().overflow_hidden();
    strip = match edge {
        GradientEdge::Top => strip.top_0().left_0().right_0().h(px(thickness_px)),
        GradientEdge::Bottom => strip.bottom_0().left_0().right_0().h(px(thickness_px)),
        GradientEdge::Left => strip.top_0().bottom_0().left_0().w(px(thickness_px)),
        GradientEdge::Right => strip.top_0().bottom_0().right_0().w(px(thickness_px)),
    };
    Some(strip.child(edge_canvas).into_any_element())
}

/// Paints an in-flow, live-breathing 4-color rainbow gradient divider line (e.g. between sidebar
/// project groups).
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
    let custom_override =
        region.and_then(|panel| state.component_palettes.get(panel).map(|p| p.stops));
    let edge_canvas =
        ui::gradient_chrome::render_gradient_edge_canvas(custom_override, horizontal, alpha, cx)?;
    let mut strip = div().relative().flex_shrink_0().overflow_hidden();
    strip = if horizontal {
        strip.w_full().h(px(thickness_px))
    } else {
        strip.h_full().w(px(thickness_px))
    };
    Some(strip.child(edge_canvas).into_any_element())
}

fn wrap_gradient_frame_inner(
    region: Option<PanelRegion>,
    radius_px: f32,
    border_px: f32,
    fill_height: bool,
    solid_mode: Option<bool>,
    content: AnyElement,
    cx: &App,
) -> AnyElement {
    let Some(state) = cx.try_global::<GradientStudioState>().filter(|s| s.enabled) else {
        return content;
    };
    let custom_override =
        region.and_then(|panel| state.component_palettes.get(panel).map(|p| p.stops));
    let is_light = cx.theme().appearance() == Appearance::Light;
    let border_alpha = if is_light { 0.88 } else { 0.84 };

    let frame_canvas = match solid_mode {
        // Static solid textbox frame (compact single-line inputs)
        Some(false) => {
            let inner_wash_alpha = if is_light { 0.15 } else { 0.20 };
            ui::gradient_chrome::render_static_gradient_frame_canvas(
                custom_override,
                radius_px,
                border_px,
                0.0,
                0.0,
                inner_wash_alpha,
                border_alpha,
                cx,
            )
        }
        // Breathing solid-backed gradient wash + border (chatbox composer & prompt edit)
        Some(true) => {
            let inner_wash_alpha = if is_light { 0.15 } else { 0.20 };
            ui::gradient_chrome::render_breathing_solid_gradient_frame_canvas(
                custom_override,
                radius_px,
                border_px,
                0.0,
                0.0,
                inner_wash_alpha,
                border_alpha,
                cx,
            )
        }
        // Standard frosted translucent breathing gradient frame
        None => {
            let inner_wash_alpha = if is_light { 0.10 } else { 0.14 };
            ui::gradient_chrome::render_gradient_frame_canvas(
                custom_override,
                radius_px,
                border_px,
                0.0,
                0.0,
                false,
                inner_wash_alpha,
                border_alpha,
                cx,
            )
        }
    };
    let Some(frame_canvas) = frame_canvas else {
        return content;
    };

    let mut content_wrap = div().relative().w_full().child(content);
    if fill_height {
        content_wrap = content_wrap.flex_1().h_full().flex().flex_col();
    }

    let mut outer = div()
        .relative()
        .w_full()
        .p(px(border_px))
        .rounded(px(radius_px))
        .shadow_sm()
        .child(frame_canvas)
        .child(content_wrap);
    if fill_height {
        outer = outer.flex_1().h_full().flex().flex_col();
    }

    outer.into_any_element()
}

/// Wraps `content` in VS Code's signature GPU-SDF rounded, live-breathing 4-color gradient border
/// frame (`1.5px` prism perimeter around a frosted translucent glass surface with crisp content on
/// top).
pub fn wrap_gradient_frame(
    region: Option<PanelRegion>,
    radius_px: f32,
    border_px: f32,
    fill_height: bool,
    content: AnyElement,
    cx: &App,
) -> AnyElement {
    wrap_gradient_frame_inner(region, radius_px, border_px, fill_height, None, content, cx)
}

/// Wraps `content` in a **live-breathing**, rounded 4-color gradient border frame with a **solid**
/// opaque interior backing plus breathing 4-color wash (ideal for the Agent chatbox).
pub fn wrap_breathing_solid_gradient_frame(
    region: Option<PanelRegion>,
    radius_px: f32,
    border_px: f32,
    fill_height: bool,
    content: AnyElement,
    cx: &App,
) -> AnyElement {
    wrap_gradient_frame_inner(
        region,
        radius_px,
        border_px,
        fill_height,
        Some(true),
        content,
        cx,
    )
}

/// Wraps `content` in a **non-breathing**, rounded 4-color gradient border frame with a **solid**
/// interior background so compact textboxes stay calm while typing.
pub fn wrap_static_solid_gradient_frame(
    region: Option<PanelRegion>,
    radius_px: f32,
    border_px: f32,
    fill_height: bool,
    content: AnyElement,
    cx: &App,
) -> AnyElement {
    wrap_gradient_frame_inner(
        region,
        radius_px,
        border_px,
        fill_height,
        Some(false),
        content,
        cx,
    )
}

/// Renders a decorative 4-corner radial aurora backdrop inside the Gradient Studio card itself.
pub fn render_studio_card_backdrop(stops: FourStops, is_light: bool) -> AnyElement {
    let base_scale = if is_light { 0.85 } else { 1.05 };
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, cx| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            if width <= 0.0 || height <= 0.0 {
                return;
            }
            window.paint_dynamic(cx, move |window, cx| {
                let (live_stops, live_angle, live_scale, focal) = cx
                    .try_global::<GradientStudioState>()
                    .filter(|s| s.enabled)
                    .map(|state| {
                        let any_motion = state.has_any_motion();
                        if let Some(delta) = state.current_breathe_delta().filter(|_| any_motion) {
                            let color_delta = state.color_delta_for(delta);
                            evaluate_aurora_frame(
                                delta,
                                color_delta,
                                stops,
                                135.0,
                                base_scale,
                                state.breathe_cfg,
                            )
                        } else {
                            (
                                stops,
                                135.0,
                                base_scale,
                                compute_focal_points(None, state.breathe_cfg),
                            )
                        }
                    })
                    .unwrap_or_else(|| {
                        (
                            stops,
                            135.0,
                            base_scale,
                            [(0.14, 0.14), (0.86, 0.22), (0.20, 0.86), (0.86, 0.86)],
                        )
                    });
                paint_aurora_shape(
                    bounds,
                    live_stops,
                    live_angle,
                    live_scale,
                    focal,
                    AuroraShape::FullMesh,
                    window,
                );
            });
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

fn evaluate_aurora_frame(
    delta: f32,
    color_delta: f32,
    stops: FourStops,
    base_angle: f32,
    base_scale: f32,
    cfg: BreatheConfig,
) -> (FourStops, f32, f32, [(f32, f32); 4]) {
    let phase = delta * TAU;
    let color_phase = color_delta * TAU;
    let focal = compute_focal_points(Some(phase), cfg);
    let pulse_mult = if cfg.pulse {
        1.0 + 0.18 * (phase * 2.0).sin()
    } else {
        1.0
    };
    let angle = if cfg.shimmer {
        (base_angle + 28.0 * color_phase.sin()).rem_euclid(360.0)
    } else {
        base_angle
    };
    let mut active_stops = if cfg.auto_morph && !CURATED_PALETTES.is_empty() {
        let ring = GradientStudioState::morph_ring();
        ui::gradient_chrome::sample_morph_ring(stops, &ring, color_delta)
    } else {
        stops
    };
    if cfg.shimmer {
        active_stops = ui::gradient_chrome::shimmer_shift_stops(active_stops, color_delta * 4.0);
    }
    (active_stops, angle, base_scale * pulse_mult, focal)
}

fn schedule_breathe_repaint_if_needed(window: &Window, cx: &App) {
    let breathe_active = cx
        .try_global::<GradientStudioState>()
        .is_some_and(|state| {
            state.enabled && state.breathe && state.has_any_motion() && window.is_window_active()
        });
    if !breathe_active {
        return;
    }

    let win_id = window.window_handle().window_id();
    let should_spawn = PENDING_BREATHE_REFRESHES.with(|set| set.borrow_mut().insert(win_id));
    if should_spawn {
        let interval = Duration::from_secs_f32(1.0 / 24.0);
        window
            .spawn(cx, async move |cx| {
                cx.background_executor().timer(interval).await;
                PENDING_BREATHE_REFRESHES.with(|set| {
                    set.borrow_mut().remove(&win_id);
                });
                let _ = cx.update(|window, _| {
                    window.repaint_cached();
                });
            })
            .detach();
    }
}

fn render_aurora_element(
    state: &GradientStudioState,
    region: Option<PanelRegion>,
    shape: AuroraShape,
    opaque_base: Option<Hsla>,
    viewport_locked: bool,
    _window: &mut Window,
    cx: &App,
) -> AnyElement {
    let is_light = cx.theme().appearance() == Appearance::Light;
    // Because the aurora renders BEHIND text & buttons, we can match VS Code's full lush vibrancy
    // without ever washing out code or UI contrast.
    let base_scale = if is_light { 0.90 } else { 1.00 } * state.intensity;
    let stops = state.resolved_stops(region);
    let base_angle = state.base_angle;
    let cfg = state.breathe_cfg;

    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, cx| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            if width <= 0.0 || height <= 0.0 {
                return;
            }

            // Paint dynamically so 24 FPS Breathe ticks use `window.repaint_cached()` instead of
            // `window.refresh()`, keeping Dock/Pane/ThreadView/Editor layout & prepaint caches
            // completely hot while every gradient element breathes in smooth lockstep.
            window.paint_dynamic(cx, move |window, cx| {
                schedule_breathe_repaint_if_needed(window, cx);

                if let Some(bg) = opaque_base {
                    window.paint_quad(fill(bounds, bg));
                }

                let (active_stops, angle, scale, focal) = cx
                    .try_global::<GradientStudioState>()
                    .filter(|s| s.enabled)
                    .map(|live_state| {
                        let live_stops = live_state.resolved_stops(region);
                        let live_cfg = live_state.breathe_cfg;
                        let live_scale = if cx.theme().appearance() == Appearance::Light {
                            0.90
                        } else {
                            1.00
                        } * live_state.intensity;
                        if let Some(delta) = live_state.current_breathe_delta() {
                            let color_delta = live_state.color_delta_for(delta);
                            evaluate_aurora_frame(
                                delta,
                                color_delta,
                                live_stops,
                                live_state.base_angle,
                                live_scale,
                                live_cfg,
                            )
                        } else {
                            (
                                live_stops,
                                live_state.base_angle,
                                live_scale,
                                compute_focal_points(None, live_cfg),
                            )
                        }
                    })
                    .unwrap_or_else(|| {
                        (stops, base_angle, base_scale, compute_focal_points(None, cfg))
                    });

                if viewport_locked {
                    let viewport_bounds = Bounds {
                        origin: point(px(0.0), px(0.0)),
                        size: window.viewport_size(),
                    };
                    window.with_content_mask(Some(ContentMask { bounds }), |window| {
                        paint_aurora_shape(
                            viewport_bounds,
                            active_stops,
                            angle,
                            scale,
                            focal,
                            shape,
                            window,
                        );
                    });
                } else {
                    paint_aurora_shape(bounds, active_stops, angle, scale, focal, shape, window);
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

fn compute_focal_points(phase_radians: Option<f32>, cfg: BreatheConfig) -> [(f32, f32); 4] {
    let base = [(0.15, 0.15), (0.85, 0.25), (0.25, 0.85), (0.85, 0.85)];
    let Some(t) = phase_radians else {
        return base;
    };
    if !cfg.full_wave {
        return base;
    }
    // Use zero-at-origin integer harmonics of TAU so both `None -> Some(0.0)` when turning
    // Breathe on AND `1.0 -> 0.0` cycle wraps are 100% C1-continuous with zero focal jumps.
    [
        (
            base[0].0 + 0.08 * t.sin(),
            base[0].1 + 0.06 * (2.0 * t).sin(),
        ),
        (
            base[1].0 - 0.07 * (2.0 * t).sin(),
            base[1].1 + 0.08 * t.sin(),
        ),
        (
            base[2].0 + 0.08 * (2.0 * t).sin(),
            base[2].1 - 0.06 * t.sin(),
        ),
        (
            base[3].0 - 0.07 * t.sin(),
            base[3].1 - 0.07 * (2.0 * t).sin(),
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
    ui::gradient_chrome::paint_rounded_four_stop_band(
        bounds,
        px(0.0),
        stops,
        alphas,
        horizontal,
        window,
    );
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
