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
}
