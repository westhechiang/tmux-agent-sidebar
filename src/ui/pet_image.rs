//! Kitty-graphics-protocol pet rendering.
//!
//! The block-art pet in [`crate::ui::pet`] renders into ratatui cells;
//! that ceiling is ~30 cells × 8 rows of unicode block glyphs in 256
//! colors. To get a recognizable Boston-terrier + French-bulldog duo
//! scene we bypass ratatui for the pet band and draw real PNG frames
//! using the Kitty graphics protocol, which Ghostty implements natively
//! and which tmux forwards via `allow-passthrough on`.
//!
//! Each call to [`emit_pet_frame`] selects the animation frame for the
//! current [`PetState`] / `pet_frame`, transmits the PNG, and tells the
//! terminal to display it inside a `cols × rows` cell box at `(x, y)`.
//! We re-transmit on every render so we never have to track which
//! frame Ghostty currently has cached — it's cheap (~60 KB per frame,
//! base64-encoded) and stateless, which avoids whole categories of
//! desync bugs across detach/attach and screen redraws.
//!
//! Inside tmux the escape needs the passthrough wrapper:
//! `ESC P tmux ; <body-with-ESC-doubled> ESC \`. We detect tmux via the
//! `TMUX` env var and wrap automatically.

use std::io::{self, Write};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use crate::state::AppState;
use crate::ui::pet::PetState;

const IDLE_PNG: &[u8] = include_bytes!("../../assets/pet/idle_small.png");
const WALK_A_PNG: &[u8] = include_bytes!("../../assets/pet/walk_a_small.png");
const WALK_B_PNG: &[u8] = include_bytes!("../../assets/pet/walk_b_small.png");
const WORK_PNG: &[u8] = include_bytes!("../../assets/pet/work_small.png");

/// Pick the PNG frame for the current pet state. Walking left and
/// walking right share the same source frames — Ghostty doesn't have a
/// flip-X knob in the graphics protocol, so we accept the asymmetry
/// for v1 and (if it bugs us) pre-render flipped variants later.
fn select_frame(state: &AppState) -> &'static [u8] {
    match state.pet_state {
        PetState::Idle => IDLE_PNG,
        PetState::Working => WORK_PNG,
        PetState::WalkRight | PetState::WalkLeft => match state.pet_frame {
            // Frame 0 means "just entered this state, no walk-cycle
            // progression yet" — fall through to walk_a so we never
            // emit an empty frame between Idle → WalkRight transitions.
            2 => WALK_B_PNG,
            _ => WALK_A_PNG,
        },
    }
}

/// True when the process is running inside a tmux client. Used to
/// decide whether to wrap kitty-graphics escapes in the passthrough
/// envelope. Checking `TMUX` is sufficient — every tmux client sets it
/// and nothing else does.
fn inside_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
}

/// Build the kitty-graphics escape that transmits + displays `png` in a
/// `cols × rows` cell box at the current cursor position.
///
/// `f=100` selects PNG, `a=T` means "transmit and display now" (so we
/// don't have to track image IDs), `c=`/`r=` constrain the display
/// box, `q=2` suppresses the response payload Ghostty would otherwise
/// echo back into our stdin.
fn build_kitty_seq(png: &[u8], cols: u16, rows: u16) -> Vec<u8> {
    let payload = BASE64.encode(png);
    let body = format!("\x1b_Gf=100,a=T,c={cols},r={rows},q=2;{payload}\x1b\\");
    if inside_tmux() {
        // tmux passthrough: outer DCS wraps the body, every ESC inside
        // the body is doubled, body terminates with ESC \.
        let mut wrapped = String::with_capacity(body.len() + 16);
        wrapped.push_str("\x1bPtmux;");
        for ch in body.chars() {
            if ch == '\x1b' {
                wrapped.push_str("\x1b\x1b");
            } else {
                wrapped.push(ch);
            }
        }
        wrapped.push_str("\x1b\\");
        wrapped.into_bytes()
    } else {
        body.into_bytes()
    }
}

/// Position the cursor at `(x, y)` and emit the kitty-graphics escape
/// for the current pet frame. Call this *after* the ratatui frame has
/// been flushed — otherwise ratatui's cell diff will overwrite the
/// image with whatever it thinks should be in the pet band.
///
/// `cols` × `rows` should match the pet band's cell rectangle; Ghostty
/// scales the PNG into that box.
pub fn emit_pet_frame<W: Write>(
    out: &mut W,
    state: &AppState,
    x: u16,
    y: u16,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    if cols == 0 || rows == 0 {
        return Ok(());
    }
    let png = select_frame(state);
    // crossterm `MoveTo` would do the same thing, but the import surface
    // here is so small that a hand-written CUP saves a dependency edge.
    write!(out, "\x1b[{};{}H", y + 1, x + 1)?;
    out.write_all(&build_kitty_seq(png, cols, rows))?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_frames_are_nonempty_pngs() {
        for (name, png) in [
            ("idle", IDLE_PNG),
            ("walk_a", WALK_A_PNG),
            ("walk_b", WALK_B_PNG),
            ("work", WORK_PNG),
        ] {
            assert!(png.len() > 1024, "{name} png suspiciously small");
            assert_eq!(
                &png[..8],
                &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'],
                "{name} not a PNG"
            );
        }
    }

    #[test]
    fn build_kitty_seq_outside_tmux_starts_with_apc_g() {
        // Ensure no TMUX env in this test scope.
        let prev = std::env::var_os("TMUX");
        // SAFETY: tests are single-threaded for this crate by default
        // (no multi-thread tests touch TMUX).
        unsafe { std::env::remove_var("TMUX") };
        let seq = build_kitty_seq(b"\x89PNGfake", 12, 8);
        if let Some(p) = prev {
            unsafe { std::env::set_var("TMUX", p) };
        }
        let s = String::from_utf8(seq).unwrap();
        assert!(s.starts_with("\x1b_Gf=100"));
        assert!(s.ends_with("\x1b\\"));
    }

    #[test]
    fn build_kitty_seq_inside_tmux_wraps_with_passthrough() {
        let prev = std::env::var_os("TMUX");
        unsafe { std::env::set_var("TMUX", "/tmp/tmux-x/default,123,4") };
        let seq = build_kitty_seq(b"\x89PNGfake", 12, 8);
        match prev {
            Some(p) => unsafe { std::env::set_var("TMUX", p) },
            None => unsafe { std::env::remove_var("TMUX") },
        }
        let s = String::from_utf8(seq).unwrap();
        assert!(s.starts_with("\x1bPtmux;"));
        // Inner kitty escape's ESC must be doubled inside the wrapper.
        assert!(s.contains("\x1b\x1b_Gf=100"));
        assert!(s.ends_with("\x1b\\"));
    }
}
