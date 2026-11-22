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
}
