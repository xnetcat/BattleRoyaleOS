//! Main menu screen

use crate::game::state::{GameState, MainMenuOption, MenuAction};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::button::{Button, ButtonList};
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::draw_gradient_background_raw;

/// Main menu screen state
pub struct MainMenuScreen {
    pub buttons: ButtonList,
    pub fb_width: usize,
    pub fb_height: usize,
}

impl MainMenuScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        let button_width = 300;
        let button_height = 60;
        let button_spacing = 20;
        let start_y = fb_height / 2 - 100;

        let buttons = [
            Button::centered(start_y, button_width, button_height, MainMenuOption::Play.label(), fb_width),
            Button::centered(start_y + button_height + button_spacing, button_width, button_height, MainMenuOption::Settings.label(), fb_width),
            Button::centered(start_y + (button_height + button_spacing) * 2, button_width, button_height, MainMenuOption::Customization.label(), fb_width),
            Button::centered(start_y + (button_height + button_spacing) * 3, button_width, button_height, MainMenuOption::Quit.label(), fb_width),
        ];

        Self {
            buttons: ButtonList::new(buttons, MainMenuOption::COUNT),
            fb_width,
            fb_height,
        }
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Up => self.buttons.select_prev(),
            MenuAction::Down => self.buttons.select_next(),
            MenuAction::Select => {
                match MainMenuOption::from_index(self.buttons.selected_index) {
                    MainMenuOption::Play => return Some(GameState::ServerSelect),
                    MainMenuOption::Settings => return Some(GameState::Settings),
                    MainMenuOption::Customization => return Some(GameState::Customization),
                    MainMenuOption::Quit => {
                        // Signal quit (handled in main loop)
                        return None;
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Draw the main menu
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Draw gradient background
        draw_gradient_background_raw(fb, fb_width, fb_height);

        // Draw title
        let title = "BATTLE ROYALE";
        let title_scale = 5;
        let title_y = 80;
        font::draw_string_centered_raw(fb, title_y, title, colors::TITLE, title_scale);

        // Draw subtitle
        let subtitle = "100 PLAYERS. ONE WINNER.";
        let subtitle_scale = 2;
        let subtitle_y = title_y + font::char_height(title_scale) + 20;
        font::draw_string_centered_raw(fb, subtitle_y, subtitle, colors::SUBTITLE, subtitle_scale);

        // Draw buttons
        self.buttons.draw(fb);

        // Draw footer
        let footer = "PRESS ENTER TO SELECT";
        let footer_y = fb_height - 50;
        font::draw_string_centered_raw(fb, footer_y, footer, colors::SUBTITLE, 2);

        // Draw version
        let version = "V0.1.0";
        font::draw_string_raw(fb, 10, fb_height - 30, version, colors::SUBTITLE, 1);
    }

    /// Check if quit was selected
    pub fn quit_selected(&self) -> bool {
        self.buttons.selected_index == MainMenuOption::Quit.to_index()
    }
}
