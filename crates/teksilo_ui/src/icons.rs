//! `res!()` SVG factories, grouped by where the icon appears.
//!
//! Pure asset lookups: no state, no logic. Tinting is left to `TextRole` on the
//! returned `IconWidget`, so every icon follows the theme without a second asset.
//! Where an icon means "off", that is a different shape rather than a dimmer
//! colour, because a dimmed glyph reads as "disabled".

pub mod action;
pub mod nav;
pub mod theme;
