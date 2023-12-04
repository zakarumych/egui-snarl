use egui::{style::Spacing, vec2, Context, Frame, Id, Pos2, Rect, Vec2};

/// Node UI state.
#[derive(Clone, Copy, PartialEq)]
pub struct NodeState {
    /// Size occupied by title.
    pub title_size: Vec2,

    /// Size occupied by inputs.
    pub inputs_size: Vec2,

    /// Size occupied by outputs.
    pub outputs_size: Vec2,
}

impl NodeState {
    pub fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(&self, cx: &Context, id: Id) {
        cx.data_mut(|d| d.insert_temp(id, *self));
    }

    /// Finds node rect at specific position (excluding node frame margin).
    pub fn node_rect(&self, frame: &Frame, spacing: &Spacing, pos: Pos2) -> Rect {
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let height = self.title_size.y
            + frame.total_margin().bottom
            + frame.total_margin().bottom
            + self.inputs_size.y.max(self.outputs_size.y);

        Rect::from_min_size(pos, vec2(width, height))
    }

    /// Finds title rect at specific position (excluding node frame margin).
    pub fn title_rect(&self, spacing: &Spacing, pos: Pos2) -> Rect {
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let height = self.title_size.y;

        Rect::from_min_size(pos, vec2(width, height))
    }

    /// Finds pins rect at specific position (excluding node frame margin).
    pub fn pins_rect(&self, frame: &Frame, spacing: &Spacing, openness: f32, pos: Pos2) -> Rect {
        let height = self.inputs_size.y.max(self.outputs_size.y);
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let moved =
            (height + frame.total_margin().bottom + frame.total_margin().bottom) * (openness - 1.0);

        let pos = pos
            + vec2(
                0.0,
                self.title_size.y
                    + frame.total_margin().bottom
                    + frame.total_margin().bottom
                    + moved,
            );

        Rect::from_min_size(pos, vec2(width, height))
    }

    pub fn initial(spacing: &Spacing) -> Self {
        NodeState {
            title_size: spacing.interact_size,
            inputs_size: spacing.interact_size,
            outputs_size: spacing.interact_size,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SnarlState {
    /// Where viewport's left-top in graph's space.
    offset: Vec2,

    /// Scale of the viewport.
    scale: f32,

    target_scale: f32,
}

impl Default for SnarlState {
    fn default() -> Self {
        SnarlState {
            offset: Vec2::ZERO,
            scale: 1.0,
            target_scale: 1.0,
        }
    }
}

impl SnarlState {
    #[inline(always)]
    pub fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data_mut(|d| d.get_temp(id))
    }

    #[inline(always)]
    pub fn store(&self, cx: &Context, id: Id) {
        cx.data_mut(|d| d.insert_temp(id, *self));
    }

    #[inline(always)]
    pub fn animate(&mut self, id: Id, cx: &Context, pivot: Pos2, viewport: Rect) {
        let new_scale = cx.animate_value_with_time(id.with("zoom-scale"), self.target_scale, 0.1);

        let a = pivot + self.offset - viewport.center().to_vec2();

        self.offset += a * new_scale / self.scale - a;
        self.scale = new_scale;
    }

    #[inline(always)]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    #[inline(always)]
    pub fn set_scale(&mut self, scale: f32) {
        self.target_scale = scale;
    }

    #[inline(always)]
    pub fn screen_pos_to_graph(&self, pos: Pos2, viewport: Rect) -> Pos2 {
        (pos + self.offset - viewport.center().to_vec2()) / self.scale
    }

    #[inline(always)]
    pub fn graph_pos_to_screen(&self, pos: Pos2, viewport: Rect) -> Pos2 {
        pos * self.scale - self.offset + viewport.center().to_vec2()
    }

    #[inline(always)]
    pub fn graph_vec_to_screen(&self, size: Vec2) -> Vec2 {
        size * self.scale
    }

    #[inline(always)]
    pub fn screen_vec_to_graph(&self, size: Vec2) -> Vec2 {
        size / self.scale
    }

    #[inline(always)]
    pub fn graph_value_to_screen(&self, value: f32) -> f32 {
        value * self.scale
    }

    #[inline(always)]
    pub fn screen_value_to_graph(&self, value: f32) -> f32 {
        value / self.scale
    }
}
