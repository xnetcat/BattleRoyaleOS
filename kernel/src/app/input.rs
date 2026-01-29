//! Input Handling
//!
//! Handles keyboard and mouse input for game controls.

use crate::game::input::KeyState;
use crate::game::state::MenuAction;

/// Get menu action from key state (edge-triggered)
pub fn get_menu_action(current: &KeyState, prev: &KeyState) -> MenuAction {
    // Edge detection - only trigger on key press, not hold
    if current.w && !prev.w || current.up && !prev.up {
        return MenuAction::Up;
    }
    if current.s && !prev.s || current.down && !prev.down {
        return MenuAction::Down;
    }
    if current.a && !prev.a || current.left && !prev.left {
        return MenuAction::Left;
    }
    if current.d && !prev.d || current.right && !prev.right {
        return MenuAction::Right;
    }
    if current.enter && !prev.enter || current.space && !prev.space {
        return MenuAction::Select;
    }
    if current.escape && !prev.escape {
        return MenuAction::Back;
    }
    MenuAction::None
}
