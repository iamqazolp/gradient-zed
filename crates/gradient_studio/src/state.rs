use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{App, BorrowAppContext as _, Global, Task};
use rand::Rng as _;
use serde::{Deserialize, Serialize};

use crate::palettes::{
    CURATED_PALETTES, CustomPalette, DEFAULT_PALETTE_NAME, DEFAULT_STOPS, FourStops, Rgb,
    generate_procedural_palette,
};

const SAVE_DEBOUNCE: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelRegion {
    Editor,
    LeftDock,
    RightDock,
    BottomDock,
    Bars,
}

impl PanelRegion {
    pub const ALL: [PanelRegion; 5] = [
        PanelRegion::Editor,
        PanelRegion::LeftDock,
        PanelRegion::RightDock,
        PanelRegion::BottomDock,
        PanelRegion::Bars,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PanelRegion::Editor => "Editor",
            PanelRegion::LeftDock => "Explorer",
            PanelRegion::RightDock => "Chat",
            PanelRegion::BottomDock => "Terminal",
            PanelRegion::Bars => "Bars",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelMask {
    pub editor: bool,
    pub left_dock: bool,
    pub right_dock: bool,
    pub bottom_dock: bool,
    pub bars: bool,
}

impl Default for PanelMask {
    fn default() -> Self {
        Self {
            editor: true,
            left_dock: true,
            right_dock: true,
            bottom_dock: true,
            bars: true,
        }
    }
}

impl PanelMask {
    pub fn is_enabled(&self, region: PanelRegion) -> bool {
        match region {
            PanelRegion::Editor => self.editor,
            PanelRegion::LeftDock => self.left_dock,
            PanelRegion::RightDock => self.right_dock,
            PanelRegion::BottomDock => self.bottom_dock,
            PanelRegion::Bars => self.bars,
        }
    }

    pub fn toggle(&mut self, region: PanelRegion) {
        match region {
            PanelRegion::Editor => self.editor = !self.editor,
            PanelRegion::LeftDock => self.left_dock = !self.left_dock,
            PanelRegion::RightDock => self.right_dock = !self.right_dock,
            PanelRegion::BottomDock => self.bottom_dock = !self.bottom_dock,
            PanelRegion::Bars => self.bars = !self.bars,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ComponentPalettes {
    pub editor: Option<CustomPalette>,
    pub left_dock: Option<CustomPalette>,
    pub right_dock: Option<CustomPalette>,
    pub bottom_dock: Option<CustomPalette>,
    pub bars: Option<CustomPalette>,
}

impl ComponentPalettes {
    pub fn get(&self, region: PanelRegion) -> Option<&CustomPalette> {
        match region {
            PanelRegion::Editor => self.editor.as_ref(),
            PanelRegion::LeftDock => self.left_dock.as_ref(),
            PanelRegion::RightDock => self.right_dock.as_ref(),
            PanelRegion::BottomDock => self.bottom_dock.as_ref(),
            PanelRegion::Bars => self.bars.as_ref(),
        }
    }

    pub fn set(&mut self, region: PanelRegion, value: Option<CustomPalette>) {
        match region {
            PanelRegion::Editor => self.editor = value,
            PanelRegion::LeftDock => self.left_dock = value,
            PanelRegion::RightDock => self.right_dock = value,
            PanelRegion::BottomDock => self.bottom_dock = value,
            PanelRegion::Bars => self.bars = value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BreatheConfig {
    pub full_wave: bool,
    pub pulse: bool,
    pub shimmer: bool,
    pub auto_morph: bool,
    pub speed_pct: f32,
}

impl Default for BreatheConfig {
    fn default() -> Self {
        Self {
            full_wave: true,
            pulse: true,
            shimmer: true,
            auto_morph: false,
            speed_pct: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct PersistedGradientStudio {
    enabled: bool,
    palette_name: String,
    stops: FourStops,
    intensity: f32,
    breathe: bool,
    breathe_cfg: BreatheConfig,
    seamless: bool,
    panels: PanelMask,
    component_palettes: ComponentPalettes,
    custom_palettes: Vec<CustomPalette>,
    removed_palettes: Vec<String>,
    favorites: Vec<String>,
    fav_only: bool,
    base_angle: f32,
    breathe_paused_delta: Option<f32>,
}

impl Default for PersistedGradientStudio {
    fn default() -> Self {
        Self {
            enabled: true,
            palette_name: DEFAULT_PALETTE_NAME.to_string(),
            stops: DEFAULT_STOPS,
            intensity: 1.0,
            breathe: false,
            breathe_cfg: BreatheConfig::default(),
            seamless: true,
            panels: PanelMask::default(),
            component_palettes: ComponentPalettes::default(),
            custom_palettes: Vec::new(),
            removed_palettes: Vec::new(),
            favorites: Vec::new(),
            fav_only: false,
            base_angle: 135.0,
            breathe_paused_delta: None,
        }
    }
}

pub struct GradientStudioState {
    pub enabled: bool,
    pub palette_name: String,
    pub stops: FourStops,
    pub intensity: f32,
    pub breathe: bool,
    pub breathe_cfg: BreatheConfig,
    pub seamless: bool,
    pub panels: PanelMask,
    pub component_palettes: ComponentPalettes,
    pub custom_palettes: Vec<CustomPalette>,
    pub removed_palettes: Vec<String>,
    pub favorites: Vec<String>,
    pub fav_only: bool,
    pub base_angle: f32,
    pub history: Vec<(String, FourStops, f32)>,
    pub breathe_origin: Instant,
    pub breathe_paused_delta: Option<f32>,
    visual_revision: u64,
    dirty: bool,
    save_task: Option<Task<()>>,
}

impl Global for GradientStudioState {}

impl Default for GradientStudioState {
    fn default() -> Self {
        Self::from_persisted(PersistedGradientStudio::default())
    }
}

impl GradientStudioState {
    fn from_persisted(p: PersistedGradientStudio) -> Self {
        let mut cfg = p.breathe_cfg;
        cfg.speed_pct = cfg.speed_pct.clamp(25.0, 250.0);
        Self {
            enabled: p.enabled,
            palette_name: p.palette_name,
            stops: p.stops,
            intensity: p.intensity.clamp(0.2, 3.0),
            breathe: p.breathe,
            breathe_cfg: cfg,
            seamless: p.seamless,
            panels: p.panels,
            component_palettes: p.component_palettes,
            custom_palettes: p.custom_palettes,
            removed_palettes: p.removed_palettes,
            favorites: p.favorites,
            fav_only: p.fav_only,
            base_angle: p.base_angle,
            history: Vec::new(),
            breathe_origin: Instant::now(),
            breathe_paused_delta: p.breathe_paused_delta.map(|d| d.rem_euclid(1.0)),
            visual_revision: 0,
            dirty: false,
            save_task: None,
        }
    }

    fn to_persisted(&self) -> PersistedGradientStudio {
        PersistedGradientStudio {
            enabled: self.enabled,
            palette_name: self.palette_name.clone(),
            stops: self.stops,
            intensity: self.intensity,
            breathe: self.breathe,
            breathe_cfg: self.breathe_cfg,
            seamless: self.seamless,
            panels: self.panels,
            component_palettes: self.component_palettes.clone(),
            custom_palettes: self.custom_palettes.clone(),
            removed_palettes: self.removed_palettes.clone(),
            favorites: self.favorites.clone(),
            fav_only: self.fav_only,
            base_angle: self.base_angle,
            breathe_paused_delta: if self.breathe {
                self.current_breathe_delta()
            } else {
                self.breathe_paused_delta
            },
        }
    }

    fn config_path() -> PathBuf {
        paths::config_dir().join("gradient_studio.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<PersistedGradientStudio>(&contents) {
                Ok(persisted) => Self::from_persisted(persisted),
                Err(err) => {
                    log::warn!("Failed to parse {}: {err}", path.display());
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    fn write_snapshot_atomic(snapshot: &PersistedGradientStudio) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(snapshot) {
            let tmp_path = path.with_extension("json.tmp");
            if std::fs::write(&tmp_path, json).is_ok() {
                let _ = std::fs::rename(tmp_path, path);
            }
        }
    }

    pub fn visual_revision(&self) -> u64 {
        self.visual_revision
    }

    fn mark_visual_changed(&mut self) {
        self.visual_revision = self.visual_revision.wrapping_add(1);
        self.dirty = true;
    }

    fn mark_metadata_changed(&mut self) {
        self.dirty = true;
    }

    fn push_history(&mut self) {
        self.history
            .push((self.palette_name.clone(), self.stops, self.base_angle));
        if self.history.len() > 24 {
            self.history.remove(0);
        }
    }

    pub fn resolved_stops(&self, region: Option<PanelRegion>) -> FourStops {
        if let Some(panel) = region {
            if let Some(custom) = self.component_palettes.get(panel) {
                return custom.stops;
            }
        }
        self.stops
    }

    pub fn resolved_name(&self, region: Option<PanelRegion>) -> &str {
        if let Some(panel) = region {
            if let Some(custom) = self.component_palettes.get(panel) {
                return &custom.name;
            }
        }
        &self.palette_name
    }

    pub fn select_palette_for(
        &mut self,
        scope: Option<PanelRegion>,
        name: impl Into<String>,
        stops: FourStops,
    ) {
        let name = name.into();
        match scope {
            None => {
                if self.enabled && self.palette_name == name && self.stops == stops {
                    return;
                }
                self.push_history();
                self.palette_name = name;
                self.stops = stops;
                self.enabled = true;
                self.breathe_paused_delta = None;
                self.breathe_origin = Instant::now();
                self.mark_visual_changed();
            }
            Some(panel) => {
                self.enabled = true;
                self.component_palettes
                    .set(panel, Some(CustomPalette { name, stops }));
                if !self.panels.is_enabled(panel) {
                    self.panels.toggle(panel);
                }
                self.breathe_paused_delta = None;
                self.breathe_origin = Instant::now();
                self.mark_visual_changed();
            }
        }
    }

    pub fn select_palette(&mut self, name: impl Into<String>, stops: FourStops) {
        self.select_palette_for(None, name, stops);
    }

    pub fn set_stop_color(&mut self, scope: Option<PanelRegion>, stop_idx: usize, rgb: Rgb) {
        if stop_idx >= 4 {
            return;
        }
        match scope {
            None => {
                let mut next = self.stops;
                next[stop_idx] = rgb;
                self.stops = next;
                self.palette_name = "Custom Mix".to_string();
                self.enabled = true;
                self.breathe_paused_delta = None;
                self.breathe_origin = Instant::now();
                self.mark_visual_changed();
            }
            Some(panel) => {
                let mut next = self.resolved_stops(Some(panel));
                next[stop_idx] = rgb;
                self.enabled = true;
                self.component_palettes.set(
                    panel,
                    Some(CustomPalette {
                        name: "Custom Mix".to_string(),
                        stops: next,
                    }),
                );
                self.breathe_paused_delta = None;
                self.breathe_origin = Instant::now();
                self.mark_visual_changed();
            }
        }
    }

    pub fn clear_component_palette(&mut self, panel: PanelRegion) {
        if self.component_palettes.get(panel).is_some() {
            self.component_palettes.set(panel, None);
            self.mark_visual_changed();
        }
    }

    pub fn save_custom_palette(
        &mut self,
        scope: Option<PanelRegion>,
        name: String,
        stops: FourStops,
    ) {
        let trimmed = name.trim();
        let final_name = if trimmed.is_empty() {
            format!("Custom {}", self.custom_palettes.len() + 1)
        } else {
            trimmed.to_string()
        };
        if let Some(existing) = self
            .custom_palettes
            .iter_mut()
            .find(|p| p.name == final_name)
        {
            existing.stops = stops;
        } else {
            self.custom_palettes.insert(
                0,
                CustomPalette {
                    name: final_name.clone(),
                    stops,
                },
            );
        }
        self.select_palette_for(scope, final_name, stops);
    }

    pub fn remove_palette(&mut self, name: &str) {
        let before = self.custom_palettes.len();
        self.custom_palettes.retain(|p| p.name != name);
        if self.custom_palettes.len() == before && !self.removed_palettes.iter().any(|r| r == name)
        {
            self.removed_palettes.push(name.to_string());
        }
        self.favorites.retain(|f| f != name);
        self.mark_metadata_changed();
    }

    pub fn is_removed(&self, name: &str) -> bool {
        self.removed_palettes.iter().any(|r| r == name)
    }

    pub fn roll_random_for(&mut self, scope: Option<PanelRegion>) {
        let mut rng = rand::rng();

        let available_curated: Vec<_> = CURATED_PALETTES
            .iter()
            .filter(|p| !self.is_removed(p.name))
            .collect();

        if self.fav_only && !self.favorites.is_empty() {
            let candidates: Vec<_> = available_curated
                .iter()
                .filter(|p| self.favorites.iter().any(|f| f == p.name))
                .collect();
            if !candidates.is_empty() {
                let mut picked = candidates[rng.random_range(0..candidates.len())];
                if candidates.len() > 1 && picked.name == self.resolved_name(scope) {
                    picked = candidates[rng.random_range(0..candidates.len())];
                }
                self.base_angle = rng.random_range(105.0_f32..165.0_f32);
                self.select_palette_for(scope, picked.name, picked.stops);
                return;
            }
        }

        // 75% curated preset, 25% fresh procedural aurora mix
        if !available_curated.is_empty() && rng.random_range(0..4) != 0 {
            let mut picked = available_curated[rng.random_range(0..available_curated.len())];
            if available_curated.len() > 1 && picked.name == self.resolved_name(scope) {
                picked = available_curated[rng.random_range(0..available_curated.len())];
            }
            self.base_angle = rng.random_range(105.0_f32..165.0_f32);
            self.select_palette_for(scope, picked.name, picked.stops);
        } else {
            let (name, stops) = generate_procedural_palette();
            self.base_angle = rng.random_range(105.0_f32..165.0_f32);
            self.select_palette_for(scope, name, stops);
        }
    }

    pub fn roll_random(&mut self) {
        self.roll_random_for(None);
    }

    pub fn previous_palette(&mut self) -> bool {
        if let Some((name, stops, angle)) = self.history.pop() {
            self.palette_name = name;
            self.stops = stops;
            self.base_angle = angle;
            self.enabled = true;
            self.breathe_paused_delta = None;
            self.breathe_origin = Instant::now();
            self.mark_visual_changed();
            true
        } else {
            false
        }
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
        self.mark_visual_changed();
    }

    pub fn current_cycle_secs(&self) -> f32 {
        let speed_mult = (self.breathe_cfg.speed_pct / 100.0).clamp(0.25, 2.5);
        (18.0 / speed_mult).clamp(5.0, 72.0)
    }

    pub fn current_breathe_delta(&self) -> Option<f32> {
        let any_motion = self.breathe_cfg.full_wave
            || self.breathe_cfg.pulse
            || self.breathe_cfg.shimmer
            || self.breathe_cfg.auto_morph;
        if !any_motion {
            return None;
        }
        if self.breathe {
            let base = self.breathe_paused_delta.unwrap_or(0.0);
            let elapsed = self.breathe_origin.elapsed().as_secs_f32() / self.current_cycle_secs();
            Some((base + elapsed).rem_euclid(1.0))
        } else {
            self.breathe_paused_delta
        }
    }

    pub fn toggle_breathe(&mut self) {
        if self.breathe {
            self.breathe_paused_delta = self.current_breathe_delta();
            self.breathe = false;
        } else {
            self.breathe_origin = Instant::now();
            self.breathe = true;
        }
        self.mark_visual_changed();
    }

    pub fn toggle_breathe_wave(&mut self) {
        self.breathe_cfg.full_wave = !self.breathe_cfg.full_wave;
        if !self.breathe {
            self.breathe_origin = Instant::now();
            self.breathe = true;
        }
        self.mark_visual_changed();
    }

    pub fn toggle_breathe_pulse(&mut self) {
        self.breathe_cfg.pulse = !self.breathe_cfg.pulse;
        if !self.breathe {
            self.breathe_origin = Instant::now();
            self.breathe = true;
        }
        self.mark_visual_changed();
    }

    pub fn toggle_breathe_shimmer(&mut self) {
        self.breathe_cfg.shimmer = !self.breathe_cfg.shimmer;
        if !self.breathe {
            self.breathe_origin = Instant::now();
            self.breathe = true;
        }
        self.mark_visual_changed();
    }

    pub fn toggle_breathe_auto_morph(&mut self) {
        self.breathe_cfg.auto_morph = !self.breathe_cfg.auto_morph;
        if !self.breathe {
            self.breathe_origin = Instant::now();
            self.breathe = true;
        }
        self.mark_visual_changed();
    }

    pub fn set_breathe_speed(&mut self, pct: f32) {
        let next = ((pct / 5.0).round() * 5.0).clamp(25.0, 250.0);
        if (next - self.breathe_cfg.speed_pct).abs() < f32::EPSILON {
            return;
        }
        if self.breathe {
            self.breathe_paused_delta = self.current_breathe_delta();
            self.breathe_origin = Instant::now();
        }
        self.breathe_cfg.speed_pct = next;
        self.mark_visual_changed();
    }

    pub fn toggle_seamless(&mut self) {
        self.seamless = !self.seamless;
        self.mark_visual_changed();
    }

    pub fn toggle_panel(&mut self, region: PanelRegion) {
        self.panels.toggle(region);
        self.mark_visual_changed();
    }

    pub fn swap_corners(&mut self) {
        self.push_history();
        self.stops.rotate_left(1);
        self.enabled = true;
        self.breathe_paused_delta = None;
        self.breathe_origin = Instant::now();
        self.mark_visual_changed();
    }

    pub fn set_intensity(&mut self, val: f32) {
        let next = ((val * 20.0).round() / 20.0).clamp(0.2, 3.0);
        if (next - self.intensity).abs() < f32::EPSILON {
            return;
        }
        self.intensity = next;
        self.mark_visual_changed();
    }

    pub fn adjust_intensity(&mut self, delta: f32) {
        self.set_intensity(self.intensity + delta);
    }

    pub fn reset_intensity(&mut self) {
        self.set_intensity(1.0);
    }

    pub fn is_favorite(&self, name: &str) -> bool {
        self.favorites.iter().any(|f| f == name)
    }

    pub fn toggle_favorite(&mut self, name: &str) {
        if let Some(pos) = self.favorites.iter().position(|f| f == name) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(name.to_string());
        }
        self.mark_metadata_changed();
    }

    pub fn toggle_fav_only(&mut self) {
        self.fav_only = !self.fav_only;
        self.mark_metadata_changed();
    }

    /// Computes the 4 radial mesh focal centers in normalized `[0.0..=1.0]` UV space.
    pub fn focal_points_at(&self, phase_radians: Option<f32>) -> [(f32, f32); 4] {
        let base = [(0.15, 0.15), (0.85, 0.25), (0.25, 0.85), (0.85, 0.85)];
        let Some(t) = phase_radians else {
            return base;
        };
        if !self.breathe_cfg.full_wave {
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
}

/// Mutates the global `GradientStudioState` and schedules a debounced atomic background save
/// so UI interactions never block the main GPUI thread on disk I/O.
pub fn update_state(cx: &mut App, f: impl FnOnce(&mut GradientStudioState)) {
    if !cx.has_global::<GradientStudioState>() {
        return;
    }
    let visual_changed = cx.update_global::<GradientStudioState, _>(|state, cx| {
        let prev_rev = state.visual_revision;
        state.dirty = false;
        f(state);
        if state.dirty {
            state.dirty = false;
            let snapshot = state.to_persisted();
            let executor = cx.background_executor();
            let timer = executor.timer(SAVE_DEBOUNCE);
            state.save_task = Some(executor.spawn(async move {
                timer.await;
                GradientStudioState::write_snapshot_atomic(&snapshot);
            }));
        }
        state.visual_revision != prev_rev
    });
    if visual_changed {
        crate::theme_glass::sync_chrome_state(cx);
        cx.refresh_windows();
    }
}
