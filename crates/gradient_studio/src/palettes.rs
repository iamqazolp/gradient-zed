use gpui::{Hsla, Rgba};
use rand::Rng as _;
use serde::{Deserialize, Serialize};

pub type Rgb = [u8; 3];
pub type FourStops = [Rgb; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCategory {
    Prism,
    Pastel,
    Pure,
    Custom,
}

#[derive(Debug, Clone, Copy)]
pub struct CuratedPalette {
    pub name: &'static str,
    pub stops: FourStops,
    pub category: PaletteCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPalette {
    pub name: String,
    pub stops: FourStops,
}

pub const DEFAULT_PALETTE_NAME: &str = "Gradient Classic";

pub const DEFAULT_STOPS: FourStops = [
    [234, 67, 53],  // #EA4335 Red
    [251, 188, 4],  // #FBBC04 Yellow
    [52, 168, 83],  // #34A853 Green
    [66, 133, 244], // #4285F4 Blue
];

/// All 30 curated 4-stop aurora palettes ported from VSCode Gradient Themes,
/// categorized identically (`Prism` 0..12, `Pastel` 12..22, `Pure` 22..30).
pub const CURATED_PALETTES: &[CuratedPalette] = &[
    // --- Prism (12) ---
    CuratedPalette {
        name: DEFAULT_PALETTE_NAME,
        stops: DEFAULT_STOPS,
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Borealis",
        stops: [[0, 180, 216], [72, 202, 228], [6, 214, 160], [114, 9, 183]],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Sakura",
        stops: [
            [255, 117, 143],
            [255, 179, 198],
            [199, 125, 255],
            [76, 201, 240],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Matcha Yuzu",
        stops: [
            [82, 183, 136],
            [149, 213, 178],
            [250, 204, 21],
            [56, 189, 248],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Iris Dusk",
        stops: [
            [139, 92, 246],
            [236, 72, 153],
            [59, 130, 246],
            [45, 212, 191],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Sunset Lagoon",
        stops: [
            [251, 113, 133],
            [251, 191, 36],
            [45, 212, 191],
            [96, 165, 250],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Fjord",
        stops: [
            [56, 189, 248],
            [129, 140, 248],
            [52, 211, 153],
            [192, 132, 252],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Peach Sorbet",
        stops: [
            [251, 146, 60],
            [244, 114, 182],
            [250, 204, 21],
            [167, 139, 250],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Kyoto Autumn",
        stops: [
            [244, 63, 94],
            [249, 115, 22],
            [234, 179, 8],
            [139, 92, 246],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Coral Reef",
        stops: [
            [6, 182, 212],
            [16, 185, 129],
            [59, 130, 246],
            [99, 102, 241],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Rose Champagne",
        stops: [
            [251, 113, 133],
            [251, 191, 36],
            [244, 114, 182],
            [192, 132, 252],
        ],
        category: PaletteCategory::Prism,
    },
    CuratedPalette {
        name: "Glacier",
        stops: [
            [56, 189, 248],
            [129, 140, 248],
            [45, 212, 191],
            [232, 121, 249],
        ],
        category: PaletteCategory::Prism,
    },
    // --- Pastel (10) ---
    CuratedPalette {
        name: "Cafe Matcha",
        stops: [
            [52, 211, 153],
            [163, 230, 53],
            [251, 191, 36],
            [45, 212, 191],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Neon Pastel",
        stops: [
            [244, 63, 94],
            [139, 92, 246],
            [6, 182, 212],
            [250, 204, 21],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Desert Sun",
        stops: [
            [251, 146, 60],
            [244, 63, 94],
            [250, 204, 21],
            [56, 189, 248],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Orchid",
        stops: [
            [168, 85, 247],
            [236, 72, 153],
            [99, 102, 241],
            [20, 184, 166],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Tokyo Neon",
        stops: [
            [244, 63, 94],
            [217, 70, 239],
            [99, 102, 241],
            [6, 182, 212],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Amalfi Citrus",
        stops: [
            [250, 204, 21],
            [251, 146, 60],
            [52, 211, 153],
            [56, 189, 248],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Taro Milk Tea",
        stops: [
            [192, 132, 252],
            [244, 114, 182],
            [251, 191, 36],
            [129, 140, 248],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Emerald",
        stops: [
            [16, 185, 129],
            [6, 182, 212],
            [59, 130, 246],
            [168, 85, 247],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Sangria",
        stops: [
            [225, 29, 72],
            [219, 39, 119],
            [147, 51, 234],
            [245, 158, 11],
        ],
        category: PaletteCategory::Pastel,
    },
    CuratedPalette {
        name: "Sunbeam",
        stops: [
            [56, 189, 248],
            [45, 212, 191],
            [253, 224, 71],
            [129, 140, 248],
        ],
        category: PaletteCategory::Pastel,
    },
    // --- Pure (8) ---
    CuratedPalette {
        name: "Pure Cyan",
        stops: [
            [8, 145, 178],
            [6, 182, 212],
            [34, 211, 238],
            [103, 232, 249],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Peach",
        stops: [
            [234, 88, 12],
            [249, 115, 22],
            [251, 146, 60],
            [253, 186, 116],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Indigo",
        stops: [
            [79, 70, 229],
            [99, 102, 241],
            [129, 140, 248],
            [165, 180, 252],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Rose",
        stops: [
            [225, 29, 72],
            [244, 63, 94],
            [251, 113, 133],
            [253, 164, 175],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Blue",
        stops: [
            [37, 99, 235],
            [59, 130, 246],
            [96, 165, 250],
            [147, 197, 253],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Matcha",
        stops: [
            [5, 150, 105],
            [16, 185, 129],
            [52, 211, 153],
            [110, 231, 183],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Honey",
        stops: [
            [217, 119, 6],
            [245, 158, 11],
            [251, 191, 36],
            [253, 224, 71],
        ],
        category: PaletteCategory::Pure,
    },
    CuratedPalette {
        name: "Pure Violet",
        stops: [
            [124, 58, 237],
            [139, 92, 246],
            [167, 139, 250],
            [196, 181, 253],
        ],
        category: PaletteCategory::Pure,
    },
];

pub fn rgb_to_hsla(rgb: Rgb, alpha: f32) -> Hsla {
    Rgba {
        r: rgb[0] as f32 / 255.0,
        g: rgb[1] as f32 / 255.0,
        b: rgb[2] as f32 / 255.0,
        a: alpha.clamp(0.0, 1.0),
    }
    .into()
}

pub fn rgb_to_hsl(rgb: Rgb) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d <= 1e-5 {
        return (0.0, 0.0, l * 100.0);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if (max - r).abs() <= 1e-5 {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if (max - g).abs() <= 1e-5 {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    (h.rem_euclid(360.0), (s * 100.0).clamp(0.0, 100.0), (l * 100.0).clamp(0.0, 100.0))
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Rgb {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 100.0) / 100.0;
    let l = l.clamp(0.0, 100.0) / 100.0;
    let a = s * l.min(1.0 - l);
    let channel = |n: f32| -> u8 {
        let k = (n + h / 30.0) % 12.0;
        let val = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (val * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [channel(0.0), channel(8.0), channel(4.0)]
}

pub fn rgb_to_hex(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

pub fn lerp_stops(a: FourStops, b: FourStops, t: f32) -> FourStops {
    let t = t.clamp(0.0, 1.0);
    let mut out = [[0u8; 3]; 4];
    for i in 0..4 {
        for c in 0..3 {
            let v = a[i][c] as f32 + (b[i][c] as f32 - a[i][c] as f32) * t;
            out[i][c] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Extracts up to 4 `#RRGGBB` (or `#RGB`) hex colors from an arbitrary pasted string.
pub fn parse_four_hex_colors(input: &str) -> Option<FourStops> {
    let mut found: Vec<Rgb> = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() && found.len() < 4 {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            let hex = &input[start..end];
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    found.push([r, g, b]);
                }
            } else if hex.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                ) {
                    found.push([r * 17, g * 17, b * 17]);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    if found.len() == 4 {
        Some([found[0], found[1], found[2], found[3]])
    } else {
        None
    }
}

/// Generates a harmonious procedural 4-color aurora palette.
pub fn generate_procedural_palette() -> (String, FourStops) {
    let mut rng = rand::rng();
    let base_hue = rng.random_range(0.0_f32..360.0_f32);
    let mode = rng.random_range(0..4);
    let offsets: [f32; 4] = match mode {
        0 => [0.0, 42.0, 165.0, 215.0],
        1 => [0.0, 90.0, 180.0, 270.0],
        2 => [0.0, 28.0, 56.0, 88.0],
        _ => [0.0, 115.0, 230.0, 285.0],
    };

    let mut stops = [[0u8; 3]; 4];
    for (idx, offset) in offsets.iter().enumerate() {
        let sat = rng.random_range(68.0_f32..92.0_f32);
        let light = rng.random_range(52.0_f32..68.0_f32);
        stops[idx] = hsl_to_rgb(base_hue + *offset, sat, light);
    }

    let hue_tag = match (base_hue as u32) / 30 {
        0 => "Coral",
        1 => "Amber",
        2 => "Citrus",
        3 => "Jade",
        4 => "Mint",
        5 => "Lagoon",
        6 => "Glacier",
        7 => "Cobalt",
        8 => "Indigo",
        9 => "Violet",
        10 => "Orchid",
        _ => "Rose",
    };
    let name = format!("Aurora {hue_tag}");
    (name, stops)
}
