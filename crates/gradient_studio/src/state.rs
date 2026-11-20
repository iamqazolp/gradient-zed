use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{App, BorrowAppContext as _, Global, Task};
use rand::Rng as _;
use serde::{Deserialize, Serialize};

use crate::palettes::{
    CURATED_PALETTES, DEFAULT_PALETTE_NAME, DEFAULT_STOPS, FourStops, generate_procedural_palette,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct PersistedGradientStudio {
    enabled: bool,
    palette_name: String,
    stops: FourStops,
    intensity: f32,
    breathe: bool,
    seamless: bool,
    panels: PanelMask,
    favorites: Vec<String>,
    fav_only: bool,
    base_angle: f32,
}

impl Default for PersistedGradientStudio {
    fn default() -> Self {
        Self {
            enabled: true,
            palette_name: DEFAULT_PALETTE_NAME.to_string(),
            stops: DEFAULT_STOPS,
            intensity: 1.0,
            breathe: false,
            seamless: true,
            panels: PanelMask::default(),
            favorites: Vec::new(),
            fav_only: false,
            base_angle: 135.0,
        }
    }
}

pub struct GradientStudioState {
    pub enabled: bool,
    pub palette_name: String,
    pub stops: FourStops,
    pub intensity: f32,
    pub breathe: bool,
    pub seamless: bool,
    pub panels: PanelMask,
    pub favorites: Vec<String>,
    pub fav_only: bool,
    pub base_angle: f32,
    pub history: Vec<(String, FourStops, f32)>,
    pub breathe_origin: Instant,
    visual_revision: u64,
    dirty: bool,
    save_task: Option<Task<()>>,
}

impl Global for GradientStudioState {}

impl Default for GradientStudioState {
    fn default() -> Self {
        let persisted = PersistedGradientStudio::default();
        Self::from_persisted(persisted)
    }
}

impl GradientStudioState {
    fn from_persisted(p: PersistedGradientStudio) -> Self {
        Self {
            enabled: p.enabled,
            palette_name: p.palette_name,
            stops: p.stops,
            intensity: p.intensity.clamp(0.2, 3.0),
            breathe: p.breathe,
            seamless: p.seamless,
            panels: p.panels,
            favorites: p.favorites,
            fav_only: p.fav_only,
            base_angle: p.base_angle,
            history: Vec::new(),
            breathe_origin: Instant::now(),
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
            seamless: self.seamless,
            panels: self.panels,
            favorites: self.favorites.clone(),
            fav_only: self.fav_only,
            base_angle: self.base_angle,
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

    pub fn select_palette(&mut self, name: impl Into<String>, stops: FourStops) {
        let name = name.into();
        if self.enabled && self.palette_name == name && self.stops == stops {
            return;
        }
        self.push_history();
        self.palette_name = name;
        self.stops = stops;
        self.enabled = true;
        self.mark_visual_changed();
    }

    pub fn roll_random(&mut self) {
        self.push_history();
        let mut rng = rand::rng();

        if self.fav_only && !self.favorites.is_empty() {
            let candidates: Vec<_> = CURATED_PALETTES
                .iter()
                .filter(|p| self.favorites.iter().any(|f| f == p.name))
                .collect();
            if !candidates.is_empty() {
                let mut picked = candidates[rng.random_range(0..candidates.len())];
                if candidates.len() > 1 && picked.name == self.palette_name {
                    picked = candidates[rng.random_range(0..candidates.len())];
                }
                self.palette_name = picked.name.to_string();
                self.stops = picked.stops;
                self.base_angle = rng.random_range(105.0_f32..165.0_f32);
                self.enabled = true;
                self.mark_visual_changed();
                return;
            }
        }

        // 75% curated preset, 25% fresh procedural aurora mix
        if rng.random_range(0..4) != 0 {
            let mut picked = &CURATED_PALETTES[rng.random_range(0..CURATED_PALETTES.len())];
            if picked.name == self.palette_name {
                picked = &CURATED_PALETTES[rng.random_range(0..CURATED_PALETTES.len())];
            }
            self.palette_name = picked.name.to_string();
            self.stops = picked.stops;
        } else {
            let (name, stops) = generate_procedural_palette();
            self.palette_name = name;
            self.stops = stops;
        }

        self.base_angle = rng.random_range(105.0_f32..165.0_f32);
        self.enabled = true;
        self.mark_visual_changed();
    }

    pub fn previous_palette(&mut self) -> bool {
        if let Some((name, stops, angle)) = self.history.pop() {
            self.palette_name = name;
            self.stops = stops;
            self.base_angle = angle;
            self.enabled = true;
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

    pub fn toggle_breathe(&mut self) {
        self.breathe = !self.breathe;
        if self.breathe {
            self.breathe_origin = Instant::now();
        }
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
        self.mark_visual_changed();
    }

    pub fn adjust_intensity(&mut self, delta: f32) {
        let next = (((self.intensity + delta) * 20.0).round() / 20.0).clamp(0.2, 3.0);
        if (next - self.intensity).abs() < f32::EPSILON {
            return;
        }
        self.intensity = next;
        self.mark_visual_changed();
    }

    pub fn reset_intensity(&mut self) {
        if (self.intensity - 1.0).abs() < f32::EPSILON {
            return;
        }
        self.intensity = 1.0;
        self.mark_visual_changed();
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
    /// `phase_radians` drives gentle harmonic Lissajous orbits when Breathe mode is enabled.
    pub fn focal_points_at(&self, phase_radians: Option<f32>) -> [(f32, f32); 4] {
        let base = [(0.15, 0.15), (0.85, 0.25), (0.25, 0.85), (0.85, 0.85)];
        let Some(t) = phase_radians else {
            return base;
        };
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
    cx.update_global::<GradientStudioState, _>(|state, cx| {
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
    });
}
