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

// ── Paired duo sprites: 5-row design ────────────────────────────────
//
// Each pup is 5 cols × 5 rows. The paired sprite is 11 cols (Frenchie
// left + 1-col gap + Boston right) × 5 rows. Quadrant glyphs
// (▘▝▖▗▙▟) carve the silhouette; `▀`/`█` fill the body; eye pupils
// use FG=PET_EYE on BG=PET_BODY so the white quadrant reads on a
// black face. Boston gets a 3-col chest blaze on row 3 to read as a
// tuxedo coat — the one mark distinguishing her from the all-black
// Frenchie at this scale.
//
// Row breakdown (per pup):
//   row 0  ears        ▙ . ▟
//   row 1  forehead    ▙█████▟
//   row 2  eyes        █▘█▝█  (▘▝ = white pupils on body bg)
//   row 3  chin/chest  ▝███▘  (Frenchie) or █WWW█ (Boston tuxedo)
//   row 4  legs        ▖█.█▖

/// Build a span styled with `fg=PET_BODY`, no bg. Recolored at draw
/// time via `recolor_sprite` → theme.pet_body.
fn body(g: &'static str) -> Span<'static> {
    Span::styled(g, Style::new().fg(PET_BODY))
}

/// Eye pupil — white quadrant glyph painted onto a body-colored cell
/// background, so the pupil reads on the dog's face whatever the
/// terminal background is. Recolored at draw time via PET_EYE.
fn eye(g: &'static str) -> Span<'static> {
    Span::styled(g, Style::new().fg(PET_EYE).bg(PET_BODY))
}

/// Boston's tuxedo chest blaze — full block in CHEST_COLOR. Hardcoded
/// rather than themed: the white blaze is the breed-defining mark.
fn chest(g: &'static str) -> Span<'static> {
    Span::styled(g, Style::new().fg(CHEST_COLOR))
}

fn nil() -> Span<'static> {
    Span::raw(" ")
}

/// Combine a frenchie row (5 spans) + 1-col gap + boston row (5 spans)
/// into a single 11-col Line. The two pups share each row's vertical
/// position so they read as standing side by side.
fn pair_row(left: Vec<Span<'static>>, right: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = left;
    spans.push(nil());
    spans.extend(right);
    Line::from(spans)
}

// ── Frenchie poses (5 cols per row) ──

fn fr_ears() -> Vec<Span<'static>> {
    vec![nil(), body("▙"), nil(), body("▟"), nil()]
}
fn fr_head() -> Vec<Span<'static>> {
    vec![body("▙"), body("█"), body("█"), body("█"), body("▟")]
}
fn fr_eyes_open() -> Vec<Span<'static>> {
    vec![body("█"), eye("▘"), body("█"), eye("▝"), body("█")]
}
fn fr_eyes_blink() -> Vec<Span<'static>> {
    vec![body("█"), eye("▖"), body("█"), eye("▗"), body("█")]
}
fn fr_chin() -> Vec<Span<'static>> {
    vec![body("▝"), body("█"), body("█"), body("█"), body("▘")]
}
fn fr_legs_idle() -> Vec<Span<'static>> {
    vec![body("▖"), body("█"), nil(), body("█"), body("▖")]
}
fn fr_legs_walk_a() -> Vec<Span<'static>> {
    vec![body("▘"), body("█"), nil(), nil(), body("▖")]
}
fn fr_legs_walk_b() -> Vec<Span<'static>> {
    vec![body("▖"), nil(), nil(), body("█"), body("▘")]
}

// ── Boston poses (mirrors Frenchie + tuxedo blaze on row 3) ──

fn bo_ears() -> Vec<Span<'static>> {
    vec![nil(), body("▙"), nil(), body("▟"), nil()]
}
fn bo_head() -> Vec<Span<'static>> {
    vec![body("▙"), body("█"), body("█"), body("█"), body("▟")]
}
fn bo_eyes_open() -> Vec<Span<'static>> {
    vec![body("█"), eye("▘"), body("█"), eye("▝"), body("█")]
}
fn bo_eyes_blink() -> Vec<Span<'static>> {
    vec![body("█"), eye("▖"), body("█"), eye("▗"), body("█")]
}
fn bo_chest() -> Vec<Span<'static>> {
    // Black sides + 3-col white chest blaze.
    vec![body("█"), chest("█"), chest("█"), chest("█"), body("█")]
}
fn bo_legs_idle() -> Vec<Span<'static>> {
    vec![body("▖"), body("█"), nil(), body("█"), body("▖")]
}
fn bo_legs_walk_a() -> Vec<Span<'static>> {
    vec![body("▘"), body("█"), nil(), nil(), body("▖")]
}
fn bo_legs_walk_b() -> Vec<Span<'static>> {
    vec![body("▖"), nil(), nil(), body("█"), body("▘")]
}
fn bo_ears_wave() -> Vec<Span<'static>> {
    // bo_ears + a raised paw glyph hanging off Boston's right ear
    // column. Stays inside the 5-col pup footprint so Frenchie isn't
    // clipped on the left when the wave fires.
    vec![nil(), body("▙"), nil(), body("▟"), body("▘")]
}

// ── Composed sprites (sitting / blink / wave / walk / work) ──
//
// `working_*` reuse `sitting_sprite` because the seated duo doesn't
// change pose at the desk in the 5-row design — the chair (mushroom
// inkwell) renders separately under Boston's column, the leaf-stack
// shake animation lives in `working_paper_lift` not the sprite. The
// frame-1/2/3 + lifted variants exist only so the dispatcher in
// `working_sprite` keeps the same shape for future per-frame poses.

fn sitting_sprite() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_idle(), bo_legs_idle()),
    ]
}

fn sitting_sprite_blink() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_blink(), bo_eyes_blink()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_idle(), bo_legs_idle()),
    ]
}

fn sitting_sprite_wave() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears_wave()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_idle(), bo_legs_idle()),
    ]
}

fn walking_right_1() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_walk_a(), bo_legs_walk_a()),
    ]
}

fn walking_right_2() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_walk_b(), bo_legs_walk_b()),
    ]
}

fn walking_right_3() -> Vec<Line<'static>> {
    walking_right_1()
}

fn walking_left_1() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_walk_b(), bo_legs_walk_b()),
    ]
}

fn walking_left_2() -> Vec<Line<'static>> {
    vec![
        pair_row(fr_ears(), bo_ears()),
        pair_row(fr_head(), bo_head()),
        pair_row(fr_eyes_open(), bo_eyes_open()),
        pair_row(fr_chin(), bo_chest()),
        pair_row(fr_legs_walk_a(), bo_legs_walk_a()),
    ]
}

fn walking_left_3() -> Vec<Line<'static>> {
    walking_left_1()
}

fn working_sprite_1() -> Vec<Line<'static>> {
    sitting_sprite()
}
fn working_sprite_2() -> Vec<Line<'static>> {
    sitting_sprite()
}
fn working_sprite_3() -> Vec<Line<'static>> {
    sitting_sprite()
}
fn working_sprite_lifted_1() -> Vec<Line<'static>> {
    sitting_sprite()
}
fn working_sprite_lifted_2() -> Vec<Line<'static>> {
    sitting_sprite()
}
fn working_sprite_lifted_3() -> Vec<Line<'static>> {
    sitting_sprite()
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
pub fn draw_pet(frame: &mut Frame, state: &AppState, bottom_area: Rect, running_count: usize) {
    if bottom_area.height == 0 || bottom_area.width == 0 {
        return;
    }
    let panel_width = bottom_area.width;
    // Baseline: the bottom-most row for all elements, inside the drawable area.
    let baseline = bottom_area.y + bottom_area.height - 1;

    // --- Positions ---
    let desk_x = bottom_area.x + panel_width.saturating_sub(DESK_OFFSET + DESK_WIDTH + 1);
    let chair_x = desk_x.saturating_sub(CHAIR_WIDTH + CHAIR_DESK_GAP);

    // --- Draw pet first (so desk/chair render on top if overlapping) ---
    let sprite_lines = match state.pet_state {
        PetState::Idle => idle_sprite(idle_motion(state)),
        PetState::WalkRight => match walking_sprite_frame(state) {
            1 => walking_right_1(),
            2 => walking_right_2(),
            3 => walking_right_3(),
            _ => walking_right_1(),
        },
        PetState::Working => working_sprite(state),
        PetState::WalkLeft => match walking_sprite_frame(state) {
            1 => walking_left_1(),
            2 => walking_left_2(),
            3 => walking_left_3(),
            _ => walking_left_1(),
        },
    };
    let sprite_lines = recolor_sprite(sprite_lines, state.theme.pet_body, state.theme.pet_eye);

    let sprite_height = sprite_lines.len() as u16;
    let pet_y = match state.pet_state {
        PetState::Working => {
            // Pet sits on top of chair: 1 row above baseline
            baseline.saturating_sub(sprite_height)
        }
        PetState::Idle if matches!(idle_motion(state), IdleMotion::Jump) => {
            baseline.saturating_sub(sprite_height)
        }
        PetState::Idle => baseline.saturating_sub(sprite_height - 1),
        PetState::WalkRight | PetState::WalkLeft => {
            baseline.saturating_sub(sprite_height - 1 + walking_vertical_lift(state))
        }
    };
    let pet_x = bottom_area.x + state.pet_x;
    render_lines(frame, &sprite_lines, pet_x, pet_y);

    // --- Draw chair (always visible) ---
    let chair_lines = chair_sprite();
    let chair_height = chair_lines.len() as u16;
    let chair_y = baseline.saturating_sub(chair_height - 1);
    render_lines(frame, &chair_lines, chair_x, chair_y);

    // --- Draw desk (legs on baseline, top plate one row above) ---
    let desk_lines = desk_sprite();
    let desk_height = desk_lines.len() as u16;
    let desk_y = baseline.saturating_sub(desk_height - 1);
    render_lines(frame, &desk_lines, desk_x, desk_y);

    // --- Draw papers above desk ---
    if running_count > 0 {
        let papers = paper_sprite(running_count);
        if !papers.is_empty() {
            let paper_y = desk_y.saturating_sub(papers.len() as u16 + working_paper_lift(state));
            let paper_x = desk_x
                + 1
                + if working_paper_lift(state) == 1 {
                    state.pet_working_paper_x_offset
                } else {
                    0
                };
            render_lines(frame, &papers, paper_x, paper_y);
        }
    }
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

    // Per-pose glyph-pattern tests removed when the sprite migrated
    // from the original 3-row half-block design to the 5-row
    // quadrant-glyph design. The tests pinned exact glyph strings,
    // which become churn rather than signal as the pixel art
    // iterates. Structural tests (line count, prop existence) below
    // still guard the layout invariants `draw_pet` depends on.

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
    fn all_sprites_have_5_lines() {
        // `draw_pet` computes pet_y as `baseline - sprite_height`, so
        // every sprite has to be the same height for the layout to
        // line up across state transitions. 5 rows matches
        // `PET_SCENE_HEIGHT = 6` (one row of margin above the band).
        assert_eq!(sitting_sprite().len(), 5);
        assert_eq!(sitting_sprite_blink().len(), 5);
        assert_eq!(sitting_sprite_wave().len(), 5);
        assert_eq!(walking_right_1().len(), 5);
        assert_eq!(walking_right_2().len(), 5);
        assert_eq!(walking_right_3().len(), 5);
        assert_eq!(walking_left_1().len(), 5);
        assert_eq!(walking_left_2().len(), 5);
        assert_eq!(walking_left_3().len(), 5);
        assert_eq!(working_sprite_1().len(), 5);
        assert_eq!(working_sprite_2().len(), 5);
        assert_eq!(working_sprite_3().len(), 5);
        assert_eq!(working_sprite_lifted_1().len(), 5);
        assert_eq!(working_sprite_lifted_2().len(), 5);
        assert_eq!(working_sprite_lifted_3().len(), 5);
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
    fn idle_sprite_cycles_through_idle_poses() {
        // The dispatcher in `idle_sprite` must route Rest/Jump → the
        // base sitting pose, Blink → the blink variant, Wave → the
        // wave variant. We compare via Vec<Line> identity (same
        // helpers on both sides) rather than via stringified glyphs,
        // since the latter churned every time we tweaked the art.
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

    /// Helper: render draw_pet into a buffer and return as string for visual inspection.
    fn render_pet_scene(state: &AppState, running_count: usize, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let bottom_y = height.saturating_sub(10);
        terminal
            .draw(|frame| {
                let bottom_area = Rect::new(0, bottom_y, width, 10);
                draw_pet(frame, state, bottom_area, running_count);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let area = buf.area;
        let mut lines = Vec::new();
        for y in area.y..area.y + area.height {
            let mut line = String::new();
            for x in area.x..area.x + area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        while lines.first().is_some_and(|l| l.is_empty()) {
            lines.remove(0);
        }
        // Remove trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    // Per-frame snapshot tests removed when the sprite design moved
    // to 5 rows + quadrant glyphs. The structural assertions
    // (`all_sprites_have_5_lines`, `idle_sprite_cycles_*`) cover the
    // invariants `draw_pet` actually relies on; pinning exact glyph
    // strings just slowed iteration on the pixel art.
}
