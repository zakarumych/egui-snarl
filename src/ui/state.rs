use egui::{style::Spacing, vec2, Context, Id, Pos2, Rect, Vec2};

use crate::Snarl;

use super::SnarlStyle;

/// Node UI state.
#[derive(Clone, Copy, PartialEq)]
pub struct NodeState {
    /// Node size for this frame.
    /// It is updated to fit content.
    pub size: Vec2,
}

impl NodeState {
    pub fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(&self, cx: &Context, id: Id) {
        cx.data_mut(|d| d.insert_temp(id, *self));
    }

    /// Finds node rect at specific position (excluding node frame margin).
    pub fn node_rect(&self, pos: Pos2) -> Rect {
        Rect::from_min_size(pos, self.size)
    }

    pub fn initial(spacing: &Spacing) -> Self {
        NodeState {
            // title_size: spacing.interact_size,
            // inputs_size: spacing.interact_size,
            // outputs_size: spacing.interact_size,
            size: spacing.interact_size * 3.0,
        }
    }
}

#[derive(PartialEq)]
pub struct SnarlState {
    /// Where viewport's center in graph's space.
    offset: Vec2,

    /// Scale of the viewport.
    scale: f32,

    target_scale: f32,

    /// Flag indicating that the graph state is dirty must be saved.
    dirty: bool,
}

#[derive(Clone, Copy)]
struct SnarlStateData {
    offset: Vec2,
    scale: f32,
    target_scale: f32,
}

impl SnarlState {
    #[inline(always)]
    pub fn load<T>(
        cx: &Context,
        id: Id,
        pivot: Pos2,
        viewport: Rect,
        snarl: &Snarl<T>,
        style: &SnarlStyle,
    ) -> Self {
        let Some(SnarlStateData {
            mut offset,
            mut scale,
            target_scale,
        }) = cx.data_mut(|d| d.get_temp(id))
        else {
            return Self::initial(viewport, snarl, style);
        };

        let new_scale = cx.animate_value_with_time(id.with("zoom-scale"), target_scale, 0.1);

        let mut dirty = false;
        if new_scale != scale {
            let a = pivot + offset - viewport.center().to_vec2();

            offset += a * new_scale / scale - a;
            scale = new_scale;
            dirty = true;
        }

        SnarlState {
            offset,
            scale,
            target_scale,
            dirty,
        }
    }

    #[inline(always)]
    pub fn initial<T>(viewport: Rect, snarl: &Snarl<T>, style: &SnarlStyle) -> Self {
        if snarl.nodes.is_empty() {
            let scale = 1.0f32.clamp(style.min_scale, style.max_scale);

            return SnarlState {
                offset: Vec2::ZERO,
                scale: scale,
                target_scale: scale,
                dirty: true,
            };
        }

        let mut bb = Rect::NOTHING;

        for (_, node) in snarl.nodes.iter() {
            bb.extend_with(node.pos);
            bb.extend_with(node.pos + vec2(100.0, 100.0));
        }

        let bb_size = bb.size();
        let viewport_size = viewport.size();

        let scale = (viewport_size.x / bb_size.x)
            .min(viewport_size.y / bb_size.y)
            .min(style.max_scale)
            .max(style.min_scale);

        let offset = bb.center().to_vec2() * scale;

        SnarlState {
            offset,
            scale,
            target_scale: scale,
            dirty: true,
        }
    }

    #[inline(always)]
    pub fn store(&self, cx: &Context, id: Id) {
        if self.dirty {
            cx.data_mut(|d| {
                d.insert_temp(
                    id,
                    SnarlStateData {
                        offset: self.offset,
                        scale: self.scale,
                        target_scale: self.target_scale,
                    },
                )
            });
        }
    }

    #[inline(always)]
    pub fn pan(&mut self, delta: Vec2) {
        self.offset += delta;
        self.dirty = true;
    }

    #[inline(always)]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    #[inline(always)]
    pub fn set_scale(&mut self, scale: f32) {
        self.target_scale = scale;
        self.dirty = true;
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
