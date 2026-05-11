/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Color mode and panel type color resolution with grayscale fallback.

use crate::model::PanelTypeColors;

/// ITU-R BT.601 luma coefficients for converting sRGB to perceived brightness.
/// <https://www.itu.int/rec/R-REC-BT.601>
const LUMA_R: f64 = 0.299;
const LUMA_G: f64 = 0.587;
const LUMA_B: f64 = 0.114;

/// Whether to produce color or black-and-white output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    #[default]
    Color,
    Bw,
}

/// Resolved color for a panel type in the chosen mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelColor {
    /// Hex color string (e.g. `"#E2F9D7"`).
    pub hex: String,
}

impl PanelColor {
    /// Resolve the display color for the given mode.
    ///
    /// - `ColorMode::Color` → uses `colors.color` if present, else `None`
    /// - `ColorMode::Bw` → uses `colors.bw` if present; otherwise derives
    ///   grayscale from `colors.color` via ITU-R BT.601 luminance and returns
    ///   that as a left-border accent color (backgrounds stay white).
    pub fn resolve(colors: &PanelTypeColors, mode: ColorMode) -> Option<Self> {
        match mode {
            ColorMode::Color => colors.color.as_ref().map(|c| Self { hex: c.clone() }),
            ColorMode::Bw => {
                if let Some(bw) = &colors.bw {
                    if !bw.is_empty() {
                        return Some(Self { hex: bw.clone() });
                    }
                }
                // Derive grayscale from color via ITU-R BT.601 luminance
                colors
                    .color
                    .as_ref()
                    .and_then(|c| parse_hex_color(c))
                    .map(|(r, g, b)| {
                        let luma = (LUMA_R * r as f64 + LUMA_G * g as f64 + LUMA_B * b as f64)
                            .round() as u8;
                        Self {
                            hex: format!("#{:02X}{:02X}{:02X}", luma, luma, luma),
                        }
                    })
            }
        }
    }
}

/// Parse a `#RRGGBB` or `#RGB` hex color string into `(r, g, b)`.
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(color: &str, bw: &str) -> PanelTypeColors {
        PanelTypeColors {
            color: if color.is_empty() {
                None
            } else {
                Some(color.to_string())
            },
            bw: if bw.is_empty() {
                None
            } else {
                Some(bw.to_string())
            },
        }
    }

    #[test]
    fn test_color_mode_returns_color() {
        let c = colors("#E2F9D7", "");
        let result = PanelColor::resolve(&c, ColorMode::Color).unwrap();
        assert_eq!(result.hex, "#E2F9D7");
    }

    #[test]
    fn test_bw_mode_uses_bw_field_when_present() {
        let c = colors("#E2F9D7", "#CCCCCC");
        let result = PanelColor::resolve(&c, ColorMode::Bw).unwrap();
        assert_eq!(result.hex, "#CCCCCC");
    }

    #[test]
    fn test_bw_mode_derives_grayscale_when_bw_absent() {
        // #E2F9D7 = rgb(226, 249, 215)
        // luma = 0.299*226 + 0.587*249 + 0.114*215
        //       = 67.574 + 146.163 + 24.51 = 238.247 ≈ 238 = 0xEE
        let c = colors("#E2F9D7", "");
        let result = PanelColor::resolve(&c, ColorMode::Bw).unwrap();
        assert_eq!(result.hex, "#EEEEEE");
    }

    #[test]
    fn test_bw_mode_derives_grayscale_when_bw_empty_string() {
        let c = PanelTypeColors {
            color: Some("#000000".to_string()),
            bw: Some(String::new()),
        };
        let result = PanelColor::resolve(&c, ColorMode::Bw).unwrap();
        assert_eq!(result.hex, "#000000");
    }

    #[test]
    fn test_no_color_returns_none() {
        let c = PanelTypeColors {
            color: None,
            bw: None,
        };
        assert!(PanelColor::resolve(&c, ColorMode::Color).is_none());
        assert!(PanelColor::resolve(&c, ColorMode::Bw).is_none());
    }

    #[test]
    fn test_parse_hex_short_form() {
        assert_eq!(parse_hex_color("#FFF"), Some((255, 255, 255)));
        assert_eq!(parse_hex_color("#000"), Some((0, 0, 0)));
    }
}
