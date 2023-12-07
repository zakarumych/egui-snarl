use egui::{style::Spacing, vec2, Context, Id, Pos2, Rect, Vec2};

use crate::{InPinId, OutPinId, Snarl};

use super::SnarlStyle;

/// Node UI state.

pub struct NodeState {
    /// Node size for this frame.
    /// It is updated to fit content.
    size: Vec2,
    id: Id,

    dirty: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct NodeData {
    size: Vec2,
}

impl NodeState {
    pub fn load(cx: &Context, id: Id, spacing: &Spacing) -> Self {
        match cx.data_mut(|d| d.get_temp(id)) {
            Some(NodeData { size }) => NodeState {
                size,
                id,
                dirty: false,
            },
            None => Self::initial(id, spacing),
        }
    }

    pub fn clear(&mut self, cx: &Context) {
        cx.data_mut(|d| d.remove::<Self>(self.id));
        self.dirty = false;
    }

    pub fn store(&self, cx: &Context) {
        if self.dirty {
            cx.data_mut(|d| d.insert_temp(self.id, NodeData { size: self.size }));
        }
    }

    /// Finds node rect at specific position (excluding node frame margin).
    pub fn node_rect(&self, pos: Pos2) -> Rect {
        Rect::from_min_size(pos, self.size)
    }

    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
        self.dirty = true;
    }

    fn initial(id: Id, spacing: &Spacing) -> Self {
        NodeState {
            // title_size: spacing.interact_size,
            // inputs_size: spacing.interact_size,
            // outputs_size: spacing.interact_size,
            size: spacing.interact_size * 3.0,

            id,

            dirty: true,
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
        if snarl.nodes.is_empty() {
            let scale = 1.0f32.clamp(style.min_scale, style.max_scale);

            return SnarlState {
                offset: Vec2::ZERO,
                scale: scale,
                target_scale: scale,
                new_wires: None,
                id,
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
        match self.new_wires {
            Some(NewWires::In(ref mut pins)) => {
                if !pins.contains(&pin) {
                    pins.push(pin);
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    pub fn add_new_wire_out(&mut self, pin: OutPinId) {
        match self.new_wires {
            Some(NewWires::Out(ref mut pins)) => {
                if !pins.contains(&pin) {
                    pins.push(pin);
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    pub fn remove_new_wire_in(&mut self, pin: InPinId) {
        match self.new_wires {
            Some(NewWires::In(ref mut pins)) => {
                if let Some(idx) = pins.iter().position(|p| *p == pin) {
                    pins.swap_remove(idx);
                    self.dirty = true;
                }
            }
            _ => {}
        }
    }

    pub fn remove_new_wire_out(&mut self, pin: OutPinId) {
        match self.new_wires {
            Some(NewWires::Out(ref mut pins)) => {
                if let Some(idx) = pins.iter().position(|p| *p == pin) {
                    pins.swap_remove(idx);
                    self.dirty = true;
                }
            }
            _ => {}
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
}
