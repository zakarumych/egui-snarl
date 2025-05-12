use egui_scale::EguiScale;

use super::{BackgroundPattern, PinPlacement, SelectionStyle, SnarlStyle, WireStyle};

impl EguiScale for WireStyle {
    #[inline(always)]
    fn scale(&mut self, scale: f32) {
        match self {
            WireStyle::Line | WireStyle::Bezier3 | WireStyle::Bezier5 => {}
            WireStyle::AxisAligned { corner_radius } => {
                corner_radius.scale(scale);
            }
        }
    }
}

impl EguiScale for SelectionStyle {
    #[inline(always)]
    fn scale(&mut self, scale: f32) {
        self.margin.scale(scale);
        self.rounding.scale(scale);
        self.stroke.scale(scale);
    }
}

impl EguiScale for PinPlacement {
    fn scale(&mut self, scale: f32) {
        if let PinPlacement::Outside { margin } = self {
            margin.scale(scale);
        }
    }
}

impl EguiScale for BackgroundPattern {
    fn scale(&mut self, scale: f32) {
        if let BackgroundPattern::Grid(grid) = self {
            grid.spacing.scale(scale);
        }
    }
}

impl EguiScale for SnarlStyle {
    fn scale(&mut self, scale: f32) {
        self.node_frame.scale(scale);
        self.header_frame.scale(scale);
        self.header_drag_space.scale(scale);
        self.pin_size.scale(scale);
        self.pin_stroke.scale(scale);
        self.pin_placement.scale(scale);
        self.wire_width.scale(scale);
        self.wire_frame_size.scale(scale);
        self.wire_style.scale(scale);
        self.bg_frame.scale(scale);
        self.bg_pattern.scale(scale);
        self.bg_pattern_stroke.scale(scale);
        self.min_scale.scale(scale);
        self.max_scale.scale(scale);
        self.select_stoke.scale(scale);
        self.select_style.scale(scale);
    }
}
