use gpui::{
    Anchor, App, Context, DismissEvent, EventEmitter, FocusHandle, Focusable, MouseButton,
    ParentElement, Render, SharedString, Styled, Window, div, linear_color_stop, linear_gradient,
    px,
};
use ui::{
    Button, ButtonSize, ButtonStyle, Color, Icon, IconName, IconSize, Label, LabelSize,
    PopoverMenu, PopoverMenuHandle, TintColor, Tooltip, h_flex, prelude::*, v_flex,
};

use crate::palettes::{CURATED_PALETTES, FourStops, rgb_to_hsla};
use crate::state::{GradientStudioState, PanelRegion, update_state};
use crate::{RollRandom, ToggleStudio};

pub struct GradientStudioButton {
    popover_handle: PopoverMenuHandle<GradientStudioPopover>,
}

impl GradientStudioButton {
    pub fn new(
        popover_handle: PopoverMenuHandle<GradientStudioPopover>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe_global::<GradientStudioState>(|_, cx| {
            cx.notify();
        })
        .detach();
        Self { popover_handle }
    }

    pub fn popover_handle(&self) -> PopoverMenuHandle<GradientStudioPopover> {
        self.popover_handle.clone()
    }
}

impl Render for GradientStudioButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(state) = cx.try_global::<GradientStudioState>() else {
            return div().into_any_element();
        };

        let enabled = state.enabled;
        let palette_name = state.palette_name.clone();
        let stops = state.stops;
        let swatch_bg = linear_gradient(
            135.0,
            linear_color_stop(rgb_to_hsla(stops[0], if enabled { 0.95 } else { 0.35 }), 0.0),
            linear_color_stop(rgb_to_hsla(stops[3], if enabled { 0.95 } else { 0.35 }), 1.0),
        );

        let roll_btn = div()
            .id("gradient-roll-btn")
            .cursor_pointer()
            .px_1()
            .py(px(2.))
            .rounded_sm()
            .hover(|s| s.bg(cx.theme().colors().element_hover))
            .on_mouse_down(MouseButton::Left, |event, _window, cx| {
                if event.modifiers.shift {
                    update_state(cx, |state| state.toggle_enabled());
                } else {
                    update_state(cx, |state| state.roll_random());
                }
            })
            .on_mouse_down(MouseButton::Right, |_event, _window, cx| {
                update_state(cx, |state| state.toggle_enabled());
            })
            .tooltip(|_window, cx| {
                Tooltip::for_action(
                    "Roll Random Gradient (Right-click to toggle overlay)",
                    &RollRandom,
                    cx,
                )
            })
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(div().size(px(8.)).rounded_full().bg(swatch_bg))
                    .child(
                        Label::new("↻")
                            .size(LabelSize::Small)
                            .color(if enabled { Color::Default } else { Color::Muted }),
                    ),
            );

        let studio_label: SharedString = if enabled {
            "Studio ▴".into()
        } else {
            "Studio (Off)".into()
        };

        let studio_trigger = Button::new("gradient-studio-trigger", studio_label)
            .label_size(LabelSize::Small)
            .style(ButtonStyle::Subtle);

        let popover_menu = PopoverMenu::new("gradient-studio-menu")
            .anchor(Anchor::BottomRight)
            .with_handle(self.popover_handle.clone())
            .trigger_with_tooltip(studio_trigger, move |_window, cx| {
                Tooltip::for_action(
                    format!("Gradient Studio ({palette_name})"),
                    &ToggleStudio,
                    cx,
                )
            })
            .menu(|_window, cx| Some(cx.new(GradientStudioPopover::new)));

        h_flex()
            .gap_0p5()
            .items_center()
            .child(roll_btn)
            .child(popover_menu)
            .into_any_element()
    }
}

pub struct GradientStudioPopover {
    focus_handle: FocusHandle,
    show_favs_only_tab: bool,
}

impl GradientStudioPopover {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<GradientStudioState>(|_, cx| {
            cx.notify();
        })
        .detach();
        Self {
            focus_handle: cx.focus_handle(),
            show_favs_only_tab: false,
        }
    }

    fn render_swatch_bar(stops: FourStops, height: f32) -> impl IntoElement {
        h_flex()
            .w_full()
            .h(px(height))
            .rounded_sm()
            .overflow_hidden()
            .child(
                div().flex_1().h_full().bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgb_to_hsla(stops[0], 0.92), 0.0),
                    linear_color_stop(rgb_to_hsla(stops[1], 0.92), 1.0),
                )),
            )
            .child(
                div().flex_1().h_full().bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgb_to_hsla(stops[1], 0.92), 0.0),
                    linear_color_stop(rgb_to_hsla(stops[2], 0.92), 1.0),
                )),
            )
            .child(
                div().flex_1().h_full().bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgb_to_hsla(stops[2], 0.92), 0.0),
                    linear_color_stop(rgb_to_hsla(stops[3], 0.92), 1.0),
                )),
            )
    }
}

impl Focusable for GradientStudioPopover {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for GradientStudioPopover {}

impl Render for GradientStudioPopover {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(state) = cx.try_global::<GradientStudioState>() else {
            return div().into_any_element();
        };

        let enabled = state.enabled;
        let palette_name = state.palette_name.clone();
        let stops = state.stops;
        let intensity_pct = (state.intensity * 100.0).round() as i32;
        let breathe = state.breathe;
        let seamless = state.seamless;
        let panels = state.panels;
        let is_active_fav = state.is_favorite(&palette_name);
        let fav_only_roll = state.fav_only;
        let can_undo = !state.history.is_empty();
        let favorites = state.favorites.clone();

        let colors = cx.theme().colors();
        let show_favs_tab = self.show_favs_only_tab;

        let visible_palettes: Vec<_> = CURATED_PALETTES
            .iter()
            .filter(|p| !show_favs_tab || favorites.iter().any(|f| f == p.name))
            .collect();

        let panel_defs = [
            (PanelRegion::Editor, "Editor"),
            (PanelRegion::LeftDock, "Left"),
            (PanelRegion::RightDock, "Right"),
            (PanelRegion::BottomDock, "Bottom"),
            (PanelRegion::Bars, "Bars"),
        ];

        v_flex()
            .id("gradient-studio-popover")
            .track_focus(&self.focus_handle)
            .occlude()
            .w(px(360.))
            .p_3()
            .gap_2p5()
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .bg(colors.elevated_surface_background)
            .shadow_lg()
            // Header + Live Palette Preview
            .child(
                v_flex()
                    .gap_1p5()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .child(
                                        Icon::new(IconName::Sparkle)
                                            .size(IconSize::Small)
                                            .color(Color::Accent),
                                    )
                                    .child(Label::new("Gradient Studio").size(LabelSize::Default))
                                    .child(
                                        Label::new(palette_name.clone())
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child({
                                        let fav_name = palette_name.clone();
                                        Button::new(
                                            "studio-fav-active",
                                            if is_active_fav { "★" } else { "☆" },
                                        )
                                        .size(ButtonSize::Compact)
                                        .style(if is_active_fav {
                                            ButtonStyle::Tinted(TintColor::Accent)
                                        } else {
                                            ButtonStyle::Subtle
                                        })
                                        .on_click(move |_, _, cx| {
                                            update_state(cx, |s| s.toggle_favorite(&fav_name));
                                        })
                                    })
                                    .child(
                                        Button::new("studio-close", "✕")
                                            .size(ButtonSize::Compact)
                                            .style(ButtonStyle::Subtle)
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                cx.emit(DismissEvent);
                                            })),
                                    ),
                            ),
                    )
                    .child(Self::render_swatch_bar(stops, 10.0)),
            )
            // Primary Controls Row
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .child(
                        Button::new("studio-roll", "↻ Roll")
                            .size(ButtonSize::Compact)
                            .style(ButtonStyle::Tinted(TintColor::Accent))
                            .on_click(|_, _, cx| update_state(cx, |s| s.roll_random())),
                    )
                    .child(
                        Button::new("studio-toggle-overlay", if enabled { "Overlay: On" } else { "Overlay: Off" })
                            .size(ButtonSize::Compact)
                            .style(if enabled {
                                ButtonStyle::Tinted(TintColor::Success)
                            } else {
                                ButtonStyle::Outlined
                            })
                            .on_click(|_, _, cx| update_state(cx, |s| s.toggle_enabled())),
                    )
                    .child(
                        Button::new("studio-breathe", if breathe { "Breathe: On" } else { "Breathe" })
                            .size(ButtonSize::Compact)
                            .style(if breathe {
                                ButtonStyle::Tinted(TintColor::Accent)
                            } else {
                                ButtonStyle::Outlined
                            })
                            .on_click(|_, _, cx| update_state(cx, |s| s.toggle_breathe())),
                    )
                    .child(
                        Button::new(
                            "studio-seamless",
                            if seamless { "Seamless" } else { "Per-Panel" },
                        )
                        .size(ButtonSize::Compact)
                        .style(ButtonStyle::Outlined)
                        .on_click(|_, _, cx| update_state(cx, |s| s.toggle_seamless())),
                    )
                    .child(
                        Button::new("studio-swap", "⇄ Swap")
                            .size(ButtonSize::Compact)
                            .style(ButtonStyle::Subtle)
                            .on_click(|_, _, cx| update_state(cx, |s| s.swap_corners())),
                    )
                    .when(can_undo, |el| {
                        el.child(
                            Button::new("studio-undo", "↩ Undo")
                                .size(ButtonSize::Compact)
                                .style(ButtonStyle::Subtle)
                                .on_click(|_, _, cx| {
                                    update_state(cx, |s| {
                                        s.previous_palette();
                                    });
                                }),
                        )
                    }),
            )
            // Intensity Row
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(colors.surface_background)
                    .child(
                        Label::new(format!("Intensity: {intensity_pct}%"))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("studio-int-down", "−")
                                    .size(ButtonSize::Compact)
                                    .style(ButtonStyle::Outlined)
                                    .on_click(|_, _, cx| {
                                        update_state(cx, |s| s.adjust_intensity(-0.15))
                                    }),
                            )
                            .child(
                                Button::new("studio-int-reset", "100%")
                                    .size(ButtonSize::Compact)
                                    .style(ButtonStyle::Subtle)
                                    .on_click(|_, _, cx| {
                                        update_state(cx, |s| s.reset_intensity())
                                    }),
                            )
                            .child(
                                Button::new("studio-int-up", "+")
                                    .size(ButtonSize::Compact)
                                    .style(ButtonStyle::Outlined)
                                    .on_click(|_, _, cx| {
                                        update_state(cx, |s| s.adjust_intensity(0.15))
                                    }),
                            ),
                    ),
            )
            // Per-Panel toggles (shown when Seamless is off)
            .when(!seamless, |el| {
                el.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .items_center()
                        .children(panel_defs.into_iter().map(|(region, label)| {
                            let active = panels.is_enabled(region);
                            Button::new(SharedString::from(format!("panel-{label}")), label)
                                .size(ButtonSize::Compact)
                                .style(if active {
                                    ButtonStyle::Tinted(TintColor::Accent)
                                } else {
                                    ButtonStyle::Outlined
                                })
                                .on_click(move |_, _, cx| {
                                    update_state(cx, |s| s.toggle_panel(region));
                                })
                        })),
                )
            })
            // Presets Filter Header
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new(
                                    "studio-tab-all",
                                    format!("Presets ({})", CURATED_PALETTES.len()),
                                )
                                .size(ButtonSize::Compact)
                                .style(if !show_favs_tab {
                                    ButtonStyle::Tinted(TintColor::Accent)
                                } else {
                                    ButtonStyle::Subtle
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_favs_only_tab = false;
                                    cx.notify();
                                })),
                            )
                            .child(
                                Button::new(
                                    "studio-tab-favs",
                                    format!("★ Favs ({})", favorites.len()),
                                )
                                .size(ButtonSize::Compact)
                                .style(if show_favs_tab {
                                    ButtonStyle::Tinted(TintColor::Accent)
                                } else {
                                    ButtonStyle::Subtle
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_favs_only_tab = true;
                                    cx.notify();
                                })),
                            ),
                    )
                    .child(
                        Button::new(
                            "studio-fav-roll",
                            if fav_only_roll {
                                "Roll: ★ Only"
                            } else {
                                "Roll: All"
                            },
                        )
                        .size(ButtonSize::Compact)
                        .style(if fav_only_roll {
                            ButtonStyle::Tinted(TintColor::Warning)
                        } else {
                            ButtonStyle::Subtle
                        })
                        .on_click(|_, _, cx| update_state(cx, |s| s.toggle_fav_only())),
                    ),
            )
            // Scrollable Preset Grid (2 columns)
            .child(
                v_flex()
                    .id("gradient-presets-scroll")
                    .max_h(px(260.))
                    .overflow_y_scroll()
                    .gap_1p5()
                    .when(visible_palettes.is_empty(), |el| {
                        el.child(
                            div().py_4().flex().justify_center().child(
                                Label::new("No favorite palettes yet — click ☆ on any preset!")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            ),
                        )
                    })
                    .children(visible_palettes.chunks(2).enumerate().map(|(row_idx, pair)| {
                        h_flex()
                            .id(SharedString::from(format!("preset-row-{row_idx}")))
                            .gap_1p5()
                            .w_full()
                            .children(pair.iter().map(|preset| {
                                let name = preset.name;
                                let preset_stops = preset.stops;
                                let is_selected = enabled && palette_name == name;
                                let is_fav = favorites.iter().any(|f| f == name);

                                v_flex()
                                    .id(SharedString::from(format!("preset-{name}")))
                                    .flex_1()
                                    .p_1p5()
                                    .gap_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .border_1()
                                    .border_color(if is_selected {
                                        colors.border_focused
                                    } else {
                                        colors.border_variant
                                    })
                                    .bg(if is_selected {
                                        colors.element_selected
                                    } else {
                                        colors.surface_background
                                    })
                                    .hover(|s| s.bg(colors.element_hover))
                                    .on_click(move |_, _, cx| {
                                        update_state(cx, |s| s.select_palette(name, preset_stops));
                                    })
                                    .child(Self::render_swatch_bar(preset_stops, 16.0))
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(
                                                Label::new(name)
                                                    .size(LabelSize::XSmall)
                                                    .color(if is_selected {
                                                        Color::Accent
                                                    } else {
                                                        Color::Default
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "preset-fav-{name}"
                                                    )))
                                                    .px_0p5()
                                                    .rounded_sm()
                                                    .hover(|s| s.bg(colors.element_active))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        move |_, _, cx| {
                                                            cx.stop_propagation();
                                                            update_state(cx, |s| {
                                                                s.toggle_favorite(name)
                                                            });
                                                        },
                                                    )
                                                    .child(
                                                        Label::new(if is_fav { "★" } else { "☆" })
                                                            .size(LabelSize::XSmall)
                                                            .color(if is_fav {
                                                                Color::Accent
                                                            } else {
                                                                Color::Muted
                                                            }),
                                                    ),
                                            ),
                                    )
                            }))
                    })),
            )
            .into_any_element()
    }
}
