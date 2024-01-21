use std::fmt;

use egui::{emath::Rot2, vec2, Rect, Stroke, Ui, Vec2};

use super::{state::SnarlState, SnarlStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
///Grid background pattern.
///Use `SnarlStyle::background_pattern_stroke` for change stroke options
pub struct Grid {
    pub spacing: Vec2,
    pub angle: f32,
}

const DEFAULT_GRID_SPACING: Vec2 = vec2(5.0, 5.0);
const DEFAULT_GRID_ANGLE: f32 = 1.0;

impl Default for Grid {
    fn default() -> Self {
        Self {
            spacing: DEFAULT_GRID_SPACING,
            angle: DEFAULT_GRID_ANGLE,
        }
    }
}

impl Grid {
    pub const fn new(spacing: Vec2, angle: f32) -> Self {
        Self { spacing, angle }
    }

    fn draw(&self, style: &SnarlStyle, snarl_state: &SnarlState, viewport: &Rect, ui: &mut Ui) {
        let bg_stroke = style
            .background_pattern_stroke
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_stroke);

        let stroke = Stroke::new(
            bg_stroke.width * snarl_state.scale().max(1.0),
            bg_stroke.color.gamma_multiply(snarl_state.scale().min(1.0)),
        );

        let spacing = ui.spacing().icon_width * self.spacing;

        let rot = Rot2::from_angle(self.angle);
        let rot_inv = rot.inverse();

        let graph_viewport = Rect::from_min_max(
            snarl_state.screen_pos_to_graph(viewport.min, *viewport),
            snarl_state.screen_pos_to_graph(viewport.max, *viewport),
        );

        let pattern_bounds = graph_viewport.rotate_bb(rot_inv);

        let min_x = (pattern_bounds.min.x / spacing.x).ceil();
        let max_x = (pattern_bounds.max.x / spacing.x).floor();

        for x in 0..=(max_x - min_x) as i64 {
            let x = (x as f32 + min_x) * spacing.x;

            let top = (rot * vec2(x, pattern_bounds.min.y)).to_pos2();
            let bottom = (rot * vec2(x, pattern_bounds.max.y)).to_pos2();

            let top = snarl_state.graph_pos_to_screen(top, *viewport);
            let bottom = snarl_state.graph_pos_to_screen(bottom, *viewport);

            ui.painter().line_segment([top, bottom], stroke);
        }

        let min_y = (pattern_bounds.min.y / spacing.y).ceil();
        let max_y = (pattern_bounds.max.y / spacing.y).floor();

        for y in 0..=(max_y - min_y) as i64 {
            let y = (y as f32 + min_y) * spacing.y;

            let top = (rot * vec2(pattern_bounds.min.x, y)).to_pos2();
            let bottom = (rot * vec2(pattern_bounds.max.x, y)).to_pos2();

            let top = snarl_state.graph_pos_to_screen(top, *viewport);
            let bottom = snarl_state.graph_pos_to_screen(bottom, *viewport);

            ui.painter().line_segment([top, bottom], stroke);
        }
    }
}

tiny_fn::tiny_fn! {
    pub struct CustomBackground = Fn(style: &SnarlStyle, state: &SnarlState, rect: &Rect, ui: &mut Ui);
}

/// Background pattern show beneath nodes and wires.
pub enum BackgroundPattern {
    NoPattern,
    /// Linear grid.
    Grid(Grid),

    /// Custom pattern
    Custom(CustomBackground<'static>),
}

impl PartialEq for BackgroundPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BackgroundPattern::Grid(l), BackgroundPattern::Grid(r)) => *l == *r,
            _ => false,
        }
    }
}

impl fmt::Debug for BackgroundPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackgroundPattern::Grid(grid) => f
                .debug_tuple("BackgroundPattern::Grid")
                .field(grid)
                .finish(),
            BackgroundPattern::Custom(_) => f.write_str("BackgroundPattern::Custom"),
            BackgroundPattern::NoPattern => f.write_str("BackgroundPattern::NoPattern"),
        }
    }
}

impl Default for BackgroundPattern {
    fn default() -> Self {
        Self::Grid(Default::default())
    }
}

impl BackgroundPattern {
    pub const fn new() -> Self {
        Self::Grid(Grid::new(DEFAULT_GRID_SPACING, DEFAULT_GRID_ANGLE))
    }

    pub fn draw(&self, style: &SnarlStyle, snarl_state: &SnarlState, viewport: &Rect, ui: &mut Ui) {
        match self {
            BackgroundPattern::Grid(g) => g.draw(style, snarl_state, viewport, ui),
            BackgroundPattern::Custom(c) => c.call(style, snarl_state, viewport, ui),
            BackgroundPattern::NoPattern => {}
        }
    }
}
