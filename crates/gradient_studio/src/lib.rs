pub mod overlay;
pub mod palettes;
pub mod state;
pub mod status_bar_item;
pub mod theme_glass;

use gpui::{App, actions};

pub use overlay::{
    GradientEdge, render_aurora_overlay, render_gradient_divider, render_gradient_edge,
    render_studio_card_backdrop, render_zoomed_aurora_backdrop, wrap_breathing_solid_gradient_frame,
    wrap_gradient_frame, wrap_static_solid_gradient_frame,
};
pub use palettes::{
    CURATED_PALETTES, CuratedPalette, CustomPalette, DEFAULT_PALETTE_NAME, DEFAULT_STOPS,
    PaletteCategory,
};
pub use state::{BreatheConfig, GradientStudioState, PanelMask, PanelRegion, update_state};
pub use status_bar_item::{GradientStudioButton, GradientStudioPopover};
pub use theme_glass::{canvas_background, is_enabled};

actions!(
    gradient_studio,
    [
        /// Opens or closes the Gradient Studio popover in the status bar.
        ToggleStudio,
        /// Rolls a fresh random 4-color gradient palette.
        RollRandom,
        /// Restores the previously active gradient palette.
        PreviousPalette,
        /// Toggles smooth Breathing Aurora animation.
        ToggleBreathe,
        /// Switches between Seamless full-window wash and Per-Panel mode.
        ToggleSeamless,
        /// Shows or hides the gradient panel border lines.
        ToggleBorders,
        /// Rotates the 4 corner colors of the active gradient palette.
        SwapCorners,
        /// Increases the gradient overlay intensity.
        IncreaseIntensity,
        /// Decreases the gradient overlay intensity.
        DecreaseIntensity,
        /// Toggles the gradient overlay on or off.
        ToggleOverlay,
    ]
);

pub fn init(cx: &mut App) {
    let state = GradientStudioState::load();
    cx.set_global(state);
    theme_glass::init(cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::{parse_four_hex_colors, rgb_to_hex};

    #[test]
    fn curated_palettes_count_and_default() {
        assert_eq!(CURATED_PALETTES.len(), 30);
        assert_eq!(CURATED_PALETTES[0].name, DEFAULT_PALETTE_NAME);
        assert_eq!(CURATED_PALETTES[0].stops, DEFAULT_STOPS);
    }

    #[test]
    fn hex_parsing_and_formatting() {
        let parsed = parse_four_hex_colors("#EA4335 #FBBC04 #34A853 #4285F4").unwrap();
        assert_eq!(parsed, DEFAULT_STOPS);
        assert_eq!(rgb_to_hex(parsed[0]), "#EA4335");
        assert!(parse_four_hex_colors("#EA4335 #FBBC04").is_none());
    }

    #[test]
    fn intensity_and_corner_swap_behave_consistently() {
        let mut state = GradientStudioState::default();
        assert_eq!(state.intensity, 1.0);
        state.adjust_intensity(0.5);
        assert!((state.intensity - 1.5).abs() < 0.01);
        state.adjust_intensity(10.0);
        assert_eq!(state.intensity, 3.0);
        state.adjust_intensity(-10.0);
        assert_eq!(state.intensity, 0.2);

        let original = state.stops;
        state.swap_corners();
        assert_eq!(state.stops[0], original[1]);
        assert_eq!(state.stops[3], original[0]);
        assert!(state.previous_palette());
        assert_eq!(state.stops, original);
    }

    #[test]
    fn sample_morph_ring_starts_at_selected_palette_not_classic() {
        let ring = GradientStudioState::morph_ring();
        let sakura = CURATED_PALETTES[2].stops;
        assert_ne!(sakura, DEFAULT_STOPS);
        assert_eq!(ui::gradient_chrome::sample_morph_ring(sakura, &ring, 0.0), sakura);

        let custom = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
        assert_eq!(ui::gradient_chrome::sample_morph_ring(custom, &ring, 0.0), custom);
    }

    #[test]
    fn breathe_toggle_round_trip_preserves_colors_and_focal_points() {
        use std::f32::consts::TAU;
        use std::time::{Duration, Instant};

        let mut state = GradientStudioState::default();
        let sakura = CURATED_PALETTES[2].stops;
        state.select_palette("Sakura", sakura);
        state.breathe_cfg.auto_morph = true;
        state.breathe_cfg.shimmer = true;
        state.breathe_cfg.full_wave = true;
        state.breathe_cfg.pulse = true;

        // Turning Breathe ON must begin seamlessly at Sakura, NOT jump to Gradient Classic.
        state.toggle_breathe();
        let d_start = state.current_breathe_delta().unwrap();
        let start_stops = state.evaluate_stops_at(state.stops, Some(state.color_delta_for(d_start)));
        for c in 0..4 {
            for ch in 0..3 {
                assert!(
                    (start_stops[c][ch] as i16 - sakura[c][ch] as i16).abs() <= 2,
                    "unexpected color jump when enabling Breathe: {start_stops:?} vs {sakura:?}"
                );
            }
        }
        let base_focal = state.focal_points_at(None);
        let start_focal = state.focal_points_at(Some(d_start * TAU));
        for i in 0..4 {
            assert!((start_focal[i].0 - base_focal[i].0).abs() < 0.005);
            assert!((start_focal[i].1 - base_focal[i].1).abs() < 0.005);
        }

        // Advance time 6s so Auto-Morph + Shimmer have moved far away from Sakura / Classic.
        state.breathe_origin = Instant::now() - Duration::from_secs(6);
        let d_mid = state.current_breathe_delta().unwrap();
        let c_mid = state.color_delta_for(d_mid);
        let mid_stops = state.evaluate_stops_at(state.stops, Some(c_mid));
        assert_ne!(mid_stops, DEFAULT_STOPS);
        assert_ne!(mid_stops, sakura);

        // Turning Breathe OFF preserves exact frozen phase and stops.
        state.toggle_breathe();
        let d_paused = state.current_breathe_delta().unwrap();
        assert!((d_paused - d_mid).abs() < 0.01);
        let paused_stops =
            state.evaluate_stops_at(state.stops, Some(state.color_delta_for(d_paused)));
        assert_eq!(paused_stops, mid_stops);

        // Turning Breathe back ON resumes continuously from paused position.
        state.toggle_breathe();
        let d_resumed = state.current_breathe_delta().unwrap();
        assert!((d_resumed - d_paused).abs() < 0.01);
    }

    #[test]
    fn toggling_shimmer_and_auto_morph_bakes_live_colors_without_motion_jump() {
        use std::time::{Duration, Instant};

        let mut state = GradientStudioState::default();
        state.breathe = true;
        state.breathe_cfg.auto_morph = true;
        state.breathe_cfg.shimmer = true;
        state.breathe_cfg.full_wave = true;
        state.breathe_cfg.pulse = true;
        state.breathe_origin = Instant::now() - Duration::from_secs(7);

        let motion_before = state.current_breathe_delta().unwrap();
        let stops_before =
            state.evaluate_stops_at(state.stops, Some(state.color_delta_for(motion_before)));
        assert_ne!(stops_before, DEFAULT_STOPS);

        // Turning Auto-Morph OFF keeps visible colors instead of snapping to Gradient Classic,
        // and keeps Full Wave / Pulse motion_delta continuous.
        state.toggle_breathe_auto_morph();
        let motion_after_morph = state.current_breathe_delta().unwrap();
        let stops_after_morph = state.evaluate_stops_at(
            state.stops,
            Some(state.color_delta_for(motion_after_morph)),
        );
        assert_eq!(stops_after_morph, stops_before);
        assert!((motion_after_morph - motion_before).abs() < 0.01);

        // Turning Shimmer OFF likewise preserves visible colors and motion phase.
        state.toggle_breathe_shimmer();
        let motion_after_shimmer = state.current_breathe_delta().unwrap();
        let stops_after_shimmer = state.evaluate_stops_at(
            state.stops,
            Some(state.color_delta_for(motion_after_shimmer)),
        );
        assert_eq!(stops_after_shimmer, stops_before);
        assert!((motion_after_shimmer - motion_before).abs() < 0.01);
    }

    #[test]
    fn borders_panel_region_toggles_and_custom_palette() {
        let mut state = GradientStudioState::default();
        assert!(PanelRegion::ALL.contains(&PanelRegion::Borders));
        assert!(state.panels.is_enabled(PanelRegion::Borders));
        assert!(state.panels.borders);

        state.toggle_borders();
        assert!(!state.panels.is_enabled(PanelRegion::Borders));
        assert!(!state.panels.borders);

        state.toggle_panel(PanelRegion::Borders);
        assert!(state.panels.is_enabled(PanelRegion::Borders));

        let custom = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
        state.select_palette_for(Some(PanelRegion::Borders), "Subtle", custom);
        assert_eq!(state.resolved_stops(Some(PanelRegion::Borders)), custom);
        state.clear_component_palette(PanelRegion::Borders);
        assert_eq!(state.resolved_stops(Some(PanelRegion::Borders)), state.stops);
    }

    #[test]
    fn breathe_fps_scales_gently_between_10_and_16_fps() {
        assert_eq!(overlay::breathe_fps_for_speed(25.0), 10.0);
        assert_eq!(overlay::breathe_fps_for_speed(60.0), 10.0);
        assert_eq!(overlay::breathe_fps_for_speed(100.0), 12.0);
        assert_eq!(overlay::breathe_fps_for_speed(200.0), 16.0);
        assert_eq!(overlay::breathe_fps_for_speed(250.0), 16.0);
    }

    #[gpui::test]
    fn repaint_cached_updates_dynamic_gradient_without_rerendering_cached_child(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::{
            AppContext as _, Bounds, Context, Entity, Global, IntoElement, ParentElement, Render,
            ScaledPixels, StyleRefinement, Styled, Window, canvas, div, fill, hsla, px, size,
        };
        use std::cell::Cell;
        use std::rc::Rc;

        struct TestHue(Cell<f32>);
        impl Global for TestHue {}

        struct CachedChild {
            render_count: Rc<Cell<usize>>,
            dynamic_paint_count: Rc<Cell<usize>>,
        }

        impl Render for CachedChild {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                self.render_count.set(self.render_count.get() + 1);
                let dynamic_paint_count = self.dynamic_paint_count.clone();
                div().relative().size(px(100.)).child(
                    div().size(px(40.)).overflow_hidden().child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, (), window, cx| {
                                window.paint_quad(fill(bounds, hsla(0.1, 1.0, 0.5, 1.0)));
                                let count = dynamic_paint_count.clone();
                                window.paint_dynamic(cx, move |window, cx| {
                                    count.set(count.get() + 1);
                                    let hue = cx.global::<TestHue>().0.get();
                                    window.paint_quad(fill(
                                        Bounds::new(bounds.origin, size(px(100.), px(100.))),
                                        hsla(hue, 1.0, 0.5, 1.0),
                                    ));
                                });
                                window.paint_quad(fill(bounds, hsla(0.9, 1.0, 0.5, 1.0)));
                            },
                        )
                        .size_full(),
                    ),
                )
            }
        }

        struct Root {
            child: Entity<CachedChild>,
        }

        impl Render for Root {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                self.child
                    .clone()
                    .cached(StyleRefinement::default().size(px(100.)))
            }
        }

        cx.update(|cx| cx.set_global(TestHue(Cell::new(0.25))));
        let render_count = Rc::new(Cell::new(0));
        let dynamic_paint_count = Rc::new(Cell::new(0));

        let window = cx.add_window({
            let render_count = render_count.clone();
            let dynamic_paint_count = dynamic_paint_count.clone();
            move |_, cx| {
                let child = cx.new(|_| CachedChild {
                    render_count,
                    dynamic_paint_count,
                });
                Root { child }
            }
        });

        let (baseline_renders, baseline_paints) = cx
            .update_window(window.into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                let renders = render_count.get();
                let paints = dynamic_paint_count.get();
                assert!(renders >= 1);
                assert!(paints >= 1);
                let quads = window.painted_quads();
                assert_eq!(quads.len(), 3);
                assert!(quads[0].order <= quads[1].order);
                assert!(quads[1].order <= quads[2].order);
                assert!((quads[0].background.as_solid().unwrap().h - 0.1).abs() < 1e-4);
                assert!((quads[1].background.as_solid().unwrap().h - 0.25).abs() < 1e-4);
                assert!((quads[2].background.as_solid().unwrap().h - 0.9).abs() < 1e-4);
                let expected_mask = size(
                    ScaledPixels(40. * window.scale_factor()),
                    ScaledPixels(40. * window.scale_factor()),
                );
                assert_eq!(quads[1].content_mask.bounds.size, expected_mask);
                (renders, paints)
            })
            .unwrap();

        for step in 1..=50 {
            let next_hue = (step as f32 * 0.015) % 1.0;
            cx.update(|cx| cx.global::<TestHue>().0.set(next_hue));
            cx.update_window(window.into(), |_, window, cx| {
                window.repaint_cached();
                window.draw(cx).clear(cx);
                assert_eq!(
                    render_count.get(),
                    baseline_renders,
                    "cached child view must NOT re-render during repaint_cached"
                );
                assert_eq!(dynamic_paint_count.get(), baseline_paints + step);
                let quads = window.painted_quads();
                assert_eq!(quads.len(), 3);
                assert!(quads[0].order <= quads[1].order);
                assert!(quads[1].order <= quads[2].order);
                assert!((quads[0].background.as_solid().unwrap().h - 0.1).abs() < 1e-4);
                assert!((quads[1].background.as_solid().unwrap().h - next_hue).abs() < 1e-4);
                assert!((quads[2].background.as_solid().unwrap().h - 0.9).abs() < 1e-4);
                let expected_mask = size(
                    ScaledPixels(40. * window.scale_factor()),
                    ScaledPixels(40. * window.scale_factor()),
                );
                assert_eq!(quads[1].content_mask.bounds.size, expected_mask);
            })
            .unwrap();
        }

        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            assert_eq!(render_count.get(), baseline_renders + 1);
            assert_eq!(dynamic_paint_count.get(), baseline_paints + 51);
        })
        .unwrap();
    }

    #[gpui::test]
    fn glass_theme_preserves_opaque_raw_colors_for_mermaid_during_theme_notifications(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::Arc;
        use theme::{ActiveTheme as _, Appearance, GlobalTheme, ThemeColors};

        let mut light_colors = ThemeColors::light();
        light_colors.editor_background = gpui::rgb(0xfafafa).into();
        light_colors.panel_background = gpui::rgb(0xebebec).into();
        light_colors.border = gpui::rgb(0xc9c9ca).into();
        light_colors.border_variant = gpui::rgb(0xdfdfe0).into();
        light_colors.text_muted = gpui::rgb(0x58585a).into();

        let base_theme = theme::ThemeRegistry::new(Box::new(()))
            .get(theme::DEFAULT_DARK_THEME)
            .unwrap();

        let mut raw_light_theme = (*base_theme).clone();
        raw_light_theme.id = "one-light-test".to_string();
        raw_light_theme.name = "One Light Test".into();
        raw_light_theme.appearance = Appearance::Light;
        raw_light_theme.styles.colors = light_colors;
        let raw_light = Arc::new(raw_light_theme);

        let mut dark_colors = ThemeColors::dark();
        dark_colors.editor_background = gpui::rgb(0x282c33).into();
        dark_colors.panel_background = gpui::rgb(0x2f343e).into();
        dark_colors.border = gpui::rgb(0x464b57).into();
        dark_colors.border_variant = gpui::rgb(0x363c46).into();
        dark_colors.text_muted = gpui::rgb(0xa9afbc).into();

        let mut raw_dark_theme = (*base_theme).clone();
        raw_dark_theme.id = "one-dark-test".to_string();
        raw_dark_theme.name = "One Dark Test".into();
        raw_dark_theme.appearance = Appearance::Dark;
        raw_dark_theme.styles.colors = dark_colors;
        let raw_dark = Arc::new(raw_dark_theme);

        let observed_editor_bg_alphas: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::new()));
        let observed_border_alphas: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::new()));

        cx.update(|cx| {
            cx.set_global(GlobalTheme::new(
                raw_light.clone(),
                theme::default_icon_theme(),
            ));
            cx.set_global(GradientStudioState::default());
            theme_glass::init(cx);

            // Glass theme makes live `cx.theme().colors().editor_background` transparent,
            // while `ui::gradient_chrome::raw_theme_colors(cx)` preserves opaque raw colors.
            assert_eq!(cx.theme().colors().editor_background.a, 0.0);
            let raw = ui::gradient_chrome::raw_theme_colors(cx);
            assert_eq!(raw.editor_background.a, 1.0);
            assert_eq!(raw.panel_background.a, 1.0);
            assert_eq!(raw.border.a, 1.0);
            assert_eq!(raw.border_variant.a, 1.0);
            assert!(raw.editor_background.l > 0.90);

            let line = ui::gradient_chrome::mermaid_line_color(raw.border, raw.text_muted);
            assert_eq!(line.a, 1.0);
            assert!(
                line.l < raw.border.l && line.l > raw.text_muted.l,
                "mermaid_line_color should fall between light border and text_muted, got l={}",
                line.l
            );

            let bg_log = observed_editor_bg_alphas.clone();
            let border_log = observed_border_alphas.clone();
            cx.observe_global::<GlobalTheme>(move |cx| {
                let raw = ui::gradient_chrome::raw_theme_colors(cx);
                bg_log.borrow_mut().push(raw.editor_background.a);
                border_log.borrow_mut().push(raw.border.a);
            })
            .detach();
        });

        // Toggle overlay OFF, then back ON, then switch Light -> Dark while ON:
        // every GlobalTheme notification must expose opaque raw colors (never transparent_black()).
        cx.update(|cx| {
            state::update_state(cx, |s| s.enabled = false);
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(cx.theme().colors().editor_background.a, 1.0);
            state::update_state(cx, |s| s.enabled = true);
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(cx.theme().colors().editor_background.a, 0.0);
            assert!(ui::gradient_chrome::raw_theme_colors(cx).editor_background.l > 0.90);
            GlobalTheme::update_theme(cx, raw_dark.clone());
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(cx.theme().colors().editor_background.a, 0.0);
            let raw = ui::gradient_chrome::raw_theme_colors(cx);
            assert_eq!(raw.editor_background.a, 1.0);
            assert!(
                raw.editor_background.l < 0.30,
                "after switching to dark theme, raw editor_background must reflect dark theme, got l={}",
                raw.editor_background.l
            );
        });

        let bg_alphas = observed_editor_bg_alphas.borrow();
        let border_alphas = observed_border_alphas.borrow();
        assert!(bg_alphas.len() >= 4, "expected theme notifications on toggle and theme switch");
        assert!(
            bg_alphas.iter().all(|&a| a == 1.0),
            "every GlobalTheme notification must expose opaque raw editor_background: {bg_alphas:?}"
        );
        assert!(
            border_alphas.iter().all(|&a| a == 1.0),
            "every GlobalTheme notification must expose opaque raw border: {border_alphas:?}"
        );
    }
}
