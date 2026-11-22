# Gradient Zed

**Gradient Zed** is a thin, upstream-tracking fork of [Zed](https://github.com/zed-industries/zed) that brings the full **VSCode Gradient Themes** experience natively to Zed's 120 FPS GPU engine (Metal / WGPU / DirectX).

## What's Inside

- **True 4-Corner Radial Aurora GPU Shader** (`crates/gpui`, `crates/gpui_apple/src/shaders.metal`, `crates/gpui_wgpu/src/shaders.wgsl`, `crates/gpui_windows/src/shaders.hlsl`):
  - Adds `gpui::radial_gradient(...)` (`BackgroundTag::RadialGradient = 4`) with zero `Background` struct ABI change and built-in triangular dithering to eliminate 8-bit banding.
  - Blends softly over **any** Zed theme (One Dark, Catppuccin, Gruvbox, Tokyo Night, or the 12 built-in Gradient themes).
- **Gradient Studio Crate (`crates/gradient_studio/`)**:
  - **30 curated 4-color palettes** (`Gradient Classic`, `Borealis`, `Sakura`, `Matcha Yuzu`, `Iris Dusk`, `Glacier`, `Pure Blue`, etc.) + procedural harmonious randomizer.
  - **Status Bar Controls**: **`↻`** quick-roll button (right-click to toggle overlay) + **`Studio ▴`** live popover menu.
  - **Breathe Mode**: smooth Lissajous focal-point drift at native frame rate (automatically pauses when the window loses focus).
  - **Seamless / Per-Panel Modes**: one continuous aurora wash across the whole window, or toggle **Editor**, **Left**, **Right**, **Bottom**, and **Bars** independently.
  - **Intensity (`20%–300%`)**, **Favorites**, **Swap Corners**, **Undo history**, persisted to `~/.config/zed/gradient_studio.json`.
- **12 Built-in Gradient Themes** (`assets/themes/gradient/gradient.json`):
  - `Gradient — Light`, `Gradient — Dark`
  - `Gradient — Glacier Light`, `Gradient — Glacier Dark`
  - `Gradient — Matcha Light`, `Gradient — Pine Dark`
  - `Gradient — Rose Light`, `Gradient — Berry Dark`
  - `Gradient — Cream Light`, `Gradient — Espresso Dark`
  - `Gradient — Iris Light`, `Gradient — Plum Dark`
- **Auto-Updates & Upstream Sync**:
  - Points desktop app updates to `iamqazolp/gradient-zed` GitHub Releases so vanilla Zed never overwrites Gradient Zed.
  - `.github/workflows/sync-upstream.yml` checks daily for new `zed-industries/zed` stable releases and opens a sync PR automatically.
  - `.github/workflows/gradient-zed-release.yml` builds and publishes `GradientZed-aarch64.dmg` on GitHub Actions.

## Install & Update on macOS (Apple Silicon)

### Option 1: From Browser Download (DMG)

1. Download [`GradientZed-aarch64.dmg`](https://github.com/iamqazolp/gradient-zed/releases/latest/download/GradientZed-aarch64.dmg) and drag **Gradient Zed.app** into `/Applications`.
2. Because this community build is ad-hoc signed, clear the browser quarantine attribute before first launch (or click **Open Anyway** in **System Settings → Privacy & Security**):
   ```bash
   xattr -cr "/Applications/Gradient Zed.app"
   ```

### Option 2: One-Command Install / Update via Terminal

Downloads the latest release without browser quarantine, replaces `/Applications/Gradient Zed.app`, and clears extended attributes automatically:

```bash
curl -fL "https://github.com/iamqazolp/gradient-zed/releases/latest/download/GradientZed-aarch64.dmg" -o /tmp/GradientZed-aarch64.dmg && \
hdiutil attach -nobrowse /tmp/GradientZed-aarch64.dmg -mountroot /tmp && \
rsync -a --delete "/tmp/Gradient Zed/Gradient Zed.app/" "/Applications/Gradient Zed.app/" && \
xattr -cr "/Applications/Gradient Zed.app" && \
hdiutil detach "/tmp/Gradient Zed" && \
rm -f /tmp/GradientZed-aarch64.dmg
```

## Keyboard Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **Open Gradient Studio** | `Alt / Option + Shift + P` |
| **Roll Random Palette** | `Alt / Option + Shift + G` |
| **Previous Palette (Undo)** | `Alt / Option + Shift + Z` |
| **Toggle Breathe Animation** | `Alt / Option + Shift + B` |
| **Seamless / Per-Panel** | `Alt / Option + Shift + S` |
| **Swap Corner Colors** | `Alt / Option + Shift + R` |
| **Intensity Up / Down** | `Alt / Option + Shift + = / -` |
| **Toggle Overlay On / Off** | `Alt / Option + Shift + 0` |
