use std::sync::Arc;

use gpui::{App, BorrowAppContext as _, Global, Hsla, transparent_black};
use theme::{ActiveTheme as _, Appearance, GlobalTheme, Theme};

use crate::palettes::CURATED_PALETTES;
use crate::state::GradientStudioState;

const GLASS_THEME_SUFFIX: &str = "#gradient-glass";

struct GradientThemeGlass {
    raw_theme: Option<Arc<Theme>>,
    canvas_bg: Hsla,
}

impl Global for GradientThemeGlass {}

fn compute_canvas_bg(theme: &Theme) -> Hsla {
    let fallback: Hsla = match theme.appearance {
        Appearance::Light => gpui::rgb(0xffffff).into(),
        Appearance::Dark => gpui::rgb(0x111722).into(),
    };
    let base = fallback.blend(theme.colors().background);
    let mut canvas = base.blend(theme.colors().editor_background);
    canvas.a = 1.0;
    canvas
}

/// Returns the active theme's solid opaque canvas background color (matching VS Code's
/// `--g-canvas-bg`), preserved before glass transparency is applied so the root window and
/// split/zoomed panels always have a crisp opaque base beneath the aurora underlay.
pub fn canvas_background(cx: &App) -> Hsla {
    if let Some(glass) = cx.try_global::<GradientThemeGlass>() {
        glass.canvas_bg
    } else {
        compute_canvas_bg(cx.theme())
    }
}

/// Returns whether Gradient Zed's aurora backdrop engine is currently enabled.
pub fn is_enabled(cx: &App) -> bool {
    cx.try_global::<GradientStudioState>()
        .is_some_and(|s| s.enabled)
}

/// Synchronizes the lightweight `ui::gradient_chrome::GradientChromeState` global so every
/// UI component (`ui`, `markdown`, `editor`, `project_panel`, `agent_ui`) breathes in lockstep
/// with Gradient Studio's palette, speed, pulse, shimmer, auto-morph, and solid canvas base.
pub fn sync_chrome_state(cx: &mut App) {
    let Some(state) = cx.try_global::<GradientStudioState>() else {
        return;
    };
    let speed_mult = (state.breathe_cfg.speed_pct / 100.0).clamp(0.25, 2.5);
    let cycle_secs = (18.0 / speed_mult).clamp(5.0, 72.0);
    let canvas_bg = canvas_background(cx);
    let is_light = cx.has_global::<GlobalTheme>()
        && GlobalTheme::theme(cx).appearance == Appearance::Light;
    let morph_palettes: Vec<ui::gradient_chrome::FourStops> = CURATED_PALETTES
        .iter()
        .take(12)
        .map(|p| p.stops)
        .collect();

    let next = ui::gradient_chrome::GradientChromeState {
        enabled: state.enabled,
        stops: state.stops,
        breathe: state.breathe,
        breathe_origin: state.breathe_origin,
        breathe_paused_delta: state.breathe_paused_delta,
        breathe_color_offset: state.breathe_color_offset,
        breathe_cycle_secs: cycle_secs,
        breathe_pulse: state.breathe_cfg.pulse,
        breathe_shimmer: state.breathe_cfg.shimmer,
        breathe_auto_morph: state.breathe_cfg.auto_morph,
        intensity: state.intensity,
        canvas_bg,
        is_light,
        morph_palettes,
    };

    if cx.has_global::<ui::gradient_chrome::GradientChromeState>() {
        cx.update_global::<ui::gradient_chrome::GradientChromeState, _>(|chrome, _| {
            *chrome = next;
        });
    } else {
        cx.set_global(next);
    }
}

/// Initializes observers that keep structural panel backgrounds transparent while Gradient Aurora
/// is active (so the aurora renders *behind* text, code, buttons, and icons at 100% contrast),
/// and cleanly restores the original opaque theme whenever the overlay is toggled off.
pub fn init(cx: &mut App) {
    let initial_canvas = if cx.has_global::<GlobalTheme>() {
        compute_canvas_bg(GlobalTheme::theme(cx))
    } else {
        gpui::rgb(0x111722).into()
    };

    cx.set_global(GradientThemeGlass {
        raw_theme: None,
        canvas_bg: initial_canvas,
    });

    sync_theme_glass(cx);
    sync_chrome_state(cx);

    cx.observe_global::<GlobalTheme>(|cx| {
        cx.defer(sync_theme_glass);
    })
    .detach();

    cx.observe_global::<GradientStudioState>(|cx| {
        cx.defer(sync_theme_glass);
    })
    .detach();
}

pub fn sync_theme_glass(cx: &mut App) {
    if !cx.has_global::<GlobalTheme>()
        || !cx.has_global::<GradientStudioState>()
        || !cx.has_global::<GradientThemeGlass>()
    {
        return;
    }

    let enabled = cx.global::<GradientStudioState>().enabled;
    let current = GlobalTheme::theme(cx).clone();
    let is_currently_glass = current.id.ends_with(GLASS_THEME_SUFFIX);

    if !is_currently_glass {
        let canvas_bg = compute_canvas_bg(&current);
        let raw_theme = current.clone();
        cx.update_global::<GradientThemeGlass, _>(|glass, _| {
            glass.raw_theme = Some(raw_theme);
            glass.canvas_bg = canvas_bg;
        });
    }

    if enabled {
        if is_currently_glass {
            sync_chrome_state(cx);
            return;
        }
        let Some(raw) = cx.global::<GradientThemeGlass>().raw_theme.clone() else {
            sync_chrome_state(cx);
            return;
        };
        let canvas_bg = cx.global::<GradientThemeGlass>().canvas_bg;
        let glass_theme = build_glass_theme(&raw, canvas_bg);
        GlobalTheme::update_theme(cx, Arc::new(glass_theme));
        sync_chrome_state(cx);
        cx.refresh_windows();
    } else if is_currently_glass {
        if let Some(raw) = cx.global::<GradientThemeGlass>().raw_theme.clone() {
            GlobalTheme::update_theme(cx, raw);
            sync_chrome_state(cx);
            cx.refresh_windows();
        }
    } else {
        sync_chrome_state(cx);
    }
}

fn build_glass_theme(raw: &Theme, canvas_bg: Hsla) -> Theme {
    let mut glass = raw.clone();
    glass.id = format!("{}{}", raw.id, GLASS_THEME_SUFFIX);

    let is_light = glass.appearance == Appearance::Light;
    let mut panel_overlay = canvas_bg.blend(raw.colors().panel_background);
    panel_overlay.a = 1.0;

    // Make structural panel containers transparent so the aurora renders BEHIND text, icons,
    // buttons, and syntax tokens — never washing them out. Soften chalky opaque borders &
    // ghost selection pills into frosted aurora glass while keeping `surface_background`,
    // `elevated_surface_background`, `element_*`, `text*`, `icon*`, and `syntax` razor-sharp.
    let c = &mut glass.styles.colors;
    c.panel_overlay_background = panel_overlay;
    c.background = transparent_black();
    c.panel_background = transparent_black();
    c.editor_background = transparent_black();
    c.editor_gutter_background = transparent_black();
    c.editor_subheader_background = canvas_bg.opacity(if is_light { 0.45 } else { 0.42 });
    c.tab_bar_background = transparent_black();
    c.tab_inactive_background = transparent_black();
    c.tab_active_background = canvas_bg.opacity(if is_light { 0.72 } else { 0.62 });
    c.toolbar_background = transparent_black();
    c.title_bar_background = transparent_black();
    c.title_bar_inactive_background = transparent_black();
    c.status_bar_background = transparent_black();
    c.terminal_background = transparent_black();

    // Frosted translucent hairlines & sidebar row highlights (matches VS Code glass aesthetic)
    if is_light {
        c.border = gpui::rgba(0x0f172a1c).into();
        c.border_variant = gpui::rgba(0x0f172a12).into();
        c.pane_group_border = gpui::rgba(0x0f172a1c).into();
        c.ghost_element_hover = gpui::rgba(0xffffff58).into();
        c.ghost_element_active = gpui::rgba(0xffffff80).into();
        c.ghost_element_selected = gpui::rgba(0xffffff8c).into();
    } else {
        c.border = gpui::rgba(0xffffff1f).into();
        c.border_variant = gpui::rgba(0xffffff14).into();
        c.pane_group_border = gpui::rgba(0xffffff1f).into();
        c.ghost_element_hover = gpui::rgba(0xffffff14).into();
        c.ghost_element_active = gpui::rgba(0xffffff22).into();
        c.ghost_element_selected = gpui::rgba(0xffffff1c).into();
    }

    glass
}
