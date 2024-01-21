use egui::{style::Spacing, Align, Context, Id, Pos2, Rect, Vec2};

use crate::{InPinId, OutPinId, Snarl};

use super::SnarlStyle;

/// Node UI state.

pub struct NodeState {
    /// Node size for this frame.
    /// It is updated to fit content.
    size: Vec2,
    header_height: f32,
    body_width: f32,
    footer_width: f32,

    id: Id,
    scale: f32,
    dirty: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct NodeData {
    unscaled_size: Vec2,
    unscaled_header_height: f32,
    unscaled_body_width: f32,
    unsacled_footer_width: f32,
}

impl NodeState {
    pub fn load(cx: &Context, id: Id, spacing: &Spacing, scale: f32) -> Self {
        match cx.data_mut(|d| d.get_temp::<NodeData>(id)) {
            Some(data) => NodeState {
                size: data.unscaled_size * scale,
                header_height: data.unscaled_header_height * scale,
                body_width: data.unscaled_body_width * scale,
                footer_width: data.unsacled_footer_width * scale,
                id,
                scale,
                dirty: false,
            },
            None => Self::initial(id, spacing, scale),
        }
    }

    pub fn clear(&mut self, cx: &Context) {
        cx.data_mut(|d| d.remove::<Self>(self.id));
        self.dirty = false;
    }

    pub fn store(&self, cx: &Context) {
        if self.dirty {
            cx.data_mut(|d| {
                d.insert_temp(
                    self.id,
                    NodeData {
                        unscaled_size: self.size / self.scale,
                        unscaled_header_height: self.header_height / self.scale,
                        unscaled_body_width: self.body_width / self.scale,
                        unsacled_footer_width: self.footer_width / self.scale,
                    },
                )
            });
        }
    }

    /// Finds node rect at specific position (excluding node frame margin).
    pub fn node_rect(&self, pos: Pos2, openness: f32) -> Rect {
        Rect::from_min_size(
            pos,
            egui::vec2(
                self.size.x,
                f32::max(self.header_height, self.size.y * openness),
            ),
        )
    }

    pub fn payload_offset(&self, openness: f32) -> f32 {
        (self.size.y) * (1.0 - openness)
    }

    pub fn align_body(&mut self, rect: Rect) -> Rect {
        let x_range = Align::Center.align_size_within_range(self.body_width, rect.x_range());
        Rect::from_x_y_ranges(x_range, rect.y_range())
    }

    pub fn align_footer(&mut self, rect: Rect) -> Rect {
        let x_range = Align::Center.align_size_within_range(self.footer_width, rect.x_range());
        Rect::from_x_y_ranges(x_range, rect.y_range())
    }

    pub fn set_size(&mut self, size: Vec2) {
        if self.size != size {
            self.size = size;
            self.dirty = true;
        }
    }

    pub fn set_header_height(&mut self, height: f32) {
        if self.header_height != height {
            self.header_height = height;
            self.dirty = true;
        }
    }

    pub fn set_body_width(&mut self, width: f32) {
        if self.body_width != width {
            self.body_width = width;
            self.dirty = true;
        }
    }

    pub fn set_footer_width(&mut self, width: f32) {
        if self.footer_width != width {
            self.footer_width = width;
            self.dirty = true;
        }
    }

    fn initial(id: Id, spacing: &Spacing, scale: f32) -> Self {
        NodeState {
            size: spacing.interact_size,
            header_height: spacing.interact_size.y,
            body_width: 0.0,
            footer_width: 0.0,
            id,
            dirty: true,
            scale,
        }
    }
}

#[derive(Clone)]
pub enum NewWires {
    In(Vec<InPinId>),
    Out(Vec<OutPinId>),
}

pub struct SnarlState {
    /// Where viewport's center in graph's space.
    offset: Vec2,

    /// Scale of the viewport.
    scale: f32,

    target_scale: f32,

    new_wires: Option<NewWires>,

    id: Id,

    /// Flag indicating that the graph state is dirty must be saved.
    dirty: bool,
}

#[derive(Clone)]
struct SnarlStateData {
    offset: Vec2,
    scale: f32,
    target_scale: f32,
    new_wires: Option<NewWires>,
}

impl SnarlState {
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
            new_wires,
        }) = cx.data_mut(|d| d.get_temp(id))
        else {
            return Self::initial(id, viewport, snarl, style);
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
            new_wires,
            id,
            dirty,
        }
    }

    fn initial<T>(id: Id, viewport: Rect, snarl: &Snarl<T>, style: &SnarlStyle) -> Self {
        let mut bb = Rect::NOTHING;

        for (_, node) in snarl.nodes.iter() {
            bb.extend_with(node.pos);
        }

        if !bb.is_positive() {
            let scale = 1.0f32.clamp(style.min_scale, style.max_scale);

            return SnarlState {
                offset: Vec2::ZERO,
                scale,
                target_scale: scale,
                new_wires: None,
                id,
                dirty: true,
            };
        }

        bb = bb.expand(100.0);

        let bb_size = bb.size();
        let viewport_size = viewport.size();

        let scale = (viewport_size.x / bb_size.x)
            .min(1.0)
            .min(viewport_size.y / bb_size.y)
            .min(style.max_scale)
            .max(style.min_scale);

        let offset = bb.center().to_vec2() * scale;

        SnarlState {
            offset,
            scale,
            target_scale: scale,
            new_wires: None,
            id,
            dirty: true,
        }
    }

    #[inline(always)]
    pub fn store(self, cx: &Context) {
        if self.dirty {
            cx.data_mut(|d| {
                d.insert_temp(
                    self.id,
                    SnarlStateData {
                        offset: self.offset,
                        scale: self.scale,
                        target_scale: self.target_scale,
                        new_wires: self.new_wires,
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

    // #[inline(always)]
    // pub fn graph_vec_to_screen(&self, size: Vec2) -> Vec2 {
    //     size * self.scale
    // }

    #[inline(always)]
    pub fn screen_vec_to_graph(&self, size: Vec2) -> Vec2 {
        size / self.scale
    }

    // #[inline(always)]
    // pub fn graph_value_to_screen(&self, value: f32) -> f32 {
    //     value * self.scale
    // }

    // #[inline(always)]
    // pub fn screen_value_to_graph(&self, value: f32) -> f32 {
    //     value / self.scale
    // }

    pub fn start_new_wire_in(&mut self, pin: InPinId) {
        self.new_wires = Some(NewWires::In(vec![pin]));
        self.dirty = true;
    }

    pub fn start_new_wire_out(&mut self, pin: OutPinId) {
        self.new_wires = Some(NewWires::Out(vec![pin]));
        self.dirty = true;
    }

    pub fn start_new_wires_in(&mut self, pins: &[InPinId]) {
        self.new_wires = Some(NewWires::In(pins.to_vec()));
        self.dirty = true;
    }

    pub fn start_new_wires_out(&mut self, pins: &[OutPinId]) {
        self.new_wires = Some(NewWires::Out(pins.to_vec()));
        self.dirty = true;
    }

    pub fn add_new_wire_in(&mut self, pin: InPinId) {
        if let Some(NewWires::In(pins)) = &mut self.new_wires {
            if !pins.contains(&pin) {
                pins.push(pin);
                self.dirty = true;
            }
        }
    }

    pub fn add_new_wire_out(&mut self, pin: OutPinId) {
        if let Some(NewWires::Out(pins)) = &mut self.new_wires {
            if !pins.contains(&pin) {
                pins.push(pin);
                self.dirty = true;
            }
        }
    }

    pub fn remove_new_wire_in(&mut self, pin: InPinId) {
        if let Some(NewWires::In(pins)) = &mut self.new_wires {
            if let Some(idx) = pins.iter().position(|p| *p == pin) {
                pins.swap_remove(idx);
                self.dirty = true;
            }
        }
    }

    pub fn remove_new_wire_out(&mut self, pin: OutPinId) {
        if let Some(NewWires::Out(pins)) = &mut self.new_wires {
            if let Some(idx) = pins.iter().position(|p| *p == pin) {
                pins.swap_remove(idx);
                self.dirty = true;
            }
        }
    }

    pub fn has_new_wires(&self) -> bool {
        self.new_wires.is_some()
    }

    pub fn new_wires(&self) -> Option<&NewWires> {
        self.new_wires.as_ref()
    }

    pub fn take_wires(&mut self) -> Option<NewWires> {
        self.dirty |= self.new_wires.is_some();
        self.new_wires.take()
    }

    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
        self.dirty = true;
    }
}
