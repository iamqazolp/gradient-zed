use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Anchor, AnyElement, App, Bounds, Context, DismissEvent, EventEmitter, FocusHandle, Focusable,
    FontWeight, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement,
    Pixels, Render, SharedString, Styled, Window, canvas, div, linear_color_stop, linear_gradient,
    px, relative, rgb, rgba,
};
use theme::Appearance;
use ui::{
    ButtonLike, ButtonStyle, Color, Label, LabelSize, PopoverMenu, PopoverMenuHandle, Tooltip,
    h_flex, prelude::*, v_flex,
};

use crate::overlay::render_studio_card_backdrop;
use crate::palettes::{
    CURATED_PALETTES, FourStops, PaletteCategory, hsl_to_rgb, parse_four_hex_colors, rgb_to_hex,
    rgb_to_hsl, rgb_to_hsla,
};
use crate::state::{GradientStudioState, PanelRegion, update_state};
use crate::{RollRandom, ToggleBreathe, ToggleStudio};

fn render_gradient_bar(stops: FourStops, height: f32, alpha: f32) -> AnyElement {
    h_flex()
        .w_full()
        .h(px(height))
        .rounded_full()
        .overflow_hidden()
        .child(
            div().flex_1().h_full().bg(linear_gradient(
                90.0,
                linear_color_stop(rgb_to_hsla(stops[0], alpha), 0.0),
                linear_color_stop(rgb_to_hsla(stops[1], alpha), 1.0),
            )),
        )
        .child(
            div().flex_1().h_full().bg(linear_gradient(
                90.0,
                linear_color_stop(rgb_to_hsla(stops[1], alpha), 0.0),
                linear_color_stop(rgb_to_hsla(stops[2], alpha), 1.0),
            )),
        )
        .child(
            div().flex_1().h_full().bg(linear_gradient(
                90.0,
                linear_color_stop(rgb_to_hsla(stops[2], alpha), 0.0),
                linear_color_stop(rgb_to_hsla(stops[3], alpha), 1.0),
            )),
        )
        .into_any_element()
}

fn render_orb(stops: FourStops, size_px: f32, alpha: f32) -> AnyElement {
    div()
        .size(px(size_px))
        .rounded_full()
        .flex_shrink_0()
        .bg(linear_gradient(
            135.0,
            linear_color_stop(rgb_to_hsla(stops[0], alpha), 0.0),
            linear_color_stop(rgb_to_hsla(stops[3], alpha), 1.0),
        ))
        .into_any_element()
}

fn pill_colors(is_light: bool, active: bool, accent: Hsla) -> (Hsla, Hsla, Hsla) {
    if is_light {
        if active {
            (
                rgba(0xffffffe8).into(),
                accent.opacity(0.85),
                rgba(0x2563eb1c).into(),
            )
        } else {
            (
                rgba(0xffffffb8).into(),
                rgba(0x0f172a1c).into(),
                rgba(0xffffffdc).into(),
            )
        }
    } else if active {
        (
            rgba(0x1e293be6).into(),
            accent.opacity(0.90),
            rgba(0x3b82f626).into(),
        )
    } else {
        (
            rgba(0x0f172aa6).into(),
            rgba(0xffffff1c).into(),
            rgba(0x1e293bd0).into(),
        )
    }
}

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
        let breathe = state.breathe;
        let palette_name = state.palette_name.clone();
        let stops = state.stops;
        let is_light = cx.theme().appearance() == Appearance::Light;
        let accent = cx.theme().colors().border_focused;

        let (breathe_bg, breathe_border, breathe_hover) = pill_colors(is_light, breathe, accent);
        let (studio_bg, studio_border, _) = pill_colors(is_light, enabled, accent);

        let roll_hover_frame = enabled
            .then(|| {
                ui::gradient_chrome::render_gradient_frame_canvas(
                    None,
                    10.0,
                    1.25,
                    0.0,
                    0.0,
                    false,
                    if is_light { 0.14 } else { 0.18 },
                    0.82,
                    cx,
                )
            })
            .flatten();

        // 1. Roll button + mini 4-stop rainbow pill bar (matches VS Code status bar!)
        let roll_btn = div()
            .id("gradient-roll-btn")
            .relative()
            .group("gradient-roll-btn")
            .cursor_pointer()
            .px_1p5()
            .py(px(2.))
            .rounded_full()
            .when(!enabled, |this| {
                this.hover(|s| s.bg(cx.theme().colors().element_hover))
            })
            .when_some(roll_hover_frame, |this, frame| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .visible_on_hover("gradient-roll-btn")
                        .child(frame),
                )
            })
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
                    .relative()
                    .gap_1p5()
                    .items_center()
                    .child(
                        Label::new("↻")
                            .size(LabelSize::Small)
                            .color(if enabled { Color::Default } else { Color::Muted }),
                    )
                    .child(
                        div()
                            .w(px(28.))
                            .h(px(7.))
                            .child(render_gradient_bar(
                                stops,
                                7.0,
                                if enabled { 0.92 } else { 0.30 },
                            )),
                    ),
            );

        let breathe_frame = enabled
            .then(|| {
                let wash = if breathe {
                    if is_light { 0.24 } else { 0.30 }
                } else if is_light {
                    0.10
                } else {
                    0.14
                };
                let b_alpha = if breathe { 0.94 } else { 0.68 };
                ui::gradient_chrome::render_gradient_frame_canvas(
                    None, 10.0, 1.25, 0.0, 0.0, false, wash, b_alpha, cx,
                )
            })
            .flatten();
        let has_breathe_frame = breathe_frame.is_some();

        // 2. Dedicated Breathe pill in status bar (matches VS Code status bar!)
        let breathe_btn = div()
            .id("gradient-status-breathe")
            .relative()
            .cursor_pointer()
            .px_2()
            .py(px(1.))
            .rounded_full()
            .children(breathe_frame)
            .when(!has_breathe_frame, |this| {
                this.border_1()
                    .border_color(breathe_border)
                    .bg(breathe_bg)
                    .hover(move |s| s.bg(breathe_hover))
            })
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                update_state(cx, |state| state.toggle_breathe());
            })
            .tooltip(|_window, cx| Tooltip::for_action("Toggle Breathing Aurora", &ToggleBreathe, cx))
            .child(
                div().relative().child(
                    Label::new("Breathe")
                        .size(LabelSize::XSmall)
                        .color(if breathe { Color::Accent } else { Color::Default }),
                ),
            );

        // 3. Studio ▴ pill trigger with glowing orb
        let studio_label: SharedString = if enabled {
            "Studio ▴".into()
        } else {
            "Studio (Off)".into()
        };

        let studio_frame = enabled
            .then(|| {
                ui::gradient_chrome::render_gradient_frame_canvas(
                    None,
                    10.0,
                    1.25,
                    0.0,
                    0.0,
                    false,
                    if is_light { 0.18 } else { 0.24 },
                    0.88,
                    cx,
                )
            })
            .flatten();
        let has_studio_frame = studio_frame.is_some();

        let studio_trigger = ButtonLike::new("gradient-studio-trigger")
            .style(ButtonStyle::Transparent)
            .child(
                h_flex()
                    .relative()
                    .gap_1()
                    .px_1p5()
                    .py(px(1.))
                    .rounded_full()
                    .children(studio_frame)
                    .when(!has_studio_frame, |this| {
                        this.border_1().border_color(studio_border).bg(studio_bg)
                    })
                    .items_center()
                    .child(div().relative().child(render_orb(
                        stops,
                        10.0,
                        if enabled { 0.95 } else { 0.35 },
                    )))
                    .child(div().relative().child(Label::new(studio_label).size(LabelSize::XSmall))),
            );

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
            .gap_1()
            .items_center()
            .child(roll_btn)
            .child(breathe_btn)
            .child(popover_menu)
            .into_any_element()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StudioCategoryTab {
    All,
    Favs,
    Prism,
    Pastel,
    Pure,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveSlider {
    Intensity,
    Speed,
    Hue,
    Sat,
    Light,
}

struct DisplayPreset {
    name: String,
    stops: FourStops,
    is_custom: bool,
}

pub struct GradientStudioPopover {
    focus_handle: FocusHandle,
    active_scope: Option<PanelRegion>,
    active_category: StudioCategoryTab,
    editing_stop: Option<usize>,
    custom_input: String,
    dragging_slider: Option<ActiveSlider>,
    intensity_bounds: Rc<Cell<Bounds<Pixels>>>,
    speed_bounds: Rc<Cell<Bounds<Pixels>>>,
    hue_bounds: Rc<Cell<Bounds<Pixels>>>,
    sat_bounds: Rc<Cell<Bounds<Pixels>>>,
    light_bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl GradientStudioPopover {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<GradientStudioState>(|_, cx| {
            cx.notify();
        })
        .detach();
        Self {
            focus_handle: cx.focus_handle(),
            active_scope: None,
            active_category: StudioCategoryTab::All,
            editing_stop: None,
            custom_input: String::new(),
            dragging_slider: None,
            intensity_bounds: Rc::new(Cell::new(Bounds::default())),
            speed_bounds: Rc::new(Cell::new(Bounds::default())),
            hue_bounds: Rc::new(Cell::new(Bounds::default())),
            sat_bounds: Rc::new(Cell::new(Bounds::default())),
            light_bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }

    fn ratio_from_x(bounds: Bounds<Pixels>, x: Pixels) -> f32 {
        let w = f32::from(bounds.size.width);
        if w <= 1.0 {
            return 0.5;
        }
        ((f32::from(x) - f32::from(bounds.origin.x)) / w).clamp(0.0, 1.0)
    }

    fn apply_slider_ratio(&mut self, slider: ActiveSlider, ratio: f32, cx: &mut Context<Self>) {
        let scope = self.active_scope;
        match slider {
            ActiveSlider::Intensity => {
                let val = 0.2 + ratio * (3.0 - 0.2);
                update_state(cx, |s| s.set_intensity(val));
            }
            ActiveSlider::Speed => {
                let pct = 25.0 + ratio * (250.0 - 25.0);
                update_state(cx, |s| s.set_breathe_speed(pct));
            }
            ActiveSlider::Hue | ActiveSlider::Sat | ActiveSlider::Light => {
                let Some(stop_idx) = self.editing_stop else {
                    return;
                };
                let current_rgb = cx
                    .try_global::<GradientStudioState>()
                    .map(|s| s.resolved_stops(scope)[stop_idx])
                    .unwrap_or([66, 133, 244]);
                let (mut h, mut sat, mut l) = rgb_to_hsl(current_rgb);
                match slider {
                    ActiveSlider::Hue => {
                        h = ratio * 360.0;
                        if sat < 12.0 {
                            sat = 75.0;
                        }
                    }
                    ActiveSlider::Sat => sat = ratio * 100.0,
                    ActiveSlider::Light => l = 12.0 + ratio * 76.0,
                    _ => {}
                }
                let next_rgb = hsl_to_rgb(h, sat, l);
                update_state(cx, |s| s.set_stop_color(scope, stop_idx, next_rgb));
            }
        }
    }

    fn handle_save_custom(&mut self, cx: &mut Context<Self>) {
        let scope = self.active_scope;
        let current_stops = cx
            .try_global::<GradientStudioState>()
            .map(|s| s.resolved_stops(scope))
            .unwrap_or(crate::DEFAULT_STOPS);

        let raw = self.custom_input.trim().to_string();
        if let Some(parsed_stops) = parse_four_hex_colors(&raw) {
            // Strip hex tokens to see if user also typed a custom name
            let cleaned_name: String = raw
                .split_whitespace()
                .filter(|tok| !tok.starts_with('#'))
                .collect::<Vec<_>>()
                .join(" ");
            update_state(cx, |s| s.save_custom_palette(scope, cleaned_name, parsed_stops));
        } else {
            update_state(cx, |s| s.save_custom_palette(scope, raw, current_stops));
        }
        self.custom_input.clear();
        self.active_category = StudioCategoryTab::Custom;
        cx.notify();
    }

    fn handle_paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Some(parsed) = parse_four_hex_colors(trimmed) {
            let scope = self.active_scope;
            update_state(cx, |s| s.select_palette_for(scope, "Pasted Hex", parsed));
            self.custom_input = trimmed.to_string();
        } else {
            self.custom_input.push_str(trimmed);
            if self.custom_input.len() > 64 {
                self.custom_input.truncate(64);
            }
        }
        cx.notify();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if key == "escape" {
            cx.emit(DismissEvent);
            cx.stop_propagation();
            return;
        }
        if key == "enter" {
            self.handle_save_custom(cx);
            cx.stop_propagation();
            return;
        }
        if key == "backspace" {
            self.custom_input.pop();
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if (event.keystroke.modifiers.platform || event.keystroke.modifiers.control) && key == "v" {
            self.handle_paste_clipboard(cx);
            cx.stop_propagation();
            return;
        }
        if !event.keystroke.modifiers.platform
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
        {
            if let Some(ch) = &event.keystroke.key_char {
                if ch.chars().all(|c| !c.is_control()) && self.custom_input.len() < 64 {
                    self.custom_input.push_str(ch);
                    if let Some(parsed) = parse_four_hex_colors(&self.custom_input) {
                        let scope = self.active_scope;
                        update_state(cx, |s| s.select_palette_for(scope, "Hex Mix", parsed));
                    }
                    cx.notify();
                    cx.stop_propagation();
                }
            }
        }
    }

    fn render_rainbow_slider(
        &self,
        id: &'static str,
        slider_kind: ActiveSlider,
        ratio: f32,
        track_stops: FourStops,
        bounds_cell: Rc<Cell<Bounds<Pixels>>>,
        accent: Hsla,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ratio = ratio.clamp(0.0, 1.0);
        let bounds_for_canvas = bounds_cell.clone();
        let bounds_for_down = bounds_cell.clone();
        let bounds_for_move = bounds_cell;

        div()
            .id(id)
            .relative()
            .flex_1()
            .h(px(20.))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    this.dragging_slider = Some(slider_kind);
                    let r = Self::ratio_from_x(bounds_for_down.get(), event.position.x);
                    this.apply_slider_ratio(slider_kind, r, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                if event.pressed_button == Some(MouseButton::Left)
                    && this.dragging_slider == Some(slider_kind)
                {
                    let r = Self::ratio_from_x(bounds_for_move.get(), event.position.x);
                    this.apply_slider_ratio(slider_kind, r, cx);
                } else if event.pressed_button.is_none() && this.dragging_slider == Some(slider_kind)
                {
                    this.dragging_slider = None;
                }
            }))
            .child(
                canvas(
                    move |bounds, _, _| {
                        bounds_for_canvas.set(bounds);
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            // Full-width smooth 4-stop rainbow track
            .child(
                div()
                    .w_full()
                    .h(px(7.))
                    .rounded_full()
                    .overflow_hidden()
                    .child(render_gradient_bar(track_stops, 7.0, 0.92)),
            )
            // Crisp white circular thumb with colored ring
            .child(
                div()
                    .absolute()
                    .left(relative(ratio))
                    .ml(px(-8.))
                    .size(px(16.))
                    .rounded_full()
                    .bg(rgb(0xffffff))
                    .border_2()
                    .border_color(accent)
                    .shadow_sm(),
            )
            .into_any_element()
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

        let is_light = cx.theme().appearance() == Appearance::Light;
        let colors = cx.theme().colors();
        let accent = colors.border_focused;

        let enabled = state.enabled;
        let breathe = state.breathe;
        let breathe_cfg = state.breathe_cfg;
        let seamless = state.seamless;
        let panels = state.panels;
        let intensity = state.intensity;
        let intensity_pct = (intensity * 100.0).round() as i32;
        let speed_pct = breathe_cfg.speed_pct.round() as i32;
        let can_undo = !state.history.is_empty();

        let active_scope = self.active_scope;
        let global_stops = state.stops;
        let active_stops = state.resolved_stops(active_scope);
        let active_name = state.resolved_name(active_scope).to_string();
        let favorites = state.favorites.clone();
        let component_palettes = state.component_palettes.clone();

        // Build filtered presets matching VS Code Studio tabs
        let visible_curated: Vec<_> = CURATED_PALETTES
            .iter()
            .filter(|p| !state.is_removed(p.name))
            .collect();
        let total_all_count = visible_curated.len() + state.custom_palettes.len();
        let favs_count = visible_curated
            .iter()
            .filter(|p| favorites.iter().any(|f| f == p.name))
            .count()
            + state
                .custom_palettes
                .iter()
                .filter(|p| favorites.iter().any(|f| f == &p.name))
                .count();

        let mut presets: Vec<DisplayPreset> = Vec::new();
        match self.active_category {
            StudioCategoryTab::All => {
                for c in &state.custom_palettes {
                    presets.push(DisplayPreset {
                        name: c.name.clone(),
                        stops: c.stops,
                        is_custom: true,
                    });
                }
                for p in &visible_curated {
                    presets.push(DisplayPreset {
                        name: p.name.to_string(),
                        stops: p.stops,
                        is_custom: false,
                    });
                }
            }
            StudioCategoryTab::Favs => {
                for c in &state.custom_palettes {
                    if favorites.iter().any(|f| f == &c.name) {
                        presets.push(DisplayPreset {
                            name: c.name.clone(),
                            stops: c.stops,
                            is_custom: true,
                        });
                    }
                }
                for p in &visible_curated {
                    if favorites.iter().any(|f| f == p.name) {
                        presets.push(DisplayPreset {
                            name: p.name.to_string(),
                            stops: p.stops,
                            is_custom: false,
                        });
                    }
                }
            }
            StudioCategoryTab::Prism => {
                for p in &visible_curated {
                    if p.category == PaletteCategory::Prism {
                        presets.push(DisplayPreset {
                            name: p.name.to_string(),
                            stops: p.stops,
                            is_custom: false,
                        });
                    }
                }
            }
            StudioCategoryTab::Pastel => {
                for p in &visible_curated {
                    if p.category == PaletteCategory::Pastel {
                        presets.push(DisplayPreset {
                            name: p.name.to_string(),
                            stops: p.stops,
                            is_custom: false,
                        });
                    }
                }
            }
            StudioCategoryTab::Pure => {
                for p in &visible_curated {
                    if p.category == PaletteCategory::Pure {
                        presets.push(DisplayPreset {
                            name: p.name.to_string(),
                            stops: p.stops,
                            is_custom: false,
                        });
                    }
                }
            }
            StudioCategoryTab::Custom => {
                for c in &state.custom_palettes {
                    presets.push(DisplayPreset {
                        name: c.name.clone(),
                        stops: c.stops,
                        is_custom: true,
                    });
                }
            }
        }

        // Glass surface palette tuned identically to gradient-studio-light.png & gradient-studio-dark.png
        let card_base_bg: Hsla = if is_light {
            rgba(0xf9fafffa).into()
        } else {
            rgba(0x111722fa).into()
        };
        let glass_surface: Hsla = if is_light {
            rgba(0xffffffa8).into()
        } else {
            rgba(0x0b12208c).into()
        };
        let glass_surface_hover: Hsla = if is_light {
            rgba(0xffffffdc).into()
        } else {
            rgba(0x1b2638b8).into()
        };
        let glass_border: Hsla = if is_light {
            rgba(0x0f172a1a).into()
        } else {
            rgba(0xffffff1c).into()
        };
        let divider_color: Hsla = if is_light {
            rgba(0x0f172a14).into()
        } else {
            rgba(0xffffff14).into()
        };
        let footer_bg: Hsla = if is_light {
            rgba(0xffffff80).into()
        } else {
            rgba(0x090e1899).into()
        };

        let (breathe_pill_bg, breathe_pill_border, breathe_pill_hover) =
            pill_colors(is_light, breathe, accent);
        let (roll_bg, roll_border, roll_hover) = pill_colors(is_light, false, accent);
        let (seamless_bg, seamless_border, seamless_hover) =
            pill_colors(is_light, seamless, accent);

        let intensity_ratio = ((intensity - 0.2) / (3.0 - 0.2)).clamp(0.0, 1.0);
        let speed_ratio = ((breathe_cfg.speed_pct - 25.0) / (250.0 - 25.0)).clamp(0.0, 1.0);

        let intensity_slider = self.render_rainbow_slider(
            "studio-intensity-slider",
            ActiveSlider::Intensity,
            intensity_ratio,
            active_stops,
            self.intensity_bounds.clone(),
            accent,
            cx,
        );
        let speed_slider = self.render_rainbow_slider(
            "studio-speed-slider",
            ActiveSlider::Speed,
            speed_ratio,
            active_stops,
            self.speed_bounds.clone(),
            accent,
            cx,
        );

        let breathe_modes: [(&'static str, &'static str, bool, fn(&mut GradientStudioState)); 4] = [
            (
                "breathe-wave",
                "Full Wave",
                breathe_cfg.full_wave,
                GradientStudioState::toggle_breathe_wave,
            ),
            (
                "breathe-pulse",
                "Pulse",
                breathe_cfg.pulse,
                GradientStudioState::toggle_breathe_pulse,
            ),
            (
                "breathe-shimmer",
                "Shimmer",
                breathe_cfg.shimmer,
                GradientStudioState::toggle_breathe_shimmer,
            ),
            (
                "breathe-morph",
                "Auto-Morph",
                breathe_cfg.auto_morph,
                GradientStudioState::toggle_breathe_auto_morph,
            ),
        ];

        let category_tabs: [(StudioCategoryTab, String); 6] = [
            (StudioCategoryTab::All, format!("All ({total_all_count})")),
            (StudioCategoryTab::Favs, format!("Favs ({favs_count})")),
            (StudioCategoryTab::Prism, "Prism".to_string()),
            (StudioCategoryTab::Pastel, "Pastel".to_string()),
            (StudioCategoryTab::Pure, "Pure".to_string()),
            (StudioCategoryTab::Custom, "Custom".to_string()),
        ];

        // Optional inline HSL Color Tuner when user clicks one of the 4 big swatch pills
        let hsl_tuner: Option<AnyElement> = self.editing_stop.map(|stop_idx| {
            let stop_rgb = active_stops[stop_idx];
            let (h, s, l) = rgb_to_hsl(stop_rgb);
            let hex_label = rgb_to_hex(stop_rgb);

            let hue_track: FourStops = [
                [239, 68, 68],
                [234, 179, 8],
                [34, 197, 94],
                [139, 92, 246],
            ];
            let sat_track: FourStops = [
                hsl_to_rgb(h, 5.0, l),
                hsl_to_rgb(h, 35.0, l),
                hsl_to_rgb(h, 70.0, l),
                hsl_to_rgb(h, 100.0, l),
            ];
            let light_track: FourStops = [
                hsl_to_rgb(h, s, 15.0),
                hsl_to_rgb(h, s, 40.0),
                hsl_to_rgb(h, s, 65.0),
                hsl_to_rgb(h, s, 88.0),
            ];

            let hue_slider = self.render_rainbow_slider(
                "studio-hue-slider",
                ActiveSlider::Hue,
                h / 360.0,
                hue_track,
                self.hue_bounds.clone(),
                accent,
                cx,
            );
            let sat_slider = self.render_rainbow_slider(
                "studio-sat-slider",
                ActiveSlider::Sat,
                s / 100.0,
                sat_track,
                self.sat_bounds.clone(),
                accent,
                cx,
            );
            let light_slider = self.render_rainbow_slider(
                "studio-light-slider",
                ActiveSlider::Light,
                ((l - 12.0) / 76.0).clamp(0.0, 1.0),
                light_track,
                self.light_bounds.clone(),
                accent,
                cx,
            );

            v_flex()
                .p_2()
                .gap_1()
                .rounded_lg()
                .border_1()
                .border_color(glass_border)
                .bg(glass_surface)
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            Label::new(format!("Stop {} • {}", stop_idx + 1, hex_label))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                        .child(
                            div()
                                .id("close-hsl-tuner")
                                .cursor_pointer()
                                .px_1p5()
                                .rounded_full()
                                .hover(move |s| s.bg(glass_surface_hover))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.editing_stop = None;
                                    cx.notify();
                                }))
                                .child(Label::new("Done").size(LabelSize::XSmall).color(Color::Accent)),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(34.)).child(Label::new("Hue").size(LabelSize::XSmall).color(Color::Muted)))
                        .child(hue_slider),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(34.)).child(Label::new("Sat").size(LabelSize::XSmall).color(Color::Muted)))
                        .child(sat_slider),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(34.)).child(Label::new("Light").size(LabelSize::XSmall).color(Color::Muted)))
                        .child(light_slider),
                )
                .into_any_element()
        });

        let scope_footer_title: SharedString = match active_scope {
            None => "Pick your own color".into(),
            Some(panel) => format!("Pick your own color • {}", panel.label()).into(),
        };

        let input_display: SharedString = if self.custom_input.is_empty() {
            "Name or paste 4 #hex colors...".into()
        } else {
            self.custom_input.clone().into()
        };
        let input_is_placeholder = self.custom_input.is_empty();

        // Root floating glass card matching VS Code Gradient Studio 1:1
        div()
            .id("gradient-studio-popover")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .occlude()
            .relative()
            .w(px(444.))
            .rounded(px(14.))
            .overflow_hidden()
            .border_1()
            .border_color(glass_border)
            .bg(card_base_bg)
            .shadow_lg()
            // Live 4-corner aurora glass wash inside the Studio card itself!
            .child(render_studio_card_backdrop(active_stops, is_light))
            // Foreground interactive content
            .child(
                v_flex()
                    .relative()
                    .w_full()
                    // 3px top rainbow accent bar
                    .child(render_gradient_bar(active_stops, 3.0, 0.95))
                    // Main body container
                    .child(
                        v_flex()
                            .px_3p5()
                            .pt_3()
                            .pb_2p5()
                            .gap_2p5()
                            // 1. HEADER ROW: Orb + "Gradient Studio" | Breathe | Roll | Mode | X
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(render_orb(active_stops, 20.0, 0.96))
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(Label::new("Gradient Studio").size(LabelSize::Default)),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child({
                                                let hdr_breathe_frame = enabled
                                                    .then(|| {
                                                        let wash = if breathe {
                                                            if is_light { 0.24 } else { 0.30 }
                                                        } else if is_light {
                                                            0.10
                                                        } else {
                                                            0.14
                                                        };
                                                        let b_alpha =
                                                            if breathe { 0.94 } else { 0.68 };
                                                        ui::gradient_chrome::render_gradient_frame_canvas(
                                                            None, 12.0, 1.25, 0.0, 0.0, false,
                                                            wash, b_alpha, cx,
                                                        )
                                                    })
                                                    .flatten();
                                                let has_hdr_breathe_frame =
                                                    hdr_breathe_frame.is_some();
                                                div()
                                                    .id("studio-hdr-breathe")
                                                    .relative()
                                                    .cursor_pointer()
                                                    .px_2p5()
                                                    .py(px(4.))
                                                    .rounded_full()
                                                    .children(hdr_breathe_frame)
                                                    .when(!has_hdr_breathe_frame, |this| {
                                                        this.border_1()
                                                            .border_color(breathe_pill_border)
                                                            .bg(breathe_pill_bg)
                                                            .hover(move |s| {
                                                                s.bg(breathe_pill_hover)
                                                            })
                                                    })
                                                    .on_click(|_, _, cx| {
                                                        update_state(cx, |s| s.toggle_breathe())
                                                    })
                                                    .child(
                                                        div().relative().child(
                                                            Label::new("Breathe")
                                                                .size(LabelSize::Small)
                                                                .color(if breathe {
                                                                    Color::Accent
                                                                } else {
                                                                    Color::Default
                                                                }),
                                                        ),
                                                    )
                                            })
                                            .child(
                                                div()
                                                    .id("studio-hdr-roll")
                                                    .cursor_pointer()
                                                    .px_2p5()
                                                    .py(px(4.))
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(roll_border)
                                                    .bg(roll_bg)
                                                    .hover(move |s| s.bg(roll_hover))
                                                    .on_click(move |_, _, cx| {
                                                        update_state(cx, |s| {
                                                            s.roll_random_for(active_scope)
                                                        })
                                                    })
                                                    .child(Label::new("↻ Roll").size(LabelSize::Small)),
                                            )
                                            .child({
                                                let off_track_bg: Hsla = if is_light {
                                                    rgba(0x0f172a28).into()
                                                } else {
                                                    rgba(0xffffff26).into()
                                                };
                                                let track_border: Hsla = if enabled {
                                                    rgba(0xffffff55).into()
                                                } else {
                                                    glass_border
                                                };
                                                div()
                                                    .id("studio-hdr-mode")
                                                    .relative()
                                                    .cursor_pointer()
                                                    .w(px(36.))
                                                    .h(px(20.))
                                                    .px(px(2.5))
                                                    .flex()
                                                    .items_center()
                                                    .when(enabled, |this| this.justify_end())
                                                    .when(!enabled, |this| this.justify_start())
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(track_border)
                                                    .when(!enabled, |this| this.bg(off_track_bg))
                                                    .overflow_hidden()
                                                    .tooltip(Tooltip::text(if enabled {
                                                        "Gradient Overlay: On (click to turn off)"
                                                    } else {
                                                        "Gradient Overlay: Off (click to turn on)"
                                                    }))
                                                    .on_click(|_, _, cx| {
                                                        update_state(cx, |s| s.toggle_enabled())
                                                    })
                                                    .when(enabled, |this| {
                                                        this.child(
                                                            div()
                                                                .absolute()
                                                                .inset_0()
                                                                .child(render_gradient_bar(
                                                                    active_stops,
                                                                    20.0,
                                                                    0.92,
                                                                )),
                                                        )
                                                    })
                                                    .child(
                                                        div()
                                                            .relative()
                                                            .size(px(14.))
                                                            .rounded_full()
                                                            .bg(rgb(0xffffff))
                                                            .shadow_sm(),
                                                    )
                                            })
                                            .child(
                                                div()
                                                    .id("studio-hdr-close")
                                                    .cursor_pointer()
                                                    .size(px(26.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(roll_border)
                                                    .bg(roll_bg)
                                                    .hover(move |s| s.bg(roll_hover))
                                                    .on_click(cx.listener(|_, _, _, cx| {
                                                        cx.emit(DismissEvent);
                                                    }))
                                                    .child(Label::new("✕").size(LabelSize::XSmall)),
                                            ),
                                    ),
                            )
                            // Thin separator line
                            .child(div().w_full().h(px(1.)).bg(divider_color))
                            // 2. INTENSITY ROW: Label | Rainbow Slider | 100% | Seamless: ON
                            .child(
                                h_flex()
                                    .gap_2p5()
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(58.))
                                            .child(Label::new("Intensity").size(LabelSize::Small)),
                                    )
                                    .child(intensity_slider)
                                    .child(
                                        div()
                                            .id("studio-intensity-readout")
                                            .cursor_pointer()
                                            .w(px(38.))
                                            .flex()
                                            .justify_end()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .on_click(|_, _, cx| {
                                                update_state(cx, |s| s.reset_intensity())
                                            })
                                            .child(
                                                Label::new(format!("{intensity_pct}%"))
                                                    .size(LabelSize::Small),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("studio-seamless-pill")
                                            .cursor_pointer()
                                            .px_2p5()
                                            .py(px(4.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(seamless_border)
                                            .bg(seamless_bg)
                                            .hover(move |s| s.bg(seamless_hover))
                                            .on_click(|_, _, cx| {
                                                update_state(cx, |s| s.toggle_seamless())
                                            })
                                            .child(
                                                Label::new(if seamless {
                                                    "Seamless: ON"
                                                } else {
                                                    "Seamless: OFF"
                                                })
                                                .size(LabelSize::XSmall)
                                                .color(if seamless {
                                                    Color::Accent
                                                } else {
                                                    Color::Default
                                                }),
                                            ),
                                    ),
                            )
                            // 3. BREATHING AURORA SUB-CARD (4 modes + Speed rainbow slider)
                            .child(
                                v_flex()
                                    .p_2p5()
                                    .gap_2()
                                    .rounded(px(12.))
                                    .border_1()
                                    .border_color(if breathe {
                                        accent.opacity(0.45)
                                    } else {
                                        glass_border
                                    })
                                    .bg(glass_surface)
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(Label::new("Breathing Aurora").size(LabelSize::Small)),
                                            )
                                            .child(
                                                Label::new(if breathe { "ACTIVE" } else { "PAUSED" })
                                                    .size(LabelSize::XSmall)
                                                    .color(if breathe {
                                                        Color::Accent
                                                    } else {
                                                        Color::Muted
                                                    }),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .w_full()
                                            .children(breathe_modes.into_iter().map(
                                                |(id, label, is_on, toggle_fn)| {
                                                    let active = breathe && is_on;
                                                    let (bg, border, hover) =
                                                        pill_colors(is_light, active, accent);
                                                    div()
                                                        .id(id)
                                                        .flex_1()
                                                        .cursor_pointer()
                                                        .py(px(5.))
                                                        .flex()
                                                        .justify_center()
                                                        .items_center()
                                                        .rounded_full()
                                                        .border_1()
                                                        .border_color(border)
                                                        .bg(bg)
                                                        .hover(move |s| s.bg(hover))
                                                        .on_click(move |_, _, cx| {
                                                            update_state(cx, toggle_fn)
                                                        })
                                                        .child(
                                                            Label::new(label)
                                                                .size(LabelSize::XSmall)
                                                                .color(if active {
                                                                    Color::Accent
                                                                } else {
                                                                    Color::Default
                                                                }),
                                                        )
                                                },
                                            )),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2p5()
                                            .items_center()
                                            .child(
                                                div()
                                                    .w(px(42.))
                                                    .child(Label::new("Speed").size(LabelSize::Small)),
                                            )
                                            .child(speed_slider)
                                            .child(
                                                div()
                                                    .id("studio-speed-readout")
                                                    .cursor_pointer()
                                                    .w(px(38.))
                                                    .flex()
                                                    .justify_end()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .on_click(|_, _, cx| {
                                                        update_state(cx, |s| {
                                                            s.set_breathe_speed(100.0)
                                                        })
                                                    })
                                                    .child(
                                                        Label::new(format!("{speed_pct}%"))
                                                            .size(LabelSize::Small),
                                                    ),
                                            ),
                                    ),
                            )
                            // 4. COMPONENT SCOPE PILLS: All | Editor | Explorer | Chat | Terminal | Bars
                            .child(
                                h_flex()
                                    .gap_1()
                                    .flex_wrap()
                                    .items_center()
                                    .child({
                                        let is_active = active_scope.is_none();
                                        let (bg, border, hover) =
                                            pill_colors(is_light, is_active, accent);
                                        let stops = global_stops;
                                        div()
                                            .id("scope-all")
                                            .cursor_pointer()
                                            .px_2()
                                            .py(px(3.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(border)
                                            .bg(bg)
                                            .hover(move |s| s.bg(hover))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_scope = None;
                                                cx.notify();
                                            }))
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .items_center()
                                                    .child(render_orb(stops, 8.0, 0.95))
                                                    .child(
                                                        Label::new("All")
                                                            .size(LabelSize::XSmall)
                                                            .color(if is_active {
                                                                Color::Accent
                                                            } else {
                                                                Color::Default
                                                            }),
                                                    ),
                                            )
                                    })
                                    .children(PanelRegion::ALL.into_iter().map(|region| {
                                        let is_active = active_scope == Some(region);
                                        let (bg, border, hover) =
                                            pill_colors(is_light, is_active, accent);
                                        let region_stops = component_palettes
                                            .get(region)
                                            .map(|c| c.stops)
                                            .unwrap_or(global_stops);
                                        let is_on = panels.is_enabled(region);
                                        div()
                                            .id(SharedString::from(format!(
                                                "scope-{}",
                                                region.label()
                                            )))
                                            .cursor_pointer()
                                            .px_2()
                                            .py(px(3.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(border)
                                            .bg(bg)
                                            .hover(move |s| s.bg(hover))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.active_scope = Some(region);
                                                cx.notify();
                                            }))
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .items_center()
                                                    .child(render_orb(
                                                        region_stops,
                                                        8.0,
                                                        if is_on { 0.95 } else { 0.25 },
                                                    ))
                                                    .child(
                                                        Label::new(region.label())
                                                            .size(LabelSize::XSmall)
                                                            .color(if is_active {
                                                                Color::Accent
                                                            } else if is_on {
                                                                Color::Default
                                                            } else {
                                                                Color::Muted
                                                            }),
                                                    ),
                                            )
                                    })),
                            )
                            .child(div().w_full().h(px(1.)).bg(divider_color))
                            // 5. CATEGORY FILTER PILLS: All (30) | Favs (N) | Prism | Pastel | Pure | Custom
                            .child(
                                h_flex()
                                    .gap_1()
                                    .w_full()
                                    .children(category_tabs.into_iter().map(|(tab, label)| {
                                        let is_active = self.active_category == tab;
                                        let (bg, border, hover) =
                                            pill_colors(is_light, is_active, accent);
                                        div()
                                            .id(SharedString::from(format!("cat-{label}")))
                                            .flex_1()
                                            .cursor_pointer()
                                            .py(px(4.))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .rounded_full()
                                            .border_1()
                                            .border_color(border)
                                            .bg(bg)
                                            .hover(move |s| s.bg(hover))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.active_category = tab;
                                                cx.notify();
                                            }))
                                            .child(
                                                Label::new(label)
                                                    .size(LabelSize::XSmall)
                                                    .color(if is_active {
                                                        Color::Accent
                                                    } else {
                                                        Color::Default
                                                    }),
                                            )
                                    })),
                            )
                            // 6. CONTEXT MODE BANNER CARD (Seamless banner on All, or ON/OFF + Unified cards on Component)
                            .child(match active_scope {
                                None => v_flex()
                                    .id("studio-seamless-card")
                                    .cursor_pointer()
                                    .p_2()
                                    .gap_1p5()
                                    .rounded(px(10.))
                                    .border_1()
                                    .border_color(if seamless {
                                        accent.opacity(0.45)
                                    } else {
                                        glass_border
                                    })
                                    .bg(glass_surface)
                                    .hover(move |s| s.bg(glass_surface_hover))
                                    .on_click(|_, _, cx| update_state(cx, |s| s.toggle_seamless()))
                                    .child(render_gradient_bar(active_stops, 7.0, 0.90))
                                    .child(
                                        Label::new(if seamless {
                                            "● Seamless Mode: ON (Click for Per-Component Split Gradients)"
                                        } else {
                                            "○ Split Panels Mode: ON (Click for Unified Seamless Wash)"
                                        })
                                        .size(LabelSize::XSmall),
                                    )
                                    .into_any_element(),
                                Some(panel) => {
                                    let is_on = panels.is_enabled(panel);
                                    let is_unified = component_palettes.get(panel).is_none();
                                    let green_bar: FourStops = [
                                        [16, 185, 129],
                                        [34, 197, 94],
                                        [52, 211, 153],
                                        [16, 185, 129],
                                    ];
                                    h_flex()
                                        .gap_2()
                                        .w_full()
                                        .child(
                                            v_flex()
                                                .id("studio-comp-toggle")
                                                .flex_1()
                                                .cursor_pointer()
                                                .p_2()
                                                .gap_1p5()
                                                .rounded(px(10.))
                                                .border_1()
                                                .border_color(glass_border)
                                                .bg(glass_surface)
                                                .hover(move |s| s.bg(glass_surface_hover))
                                                .on_click(move |_, _, cx| {
                                                    update_state(cx, |s| {
                                                        s.toggle_panel(panel);
                                                    })
                                                })
                                                .child(render_gradient_bar(
                                                    green_bar,
                                                    6.0,
                                                    if is_on { 0.90 } else { 0.25 },
                                                ))
                                                .child(
                                                    Label::new(if is_on {
                                                        format!("● {}: ON (Turn Off)", panel.label())
                                                    } else {
                                                        format!("○ {}: OFF (Turn On)", panel.label())
                                                    })
                                                    .size(LabelSize::XSmall),
                                                ),
                                        )
                                        .child(
                                            v_flex()
                                                .id("studio-comp-unified")
                                                .flex_1()
                                                .cursor_pointer()
                                                .p_2()
                                                .gap_1p5()
                                                .rounded(px(10.))
                                                .border_1()
                                                .border_color(if is_unified {
                                                    accent.opacity(0.85)
                                                } else {
                                                    glass_border
                                                })
                                                .bg(glass_surface)
                                                .hover(move |s| s.bg(glass_surface_hover))
                                                .on_click(move |_, _, cx| {
                                                    update_state(cx, |s| {
                                                        s.clear_component_palette(panel)
                                                    })
                                                })
                                                .child(render_gradient_bar(global_stops, 6.0, 0.90))
                                                .child(
                                                    Label::new(if is_unified {
                                                        "⇄ Same as All (Unified)"
                                                    } else {
                                                        "⇄ Reset to Same as All"
                                                    })
                                                    .size(LabelSize::XSmall)
                                                    .color(if is_unified {
                                                        Color::Accent
                                                    } else {
                                                        Color::Default
                                                    }),
                                                ),
                                        )
                                        .into_any_element()
                                }
                            })
                            // 7. SCROLLABLE 2-COLUMN PRESET CARDS GRID
                            .child(
                                v_flex()
                                    .id("gradient-presets-scroll")
                                    .max_h(px(236.))
                                    .overflow_y_scroll()
                                    .gap_2()
                                    .when(presets.is_empty(), |el| {
                                        let empty_msg = match self.active_category {
                                            StudioCategoryTab::Favs => {
                                                "No favorites yet — click ☆ on any preset!"
                                            }
                                            StudioCategoryTab::Custom => {
                                                "No custom palettes yet — pick 4 colors below & click + Save!"
                                            }
                                            _ => "No palettes in this category.",
                                        };
                                        el.child(
                                            div().py_5().flex().justify_center().child(
                                                Label::new(empty_msg)
                                                    .size(LabelSize::Small)
                                                    .color(Color::Muted),
                                            ),
                                        )
                                    })
                                    .children(presets.chunks(2).enumerate().map(|(row_idx, pair)| {
                                        h_flex()
                                            .id(SharedString::from(format!("preset-row-{row_idx}")))
                                            .gap_2()
                                            .w_full()
                                            .children(pair.iter().map(|preset| {
                                                let name = preset.name.clone();
                                                let preset_stops = preset.stops;
                                                let is_selected = enabled && active_name == name;
                                                let is_fav = favorites.iter().any(|f| f == &name);
                                                let click_name = name.clone();
                                                let fav_name = name.clone();
                                                let remove_name = name.clone();
                                                let is_default_card =
                                                    !preset.is_custom && name == crate::DEFAULT_PALETTE_NAME;

                                                v_flex()
                                                    .id(SharedString::from(format!("preset-{name}")))
                                                    .flex_1()
                                                    .p_2()
                                                    .gap_1p5()
                                                    .rounded(px(11.))
                                                    .cursor_pointer()
                                                    .border_1()
                                                    .border_color(if is_selected {
                                                        accent
                                                    } else {
                                                        glass_border
                                                    })
                                                    .bg(if is_selected {
                                                        if is_light {
                                                            rgba(0xffffffe8).into()
                                                        } else {
                                                            rgba(0x1e293bd8).into()
                                                        }
                                                    } else {
                                                        glass_surface
                                                    })
                                                    .hover(move |s| s.bg(glass_surface_hover))
                                                    .on_click(move |_, _, cx| {
                                                        let n = click_name.clone();
                                                        update_state(cx, |s| {
                                                            s.select_palette_for(
                                                                active_scope,
                                                                n,
                                                                preset_stops,
                                                            )
                                                        });
                                                    })
                                                    // Top full-width rounded 4-stop gradient ribbon
                                                    .child(render_gradient_bar(
                                                        preset_stops,
                                                        10.0,
                                                        0.94,
                                                    ))
                                                    // Bottom row: Name | 4 dots + star + remove
                                                    .child(
                                                        h_flex()
                                                            .justify_between()
                                                            .items_center()
                                                            .gap_1()
                                                            .child(
                                                                div().overflow_hidden().child(
                                                                    Label::new(name.clone())
                                                                        .size(LabelSize::XSmall)
                                                                        .color(if is_selected {
                                                                            Color::Accent
                                                                        } else {
                                                                            Color::Default
                                                                        }),
                                                                ),
                                                            )
                                                            .child(
                                                                h_flex()
                                                                    .gap_1()
                                                                    .items_center()
                                                                    .flex_shrink_0()
                                                                    // 4 crisp circular color dots
                                                                    .child(
                                                                        h_flex()
                                                                            .gap(px(2.5))
                                                                            .items_center()
                                                                            .children(
                                                                                preset_stops
                                                                                    .into_iter()
                                                                                    .map(|c| {
                                                                                        div()
                                                                                            .size(px(6.5))
                                                                                            .rounded_full()
                                                                                            .bg(rgb_to_hsla(
                                                                                                c,
                                                                                                0.98,
                                                                                            ))
                                                                                    }),
                                                                            ),
                                                                    )
                                                                    // Star favorite button
                                                                    .child(
                                                                        div()
                                                                            .id(SharedString::from(
                                                                                format!("fav-{name}"),
                                                                            ))
                                                                            .cursor_pointer()
                                                                            .px(px(2.))
                                                                            .rounded_sm()
                                                                            .on_mouse_down(
                                                                                MouseButton::Left,
                                                                                move |_, _, cx| {
                                                                                    cx.stop_propagation();
                                                                                    update_state(cx, |s| {
                                                                                        s.toggle_favorite(&fav_name)
                                                                                    });
                                                                                },
                                                                            )
                                                                            .child(
                                                                                Label::new(if is_fav {
                                                                                    "★"
                                                                                } else {
                                                                                    "☆"
                                                                                })
                                                                                .size(LabelSize::XSmall)
                                                                                .color(if is_fav {
                                                                                    Color::Warning
                                                                                } else {
                                                                                    Color::Muted
                                                                                }),
                                                                            ),
                                                                    )
                                                                    // Hide / delete button
                                                                    .when(!is_default_card, |el| {
                                                                        el.child(
                                                                            div()
                                                                                .id(SharedString::from(
                                                                                    format!("rm-{name}"),
                                                                                ))
                                                                                .cursor_pointer()
                                                                                .px(px(2.))
                                                                                .rounded_sm()
                                                                                .on_mouse_down(
                                                                                    MouseButton::Left,
                                                                                    move |_, _, cx| {
                                                                                        cx.stop_propagation();
                                                                                        update_state(cx, |s| {
                                                                                            s.remove_palette(&remove_name)
                                                                                        });
                                                                                    },
                                                                                )
                                                                                .child(
                                                                                    Label::new("×")
                                                                                        .size(LabelSize::XSmall)
                                                                                        .color(Color::Muted),
                                                                                ),
                                                                        )
                                                                    }),
                                                            ),
                                                    )
                                            }))
                                    })),
                            ),
                    )
                    // 8. FOOTER: "Pick your own color" + 4 large rounded color pills + HSL tuner + hex/name input + Save
                    .child(
                        v_flex()
                            .px_3p5()
                            .py_2p5()
                            .gap_2()
                            .border_t_1()
                            .border_color(divider_color)
                            .bg(footer_bg)
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(Label::new(scope_footer_title).size(LabelSize::Small)),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(
                                                div()
                                                    .id("studio-swap-corners")
                                                    .cursor_pointer()
                                                    .px_1p5()
                                                    .py(px(2.))
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(glass_border)
                                                    .bg(glass_surface)
                                                    .hover(move |s| s.bg(glass_surface_hover))
                                                    .on_click(|_, _, cx| {
                                                        update_state(cx, |s| s.swap_corners())
                                                    })
                                                    .child(
                                                        Label::new("⇄ Swap")
                                                            .size(LabelSize::XSmall)
                                                            .color(Color::Muted),
                                                    ),
                                            )
                                            .when(can_undo, |el| {
                                                el.child(
                                                    div()
                                                        .id("studio-undo-palette")
                                                        .cursor_pointer()
                                                        .px_1p5()
                                                        .py(px(2.))
                                                        .rounded_full()
                                                        .border_1()
                                                        .border_color(glass_border)
                                                        .bg(glass_surface)
                                                        .hover(move |s| s.bg(glass_surface_hover))
                                                        .on_click(|_, _, cx| {
                                                            update_state(cx, |s| {
                                                                s.previous_palette();
                                                            })
                                                        })
                                                        .child(
                                                            Label::new("↩ Undo")
                                                                .size(LabelSize::XSmall)
                                                                .color(Color::Muted),
                                                        ),
                                                )
                                            }),
                                    ),
                            )
                            // Thin live preview bar
                            .child(render_gradient_bar(active_stops, 5.0, 0.94))
                            // 4 large rounded color swatch pills (click to edit via HSL sliders!)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .w_full()
                                    .children(active_stops.into_iter().enumerate().map(
                                        |(idx, stop_rgb)| {
                                            let is_editing = self.editing_stop == Some(idx);
                                            div()
                                                .id(SharedString::from(format!("swatch-pill-{idx}")))
                                                .flex_1()
                                                .h(px(26.))
                                                .rounded_full()
                                                .cursor_pointer()
                                                .bg(rgb_to_hsla(stop_rgb, 0.96))
                                                .border_2()
                                                .border_color(if is_editing {
                                                    rgb(0xffffff).into()
                                                } else {
                                                    glass_border
                                                })
                                                .shadow_sm()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.editing_stop =
                                                        if this.editing_stop == Some(idx) {
                                                            None
                                                        } else {
                                                            Some(idx)
                                                        };
                                                    cx.notify();
                                                }))
                                        },
                                    )),
                            )
                            // Inline HSL tuner (shown when a swatch pill is selected)
                            .children(hsl_tuner)
                            // Bottom Name / Hex Paste input + Paste + "+ Save" pill
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .w_full()
                                    .items_center()
                                    .child(
                                        div()
                                            .id("studio-custom-input")
                                            .flex_1()
                                            .px_3()
                                            .py(px(6.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(glass_border)
                                            .bg(glass_surface)
                                            .cursor_text()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.focus_handle.focus(window, cx);
                                            }))
                                            .child(
                                                Label::new(input_display)
                                                    .size(LabelSize::XSmall)
                                                    .color(if input_is_placeholder {
                                                        Color::Muted
                                                    } else {
                                                        Color::Default
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("studio-paste-hex")
                                            .cursor_pointer()
                                            .px_2()
                                            .py(px(6.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(glass_border)
                                            .bg(glass_surface)
                                            .hover(move |s| s.bg(glass_surface_hover))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.handle_paste_clipboard(cx);
                                            }))
                                            .child(
                                                Label::new("Paste")
                                                    .size(LabelSize::XSmall)
                                                    .color(Color::Muted),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("studio-save-custom")
                                            .cursor_pointer()
                                            .px_3()
                                            .py(px(6.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(accent.opacity(0.65))
                                            .bg(glass_surface)
                                            .hover(move |s| s.bg(glass_surface_hover))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.handle_save_custom(cx);
                                            }))
                                            .child(
                                                Label::new("+ Save")
                                                    .size(LabelSize::XSmall)
                                                    .color(Color::Accent),
                                            ),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
