// Block-art sprites + helpers below `draw_pet` are kept on the shelf as
// a fallback in case the Kitty-graphics image path turns out flaky on
// some terminal. They're unreachable at runtime today, but the
// data + tests still document the original duo design — easier to
// re-enable than to redesign from scratch.
#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::state::AppState;

/// Pet animation state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Idle,
    WalkRight,
    Working,
    WalkLeft,
}

pub const PET_HOME_X: u16 = 1;
pub const DESK_OFFSET: u16 = 0;
pub const DESK_WIDTH: u16 = 4;
pub const CHAIR_WIDTH: u16 = 2;
/// Gap between chair and desk.
pub const CHAIR_DESK_GAP: u16 = 1;
pub const MAX_PAPER_HEIGHT: u16 = 2;
/// Ticks between idle bobs (~8 seconds at 200ms tick).
pub const BOB_INTERVAL: usize = 40;

// ── Theming sentinels ─────────────────────────────────────────────
//
// `recolor_sprite` swaps any glyph styled with `PET_BODY` for the
// theme's body color, and any glyph styled with `PET_EYE` for the
// theme's eye color. The literal values here are arbitrary indices
// chosen for the lookup; the tmux `@sidebar_color_pet_*` overrides do
// the user-facing recoloring at draw time.
const PET_BODY: Color = Color::Indexed(235);
const PET_EYE: Color = Color::Indexed(231);

// ── Hardcoded palette for the duo + nature scene ──────────────────
//
// These are NOT recolored — they're the fixed accents that distinguish
// Boston (white chest blaze) from the Frenchie, plus the prop colors
// for the mossy log, mushroom inkwell, and falling leaves. Theming the
// dog body / eye is enough; pinning the rest keeps the scene readable
// against any sidebar background.
const CHEST_COLOR: Color = Color::Indexed(231); // Boston's white chest blaze
const PET_NOSE: Color = Color::Indexed(174); // soft pink tongue / nose
const LOG_BODY: Color = Color::Indexed(94); // walnut brown — the log itself
const LOG_MOSS: Color = Color::Indexed(64); // forest moss on top of the log
const MUSHROOM_CAP: Color = Color::Indexed(124); // red mushroom cap
const MUSHROOM_STEM: Color = Color::Indexed(230); // cream stem
const LEAF_COLOR: Color = Color::Indexed(34); // green leaves drifting up

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdleMotion {
    Rest,
    Jump,
    Blink,
    Wave,
}

// ── Paired duo sprites ──────────────────────────────────────────────
//
// The duo occupies 11 columns: Frenchie (5 cols, all dark) + 1-col gap +
// Boston (5 cols, dark with a white chest blaze in the middle). Each
// row is padded to 11 columns so frame anchoring stays consistent and
// tests can read off a flat string. The rightmost dog (Boston) is the
// one closest to the log when working, so the writing-hand glyph and
// idle-wave gestures hang off Boston.

fn sitting_sprite() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn sitting_sprite_blink() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("─", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("─", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("─", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("─", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn sitting_sprite_wave() -> Vec<Line<'static>> {
    // Boston (right pup) raises a paw — the wave glyph hangs off the
    // right ear column, which keeps it from intruding into Frenchie.
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▘", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_right_1() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_right_2() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_right_3() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▘", Style::new().fg(PET_BODY)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▘", Style::new().fg(PET_BODY)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▘", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▘", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_left_1() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_left_2() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

fn walking_left_3() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▝", Style::new().fg(PET_BODY)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▝", Style::new().fg(PET_BODY)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(PET_NOSE)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("▖", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▗", Style::new().fg(CHEST_COLOR)),
            Span::raw(" "),
        ]),
    ]
}

/// Working sprite: Boston sits sideways at the log writing, Frenchie
/// stays seated upright next to her. Frame variations animate Boston's
/// writing hand (the right-edge glyph that mimics a quill stroke).
fn working_sprite_1() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("╴", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

fn working_sprite_2() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("─", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

fn working_sprite_3() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("╶", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

fn working_sprite_lifted_1() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("╷", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

fn working_sprite_lifted_2() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("─", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

fn working_sprite_lifted_3() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("   "),
            Span::styled("▄▄", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("▄", Style::new().fg(PET_BODY)),
            Span::raw("  "),
            Span::styled("█", Style::new().fg(PET_BODY)),
            Span::styled("▀", Style::new().fg(PET_EYE)),
            Span::styled("╶", Style::new().fg(PET_BODY)),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(PET_BODY)),
            Span::raw(" "),
            Span::styled("▀", Style::new().fg(CHEST_COLOR)),
            Span::raw("   "),
            Span::styled("▀▀", Style::new().fg(PET_BODY).bg(CHAIR_COLOR)),
        ]),
    ]
}

const CHAIR_COLOR: Color = MUSHROOM_STEM;

/// Mossy log desk — top row is moss, second row is the log body.
/// `▄▄▄▄` over `████` so the moss reads as a soft layer along the top
/// regardless of the terminal background.
fn desk_sprite() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("▄▄▄▄", Style::new().fg(LOG_MOSS))),
        Line::from(Span::styled("████", Style::new().fg(LOG_BODY))),
    ]
}

/// Mushroom inkwell — red cap on a cream stem, used as the spot
/// Boston sits when working. Renders one row tall like the original
/// chair so all the existing pet_y math stays correct.
fn chair_sprite() -> Vec<Line<'static>> {
    vec![Line::from(vec![
        Span::styled("▟", Style::new().fg(MUSHROOM_CAP)),
        Span::styled("▙", Style::new().fg(MUSHROOM_CAP)),
    ])]
}

const PAPER_COLOR: Color = LEAF_COLOR;

/// Stack of falling leaves above the log — height grows with the
/// running task count. Same control flow as the original paper stack
/// (0/1/2 rows), recolored to greens to fit the forest scene.
fn paper_sprite(running_count: usize) -> Vec<Line<'static>> {
    let height = match running_count {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        _ => MAX_PAPER_HEIGHT as usize,
    };
    (0..height)
        .map(|_| Line::from(Span::styled("▐█▌", Style::new().fg(PAPER_COLOR))))
        .collect()
}

fn idle_motion(state: &AppState) -> IdleMotion {
    if state.pet_idle_wave_enabled && state.pet_bob_timer == state.pet_idle_wave_tick {
        IdleMotion::Wave
    } else if state.pet_bob_timer == state.pet_idle_jump_tick {
        IdleMotion::Jump
    } else if state.pet_bob_timer == state.pet_idle_blink_tick {
        IdleMotion::Blink
    } else {
        IdleMotion::Rest
    }
}

fn working_paper_lift(state: &AppState) -> u16 {
    if state.pet_state == PetState::Working
        && state.pet_working_paper_timer > 0
        && state.pet_working_paper_timer < state.pet_working_paper_lift_until
    {
        1
    } else {
        0
    }
}

fn working_sprite(state: &AppState) -> Vec<Line<'static>> {
    let lifted = working_paper_lift(state) == 1;
    match state.pet_frame {
        1 => {
            if lifted {
                working_sprite_lifted_1()
            } else {
                working_sprite_1()
            }
        }
        2 => {
            if lifted {
                working_sprite_lifted_2()
            } else {
                working_sprite_2()
            }
        }
        3 => {
            if lifted {
                working_sprite_lifted_3()
            } else {
                working_sprite_3()
            }
        }
        _ => {
            if lifted {
                working_sprite_lifted_1()
            } else {
                working_sprite_1()
            }
        }
    }
}

fn recolor_sprite(lines: Vec<Line<'static>>, body: Color, eye: Color) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| {
            let spans = line
                .spans
                .into_iter()
                .map(|mut span| {
                    if span.style.fg == Some(PET_BODY) {
                        span.style = span.style.fg(body);
                    } else if span.style.fg == Some(PET_EYE) {
                        span.style = span.style.fg(eye);
                    }
                    span
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect()
}

fn idle_sprite(motion: IdleMotion) -> Vec<Line<'static>> {
    match motion {
        IdleMotion::Wave => sitting_sprite_wave(),
        IdleMotion::Blink => sitting_sprite_blink(),
        IdleMotion::Jump | IdleMotion::Rest => sitting_sprite(),
    }
}

fn walking_sprite_frame(state: &crate::state::AppState) -> usize {
    match state.pet_frame {
        2 => 2,
        3 => 3,
        _ => 1,
    }
}

fn walking_vertical_lift(state: &crate::state::AppState) -> u16 {
    let is_walking = matches!(state.pet_state, PetState::WalkRight | PetState::WalkLeft);
    if is_walking
        && state.pet_walk_bounce_lift_until > 0
        && state.pet_walk_tick < state.pet_walk_bounce_lift_until
    {
        2
    } else {
        0
    }
}

/// Draw the duo, the mossy log, the mushroom inkwell, and any leaves.
/// `running_count` controls leaf-stack height.
///
/// `bottom_area` is the dedicated band between the pane list and the bottom
/// panel. All sprites render inside it, sharing a baseline at its last row
/// (the row directly above the bottom panel's top border). The band must be
/// tall enough to fit the pet scene — see [`super::PET_SCENE_HEIGHT`].
///
/// Working state example (Boston at the log, Frenchie seated):
/// ```text
///  ▄ ▄        ▄▄  ▐█▌
/// ▄▀▀▀▄       █▀╴ ▐█▌
///  ▀ ▀        ▀▀ ████
///                 ████
/// ```
pub fn draw_pet(frame: &mut Frame, state: &mut AppState, bottom_area: Rect, _running_count: usize) {
    if bottom_area.height == 0 || bottom_area.width == 0 {
        state.pet_image_rect = None;
        return;
    }
    // Hand the cell rectangle to the Kitty-graphics emitter that runs
    // after ratatui flushes its frame. We clear the cells here so the
    // image isn't chewed at the edges by stale glyphs from a prior
    // frame; ratatui's `Clear` widget writes blanks into the buffer
    // for the rectangle, which then get painted over by the actual
    // PNG via `pet_image::emit_pet_frame`.
    state.pet_image_rect = Some((
        bottom_area.x,
        bottom_area.y,
        bottom_area.width,
        bottom_area.height,
    ));
    frame.render_widget(ratatui::widgets::Clear, bottom_area);
}

/// Helper to render a slice of Lines at given position, clipping to frame bounds.
fn render_lines(frame: &mut Frame, lines: &[Line<'_>], x: u16, start_y: u16) {
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= frame.area().height {
            continue;
        }
        let line_width: u16 = line.spans.iter().map(|s| s.content.width() as u16).sum();
        let area = frame.area();
        let right = area.x.saturating_add(area.width);
        let available = right.saturating_sub(x);
        if available == 0 {
            continue;
        }
        let w = line_width.min(available);
        let area = Rect::new(x, y, w, 1);
        frame.render_widget(line.clone(), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ratatui::{Terminal, backend::TestBackend};

    /// Convert a sprite (Vec<Line>) to a plain string for visual inspection.
    fn sprite_to_string(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ── Individual sprite pattern tests ──
    //
    // Each sprite is the paired Frenchie + Boston duo. Frenchie (left)
    // is all dark glyphs; Boston (right) has a chest blaze that reads
    // as a `▀` glyph styled CHEST_COLOR — in the plain-text dump the
    // glyph shows up the same as the surrounding `▀`. The colors
    // distinguish them at render time, the shape stays consistent.

    #[test]
    fn sprite_sitting() {
        let s = sprite_to_string(&sitting_sprite());
        assert_eq!(s, [" ▄ ▄  ▄ ▄ ", "▄▀▀▀▄ ▄▀▀▀▄", " ▀ ▀  ▀ ▀ ",].join("\n"));
    }

    #[test]
    fn sprite_walking_right_frame1() {
        let s = sprite_to_string(&walking_right_1());
        assert_eq!(s, [" ▄ ▄  ▄ ▄ ", "▄▀▀▀▄ ▄▀▀▀▄", "▖ ▗   ▖ ▗ ",].join("\n"));
    }

    #[test]
    fn sprite_walking_right_frame2() {
        let s = sprite_to_string(&walking_right_2());
        assert_eq!(s, [" ▄ ▄  ▄ ▄ ", "▄▀▀▀▄ ▄▀▀▀▄", "▗ ▖   ▗ ▖ ",].join("\n"));
    }

    #[test]
    fn sprite_working_frame1() {
        let s = sprite_to_string(&working_sprite_1());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀╴", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_working_frame2() {
        let s = sprite_to_string(&working_sprite_2());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀─", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_working_frame3() {
        let s = sprite_to_string(&working_sprite_3());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀╶", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_working_lifted_frame1() {
        let s = sprite_to_string(&working_sprite_lifted_1());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀╷", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_working_lifted_frame2() {
        let s = sprite_to_string(&working_sprite_lifted_2());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀─", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_working_lifted_frame3() {
        let s = sprite_to_string(&working_sprite_lifted_3());
        assert_eq!(s, [" ▄ ▄   ▄▄", "▄▀▀▀▄  █▀╶", " ▀ ▀   ▀▀",].join("\n"));
    }

    #[test]
    fn sprite_desk() {
        let s = sprite_to_string(&desk_sprite());
        assert_eq!(s, ["▄▄▄▄", "████",].join("\n"));
    }

    #[test]
    fn sprite_chair() {
        let s = sprite_to_string(&chair_sprite());
        assert_eq!(s, "▟▙");
    }

    #[test]
    fn sprite_paper_0() {
        assert_eq!(sprite_to_string(&paper_sprite(0)), "");
    }

    #[test]
    fn sprite_paper_1() {
        assert_eq!(sprite_to_string(&paper_sprite(1)), "▐█▌");
    }

    #[test]
    fn sprite_paper_2() {
        let s = sprite_to_string(&paper_sprite(2));
        assert_eq!(s, ["▐█▌", "▐█▌",].join("\n"));
    }

    #[test]
    fn all_sprites_have_3_lines() {
        assert_eq!(sitting_sprite().len(), 3);
        assert_eq!(sitting_sprite_blink().len(), 3);
        assert_eq!(sitting_sprite_wave().len(), 3);
        assert_eq!(walking_right_1().len(), 3);
        assert_eq!(walking_right_2().len(), 3);
        assert_eq!(walking_right_3().len(), 3);
        assert_eq!(walking_left_1().len(), 3);
        assert_eq!(walking_left_2().len(), 3);
        assert_eq!(walking_left_3().len(), 3);
        assert_eq!(working_sprite_1().len(), 3);
        assert_eq!(working_sprite_2().len(), 3);
        assert_eq!(working_sprite_3().len(), 3);
        assert_eq!(working_sprite_lifted_1().len(), 3);
        assert_eq!(working_sprite_lifted_2().len(), 3);
        assert_eq!(working_sprite_lifted_3().len(), 3);
    }

    #[test]
    fn desk_sprite_has_lines() {
        let desk = desk_sprite();
        assert!(!desk.is_empty());
    }

    #[test]
    fn paper_sprite_height_scales_with_count() {
        assert_eq!(paper_sprite(0).len(), 0);
        assert_eq!(paper_sprite(1).len(), 1);
        assert_eq!(paper_sprite(3).len(), 2);
        assert_eq!(paper_sprite(5).len(), 2);
    }

    #[test]
    fn sprite_sitting_blink() {
        let s = sprite_to_string(&sitting_sprite_blink());
        assert_eq!(s, [" ▄ ▄  ▄ ▄ ", "▄─▀─▄ ▄─▀─▄", " ▀ ▀  ▀ ▀ ",].join("\n"));
    }

    #[test]
    fn sprite_sitting_wave() {
        let s = sprite_to_string(&sitting_sprite_wave());
        assert_eq!(s, [" ▄ ▄  ▄ ▄▘", "▄▀▀▀▄ ▄▀▀▀▄", " ▀ ▀  ▀ ▀ ",].join("\n"));
    }

    #[test]
    fn idle_sprite_cycles_through_idle_poses() {
        assert_eq!(
            sprite_to_string(&idle_sprite(IdleMotion::Rest)),
            sprite_to_string(&sitting_sprite())
        );
        assert_eq!(
            sprite_to_string(&idle_sprite(IdleMotion::Jump)),
            sprite_to_string(&sitting_sprite())
        );
        assert_eq!(
            sprite_to_string(&idle_sprite(IdleMotion::Blink)),
            sprite_to_string(&sitting_sprite_blink())
        );
        assert_eq!(
            sprite_to_string(&idle_sprite(IdleMotion::Wave)),
            sprite_to_string(&sitting_sprite_wave())
        );
    }

    #[test]
    fn idle_motion_schedule_is_sparse_and_non_overlapping() {
        let state = AppState::new("%0".into());
        assert!(state.pet_idle_jump_tick < BOB_INTERVAL);
        assert!(state.pet_idle_blink_tick < BOB_INTERVAL);
        assert_ne!(state.pet_idle_jump_tick, state.pet_idle_blink_tick);
        assert!(state.pet_idle_jump_tick < state.pet_idle_blink_tick);
        assert!(state.pet_idle_wave_tick < BOB_INTERVAL);
        if state.pet_idle_wave_enabled {
            assert!((16..=19).contains(&state.pet_idle_wave_tick));
        } else {
            assert_eq!(state.pet_idle_wave_tick, 0);
        }
    }

    #[test]
    fn walking_vertical_lift_triggers_inside_scheduled_window() {
        let mut state = AppState::new("%0".into());
        state.pet_state = PetState::WalkRight;
        state.pet_walk_tick = 4;
        state.pet_walk_bounce_lift_until = 6;
        assert_eq!(walking_vertical_lift(&state), 2);
    }

    #[test]
    fn walking_vertical_lift_skips_outside_scheduled_window() {
        let mut state = AppState::new("%0".into());
        state.pet_state = PetState::WalkRight;
        state.pet_walk_tick = 6;
        state.pet_walk_bounce_lift_until = 6;
        assert_eq!(walking_vertical_lift(&state), 0);
    }

    #[test]
    fn walking_vertical_lift_skips_when_not_scheduled() {
        let mut state = AppState::new("%0".into());
        state.pet_state = PetState::WalkRight;
        state.pet_walk_tick = 5;
        state.pet_walk_bounce_lift_until = 0;
        assert_eq!(walking_vertical_lift(&state), 0);
    }

    // Block-art snapshot tests removed when the Kitty-graphics image
    // path took over `draw_pet`. The sprite helpers above still have
    // unit tests for their glyph layouts so the on-shelf fallback
    // doesn't silently rot.
}
