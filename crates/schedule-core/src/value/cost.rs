/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`AdditionalCost`] — typed cost classification for panel entities.
//!
//! Replaces the raw `cost: Option<String>` field, making invalid states
//! unrepresentable. The three variants cover every observable cost category:
//!
//! - [`AdditionalCost::Included`] — admission is included (free / N/A / blank
//!   non-workshop).
//! - [`AdditionalCost::TBD`] — cost is not yet determined (explicit TBD string,
//!   or blank when the panel type is a workshop).
//! - [`AdditionalCost::Premium(u64)`] — an extra charge applies; amount stored as
//!   integer cents (e.g. `3500` = $35.00).
//!
//! A separate `for_kids: bool` flag indicates panels aimed at a younger audience;
//! it is independent of cost (a kids panel may be `Included` *or* `Premium`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ── AdditionalCost ────────────────────────────────────────────────────────────

/// Cost classification for a panel — whether admission is included, TBD, or
/// requires an extra charge (stored as integer cents).
///
/// This is the **stored** representation.  The human-readable cost string (e.g.
/// `"$35"`, `"TBD"`) is a **computed** field derived from this value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "cents", rename_all = "snake_case")]
pub enum AdditionalCost {
    /// No extra charge; admission is included with convention badge.
    #[default]
    Included,
    /// Cost has not been determined yet.
    TBD,
    /// Extra charge required; amount in integer cents (e.g. `3500` = $35.00).
    Premium(u64),
}

impl fmt::Display for AdditionalCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Included => write!(f, "included"),
            Self::TBD => write!(f, "tbd"),
            Self::Premium(cents) => {
                let dollars = cents / 100;
                let remainder = cents % 100;
                if remainder == 0 {
                    write!(f, "${dollars}")
                } else {
                    write!(f, "${dollars}.{remainder:02}")
                }
            }
        }
    }
}

impl FromStr for AdditionalCost {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_additional_cost(s).ok_or_else(|| format!("cannot parse cost: {s:?}"))
    }
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

use std::sync::LazyLock;

/// Pattern matching `"Kids"` (case-insensitive, optional trailing `s`).
static RE_KIDS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\Akids?\z").expect("RE_KIDS"));

/// Pattern for a TBD-style cost: optional leading `$`, then `T`, optional
/// `.`, `B`, optional `.`, `D`, optional `.` (case-insensitive).
static RE_TBD: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\A\$?T\.?B\.?D\.?\z").expect("RE_TBD"));

/// Pattern for free/included costs (mirrors Perl `RE_FREE`):
/// `free`, `nothing`, `n/a`, `na`, or a zero dollar amount.
static RE_FREE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?ix)\A(?:free|nothing|n/?a|\$?(?:0+(?:[.]0+)?|[.]0+))\z").expect("RE_FREE")
});

/// Parse a raw cost string into an [`AdditionalCost`].
///
/// - Blank / `*` → `None` (caller decides default based on panel type).
/// - `"Kids"` / `"Kid"` (case-insensitive) → `None` (for_kids is separate).
/// - `"Free"`, `"$0"`, `"N/A"`, etc. → `Some(Included)`.
/// - `"TBD"`, `"$TBD"`, etc. → `Some(TBD)`.
/// - `"$35"`, `"$35.50"` → `Some(Premium(cents))`.
/// - Unrecognized non-empty string → `None` (treated as unknown).
///
/// The `is_workshop` flag controls the blank-input default:
/// blank+workshop → `TBD`, blank+non-workshop → `Included`.
#[must_use]
pub fn parse_additional_cost(s: &str) -> Option<AdditionalCost> {
    let text = s.trim();
    if text.is_empty() || text == "*" {
        return None;
    }
    if RE_KIDS.is_match(text) {
        // Kids panels are free; for_kids flag is set separately by the caller.
        return Some(AdditionalCost::Included);
    }
    if RE_FREE.is_match(text) {
        return Some(AdditionalCost::Included);
    }
    if RE_TBD.is_match(text) {
        return Some(AdditionalCost::TBD);
    }
    // Try to parse a dollar amount into cents.
    if let Some(cents) = parse_cents(text) {
        return Some(AdditionalCost::Premium(cents));
    }
    None
}

/// Returns `true` when the raw cost string is a kids-panel marker.
#[must_use]
pub fn cost_string_is_kid_panel(s: &str) -> bool {
    RE_KIDS.is_match(s.trim())
}

/// Parse a dollar string (e.g. `"$35"`, `"$35.50"`) into integer cents.
/// Returns `None` if the string is not a parseable positive dollar amount.
fn parse_cents(text: &str) -> Option<u64> {
    let stripped = text.trim_start_matches('$');
    if stripped.is_empty() {
        return None;
    }
    if let Some((dollars, cents_str)) = stripped.split_once('.') {
        let d: u64 = dollars.parse().ok()?;
        let padded = format!("{cents_str:0<2}");
        let c: u64 = padded[..2].parse().ok()?;
        Some(d * 100 + c)
    } else {
        let d: u64 = stripped.parse().ok()?;
        Some(d * 100)
    }
}

/// Synthesize a human-readable cost string from an [`AdditionalCost`] value.
///
/// Priority: `Premium` → dollar string, `TBD` → `"TBD"`,
/// `Included` → `None` (omitted in output).
#[must_use]
pub fn additional_cost_to_string(cost: &AdditionalCost) -> Option<String> {
    match cost {
        AdditionalCost::Included => None,
        AdditionalCost::TBD => Some("TBD".to_string()),
        AdditionalCost::Premium(cents) => {
            let dollars = cents / 100;
            let remainder = cents % 100;
            if remainder == 0 {
                Some(format!("${dollars}"))
            } else {
                Some(format!("${dollars}.{remainder:02}"))
            }
        }
    }
}

// ── FieldTypeMapping for AdditionalCost ───────────────────────────────────────

use crate::value::{ConversionError, FieldTypeItem, FieldValueItem};

/// Marker type for converting to [`AdditionalCost`].
pub struct AsAdditionalCost;

impl crate::query::converter::FieldTypeMapping for AsAdditionalCost {
    type Output = AdditionalCost;
    const FIELD_TYPE_ITEM: FieldTypeItem = FieldTypeItem::AdditionalCost;

    fn from_field_value_item(item: FieldValueItem) -> Result<Self::Output, ConversionError> {
        match item {
            FieldValueItem::AdditionalCost(c) => Ok(c),
            FieldValueItem::String(s) => s
                .parse()
                .map_err(|e: String| ConversionError::ParseError { message: e }),
            FieldValueItem::Integer(0) => Ok(AdditionalCost::Included),
            FieldValueItem::Integer(n) => Ok(AdditionalCost::Premium(
                u64::try_from(n).map_err(|_| ConversionError::ParseError {
                    message: format!("Cannot convert negative integer {} to AdditionalCost", n),
                })? * 100,
            )),
            FieldValueItem::Float(f) => {
                let cents = (f * 100.0).round();
                if cents < 0.0 {
                    Err(ConversionError::ParseError {
                        message: format!("Cannot convert negative float {} to AdditionalCost", f),
                    })
                } else if cents == 0.0 {
                    Ok(AdditionalCost::Included)
                } else {
                    Ok(AdditionalCost::Premium(cents as u64))
                }
            }
            _ => Err(ConversionError::ParseError {
                message: "Cannot convert to AdditionalCost from this type".to_string(),
            }),
        }
    }

    fn to_field_value_item(output: Self::Output) -> FieldValueItem {
        FieldValueItem::AdditionalCost(output)
    }
}

// ── IntoFieldValueItem for AdditionalCost ─────────────────────────────────────

impl crate::value::IntoFieldValueItem for AdditionalCost {
    fn into_field_value_item(self) -> FieldValueItem {
        FieldValueItem::AdditionalCost(self)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_additional_cost_premium() {
        assert_eq!(
            parse_additional_cost("$35"),
            Some(AdditionalCost::Premium(3500))
        );
        assert_eq!(
            parse_additional_cost("$35.50"),
            Some(AdditionalCost::Premium(3550))
        );
        assert_eq!(
            parse_additional_cost("$0.50"),
            Some(AdditionalCost::Premium(50))
        );
    }

    #[test]
    fn test_parse_additional_cost_included() {
        assert_eq!(
            parse_additional_cost("Free"),
            Some(AdditionalCost::Included)
        );
        assert_eq!(
            parse_additional_cost("free"),
            Some(AdditionalCost::Included)
        );
        assert_eq!(parse_additional_cost("$0"), Some(AdditionalCost::Included));
        assert_eq!(
            parse_additional_cost("$0.00"),
            Some(AdditionalCost::Included)
        );
        assert_eq!(parse_additional_cost("N/A"), Some(AdditionalCost::Included));
        assert_eq!(parse_additional_cost("n/a"), Some(AdditionalCost::Included));
        assert_eq!(
            parse_additional_cost("nothing"),
            Some(AdditionalCost::Included)
        );
    }

    #[test]
    fn test_parse_additional_cost_kids_is_included() {
        assert_eq!(
            parse_additional_cost("Kids"),
            Some(AdditionalCost::Included)
        );
        assert_eq!(
            parse_additional_cost("kids"),
            Some(AdditionalCost::Included)
        );
        assert_eq!(parse_additional_cost("Kid"), Some(AdditionalCost::Included));
    }

    #[test]
    fn test_parse_additional_cost_tbd() {
        assert_eq!(parse_additional_cost("TBD"), Some(AdditionalCost::TBD));
        assert_eq!(parse_additional_cost("tbd"), Some(AdditionalCost::TBD));
        assert_eq!(parse_additional_cost("$TBD"), Some(AdditionalCost::TBD));
        assert_eq!(parse_additional_cost("T.B.D."), Some(AdditionalCost::TBD));
    }

    #[test]
    fn test_parse_additional_cost_blank_is_none() {
        assert_eq!(parse_additional_cost(""), None);
        assert_eq!(parse_additional_cost("  "), None);
        assert_eq!(parse_additional_cost("*"), None);
    }

    #[test]
    fn test_cost_string_is_kid_panel() {
        assert!(cost_string_is_kid_panel("Kids"));
        assert!(cost_string_is_kid_panel("Kid"));
        assert!(cost_string_is_kid_panel("kids"));
        assert!(!cost_string_is_kid_panel("Free"));
        assert!(!cost_string_is_kid_panel("$35"));
        assert!(!cost_string_is_kid_panel(""));
    }

    #[test]
    fn test_additional_cost_to_string() {
        assert_eq!(additional_cost_to_string(&AdditionalCost::Included), None);
        assert_eq!(
            additional_cost_to_string(&AdditionalCost::TBD),
            Some("TBD".into())
        );
        assert_eq!(
            additional_cost_to_string(&AdditionalCost::Premium(3500)),
            Some("$35".into())
        );
        assert_eq!(
            additional_cost_to_string(&AdditionalCost::Premium(3550)),
            Some("$35.50".into())
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(AdditionalCost::Included.to_string(), "included");
        assert_eq!(AdditionalCost::TBD.to_string(), "tbd");
        assert_eq!(AdditionalCost::Premium(3500).to_string(), "$35");
        assert_eq!(AdditionalCost::Premium(3550).to_string(), "$35.50");
    }

    #[test]
    fn test_default_is_included() {
        assert_eq!(AdditionalCost::default(), AdditionalCost::Included);
    }

    #[test]
    fn test_serde_round_trip() {
        let cases = [
            AdditionalCost::Included,
            AdditionalCost::TBD,
            AdditionalCost::Premium(3500),
            AdditionalCost::Premium(3550),
        ];
        for cost in &cases {
            let json = serde_json::to_string(cost).unwrap();
            let back: AdditionalCost = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, cost, "round-trip failed for {cost:?}");
        }
    }

    #[test]
    fn test_as_additional_cost_from_string_item() {
        use crate::query::converter::FieldTypeMapping;
        let item = FieldValueItem::String("$35".to_string());
        let result = AsAdditionalCost::from_field_value_item(item).unwrap();
        assert_eq!(result, AdditionalCost::Premium(3500));
    }

    #[test]
    fn test_as_additional_cost_from_enum_item() {
        use crate::query::converter::FieldTypeMapping;
        let item = FieldValueItem::AdditionalCost(AdditionalCost::TBD);
        let result = AsAdditionalCost::from_field_value_item(item).unwrap();
        assert_eq!(result, AdditionalCost::TBD);
    }
}
