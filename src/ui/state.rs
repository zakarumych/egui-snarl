use egui::{
    Context, Id, Pos2, Rect, Ui, Vec2,
    ahash::HashSet,
    emath::{GuiRounding, TSTransform},
    style::Spacing,
};

use crate::{InPinId, NodeId, OutPinId, Snarl};

use super::{SnarlWidget, transform_matching_points};

/// Node UI state.
pub struct NodeState {
    /// Node size for this frame.
    /// It is updated to fit content.
    size: Vec2,
    header_height: f32,

    id: Id,
    dirty: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct NodeData {
    size: Vec2,
    header_height: f32,
}

impl NodeState {
    pub fn load(cx: &Context, id: Id, spacing: &Spacing) -> Self {
        cx.data_mut(|d| d.get_temp::<NodeData>(id)).map_or_else(
            || {
                cx.request_discard("NodeState initialization");
                Self::initial(id, spacing)
            },
            |data| NodeState {
                size: data.size,
                header_height: data.header_height,
                id,
                dirty: false,
            },
        )
    }

    pub fn clear(self, cx: &Context) {
        cx.data_mut(|d| d.remove::<Self>(self.id));
    }

    pub fn store(&self, cx: &Context) {
        if self.dirty {
            cx.data_mut(|d| {
                d.insert_temp(
                    self.id,
                    NodeData {
                        size: self.size,
                        header_height: self.header_height,
                    },
                );
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
        .round_ui()
    }

    pub fn payload_offset(&self, openness: f32) -> f32 {
        ((self.size.y) * (1.0 - openness)).round_ui()
    }

    pub fn set_size(&mut self, size: Vec2) {
        if self.size != size {
            self.size = size;
            self.dirty = true;
        }
    }

    pub fn header_height(&self) -> f32 {
        self.header_height.round_ui()
    }

    pub fn set_header_height(&mut self, height: f32) {
        #[allow(clippy::float_cmp)]
        if self.header_height != height {
            self.header_height = height;
            self.dirty = true;
        }
    }

    const fn initial(id: Id, spacing: &Spacing) -> Self {
        NodeState {
            size: spacing.interact_size,
            header_height: spacing.interact_size.y,
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

#[derive(Clone, Copy)]
struct RectSelect {
    origin: Pos2,
    current: Pos2,
}

pub struct SnarlState {
    /// Snarl viewport transform to global space.
    to_global: TSTransform,

    new_wires: Option<NewWires>,

    /// Flag indicating that new wires are owned by the menu now.
    new_wires_menu: bool,

    id: Id,

    /// Flag indicating that the graph state is dirty must be saved.
    dirty: bool,

    /// Active rect selection.
    rect_selection: Option<RectSelect>,

    /// Order of nodes to draw.
    draw_order: Vec<NodeId>,

    /// List of currently selected nodes.
    selected_nodes: Vec<NodeId>,
}

#[derive(Clone, Default)]
struct DrawOrder(Vec<NodeId>);

impl DrawOrder {
    fn save(self, cx: &Context, id: Id) {
        cx.data_mut(|d| {
            if self.0.is_empty() {
                d.remove_temp::<Self>(id);
            } else {
                d.insert_temp::<Self>(id, self);
            }
        });
    }

    fn load(cx: &Context, id: Id) -> Self {
        cx.data(|d| d.get_temp::<Self>(id)).unwrap_or_default()
    }
}

#[derive(Clone, Default)]
struct SelectedNodes(Vec<NodeId>);

impl SelectedNodes {
    fn save(self, cx: &Context, id: Id) {
        cx.data_mut(|d| {
            if self.0.is_empty() {
                d.remove_temp::<Self>(id);
            } else {
                d.insert_temp::<Self>(id, self);
            }
        });
    }

    fn load(cx: &Context, id: Id) -> Self {
        cx.data(|d| d.get_temp::<Self>(id)).unwrap_or_default()
    }
}

#[derive(Clone)]
struct SnarlStateData {
    to_global: TSTransform,
    new_wires: Option<NewWires>,
    new_wires_menu: bool,
    rect_selection: Option<RectSelect>,
}

impl SnarlStateData {
    fn save(self, cx: &Context, id: Id) {
        cx.data_mut(|d| {
            d.insert_temp(id, self);
        });
    }

    fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data(|d| d.get_temp(id))
    }
}

fn prune_selected_nodes<T>(selected_nodes: &mut Vec<NodeId>, snarl: &Snarl<T>) -> bool {
    let old_size = selected_nodes.len();
    selected_nodes.retain(|node| snarl.nodes.contains(node.0));
    old_size != selected_nodes.len()
}

impl SnarlState {
    pub fn load<T>(
        cx: &Context,
        id: Id,
        snarl: &Snarl<T>,
        ui_rect: Rect,
        min_scale: f32,
        max_scale: f32,
    ) -> Self {
        let Some(data) = SnarlStateData::load(cx, id) else {
            cx.request_discard("Initial placing");
            return Self::initial(id, snarl, ui_rect, min_scale, max_scale);
        };

        let mut selected_nodes = SelectedNodes::load(cx, id).0;
        let dirty = prune_selected_nodes(&mut selected_nodes, snarl);

        let draw_order = DrawOrder::load(cx, id).0;

        SnarlState {
            to_global: data.to_global,
            new_wires: data.new_wires,
            new_wires_menu: data.new_wires_menu,
            id,
            dirty,
            rect_selection: data.rect_selection,
            draw_order,
            selected_nodes,
        }
    }

    fn initial<T>(id: Id, snarl: &Snarl<T>, ui_rect: Rect, min_scale: f32, max_scale: f32) -> Self {
        let mut bb = Rect::NOTHING;

        for (_, node) in &snarl.nodes {
            bb.extend_with(node.pos);
        }

        if bb.is_finite() {
            bb = bb.expand(100.0);
        } else if ui_rect.is_finite() {
            bb = ui_rect;
        } else {
            bb = Rect::from_min_max(Pos2::new(-100.0, -100.0), Pos2::new(100.0, 100.0));
        }

        let scaling2 = ui_rect.size() / bb.size();
        let scaling = scaling2.min_elem().clamp(min_scale, max_scale);

        let to_global = transform_matching_points(bb.center(), ui_rect.center(), scaling);

        SnarlState {
            to_global,
            new_wires: None,
            new_wires_menu: false,
            id,
            dirty: true,
            draw_order: Vec::new(),
            rect_selection: None,
            selected_nodes: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn store<T>(mut self, snarl: &Snarl<T>, cx: &Context) {
        self.dirty |= prune_selected_nodes(&mut self.selected_nodes, snarl);

        if self.dirty {
            let data = SnarlStateData {
                to_global: self.to_global,
                new_wires: self.new_wires,
                new_wires_menu: self.new_wires_menu,
                rect_selection: self.rect_selection,
            };
            data.save(cx, self.id);

            DrawOrder(self.draw_order).save(cx, self.id);
            SelectedNodes(self.selected_nodes).save(cx, self.id);
        }
    }

    pub fn to_global(&self) -> TSTransform {
        self.to_global
    }

    pub fn set_to_global(&mut self, to_global: TSTransform) {
        if self.to_global != to_global {
            self.to_global = to_global;
            self.dirty = true;
        }
    }

    pub fn look_at(&mut self, view: Rect, ui_rect: Rect, min_scale: f32, max_scale: f32) {
        let scaling2 = ui_rect.size() / view.size();
        let scaling = scaling2.min_elem().clamp(min_scale, max_scale);

        let to_global = transform_matching_points(view.center(), ui_rect.center(), scaling);

        if self.to_global != to_global {
            self.to_global = to_global;
            self.dirty = true;
        }
    }

    pub fn start_new_wire_in(&mut self, pin: InPinId) {
        self.new_wires = Some(NewWires::In(vec![pin]));
        self.new_wires_menu = false;
        self.dirty = true;
    }

    pub fn start_new_wire_out(&mut self, pin: OutPinId) {
        self.new_wires = Some(NewWires::Out(vec![pin]));
        self.new_wires_menu = false;
        self.dirty = true;
    }

    pub fn start_new_wires_in(&mut self, pins: &[InPinId]) {
        self.new_wires = Some(NewWires::In(pins.to_vec()));
        self.new_wires_menu = false;
        self.dirty = true;
    }

    pub fn start_new_wires_out(&mut self, pins: &[OutPinId]) {
        self.new_wires = Some(NewWires::Out(pins.to_vec()));
        self.new_wires_menu = false;
        self.dirty = true;
    }

    pub fn add_new_wire_in(&mut self, pin: InPinId) {
        debug_assert!(self.new_wires_menu == false);
        let Some(NewWires::In(pins)) = &mut self.new_wires else {
            unreachable!();
        };

        if !pins.contains(&pin) {
            pins.push(pin);
            self.dirty = true;
        }
    }

    pub fn add_new_wire_out(&mut self, pin: OutPinId) {
        debug_assert!(self.new_wires_menu == false);
        let Some(NewWires::Out(pins)) = &mut self.new_wires else {
            unreachable!();
        };

        if !pins.contains(&pin) {
            pins.push(pin);
            self.dirty = true;
        }
    }

    pub fn remove_new_wire_in(&mut self, pin: InPinId) {
        debug_assert!(self.new_wires_menu == false);
        let Some(NewWires::In(pins)) = &mut self.new_wires else {
            unreachable!();
        };

        if let Some(idx) = pins.iter().position(|p| *p == pin) {
            pins.swap_remove(idx);
            self.dirty = true;
        }
    }

    pub fn remove_new_wire_out(&mut self, pin: OutPinId) {
        debug_assert!(self.new_wires_menu == false);
        let Some(NewWires::Out(pins)) = &mut self.new_wires else {
            unreachable!();
        };

        if let Some(idx) = pins.iter().position(|p| *p == pin) {
            pins.swap_remove(idx);
            self.dirty = true;
        }
    }

    pub const fn has_new_wires(&self) -> bool {
        match (self.new_wires.as_ref(), self.new_wires_menu) {
            (Some(_), false) => true,
            _ => false,
        }
    }

    pub const fn has_new_wires_in(&self) -> bool {
        match (&self.new_wires, self.new_wires_menu) {
            (Some(NewWires::In(_)), false) => true,
            _ => false,
        }
    }

    pub const fn has_new_wires_out(&self) -> bool {
        match (&self.new_wires, self.new_wires_menu) {
            (Some(NewWires::Out(_)), false) => true,
            _ => false,
        }
    }

    pub const fn new_wires(&self) -> Option<&NewWires> {
        match (&self.new_wires, self.new_wires_menu) {
            (Some(new_wires), false) => Some(new_wires),
            _ => None,
        }
    }

    pub fn take_new_wires(&mut self) -> Option<NewWires> {
        match (&self.new_wires, self.new_wires_menu) {
            (Some(_), false) => {
                self.dirty = true;
                self.new_wires.take()
            }
            _ => None,
        }
    }

    pub(crate) fn take_new_wires_menu(&mut self) -> Option<NewWires> {
        match (&self.new_wires, self.new_wires_menu) {
            (Some(_), true) => {
                self.dirty = true;
                self.new_wires.take()
            }
            _ => None,
        }
    }

    pub(crate) fn set_new_wires_menu(&mut self, wires: NewWires) {
        debug_assert!(self.new_wires.is_none());
        self.new_wires = Some(wires);
        self.new_wires_menu = true;
    }

    pub(crate) fn update_draw_order<T>(&mut self, snarl: &Snarl<T>) -> Vec<NodeId> {
        let mut node_ids = snarl
            .nodes
            .iter()
            .map(|(id, _)| NodeId(id))
            .collect::<HashSet<_>>();

        self.draw_order.retain(|id| {
            let has = node_ids.remove(id);
            self.dirty |= !has;
            has
        });

        self.dirty |= !node_ids.is_empty();

        for new_id in node_ids {
            self.draw_order.push(new_id);
        }

        self.draw_order.clone()
    }

    pub(crate) fn node_to_top(&mut self, node: NodeId) {
        if let Some(order) = self.draw_order.iter().position(|idx| *idx == node) {
            self.draw_order.remove(order);
            self.draw_order.push(node);
        }
        self.dirty = true;
    }

    pub fn selected_nodes(&self) -> &[NodeId] {
        &self.selected_nodes
    }

    pub fn select_one_node(&mut self, reset: bool, node: NodeId) {
        if reset {
            if self.selected_nodes[..] == [node] {
                return;
            }

            self.deselect_all_nodes();
        } else if let Some(pos) = self.selected_nodes.iter().position(|n| *n == node) {
            if pos == self.selected_nodes.len() - 1 {
                return;
            }
            self.selected_nodes.remove(pos);
        }
        self.selected_nodes.push(node);
        self.dirty = true;
    }

    pub fn select_many_nodes(&mut self, reset: bool, nodes: impl Iterator<Item = NodeId>) {
        if reset {
            self.deselect_all_nodes();
            self.selected_nodes.extend(nodes);
            self.dirty = true;
        } else {
            nodes.for_each(|node| self.select_one_node(false, node));
        }
    }

    pub fn deselect_one_node(&mut self, node: NodeId) {
        if let Some(pos) = self.selected_nodes.iter().position(|n| *n == node) {
            self.selected_nodes.remove(pos);
            self.dirty = true;
        }
    }

    pub fn deselect_many_nodes(&mut self, nodes: impl Iterator<Item = NodeId>) {
        for node in nodes {
            if let Some(pos) = self.selected_nodes.iter().position(|n| *n == node) {
                self.selected_nodes.remove(pos);
                self.dirty = true;
            }
        }
    }

    pub fn deselect_all_nodes(&mut self) {
        self.dirty |= !self.selected_nodes.is_empty();
        self.selected_nodes.clear();
    }

    pub fn start_rect_selection(&mut self, pos: Pos2) {
        self.dirty |= self.rect_selection.is_none();
        self.rect_selection = Some(RectSelect {
            origin: pos,
            current: pos,
        });
    }

    pub fn stop_rect_selection(&mut self) {
        self.dirty |= self.rect_selection.is_some();
        self.rect_selection = None;
    }

    pub const fn is_rect_selection(&self) -> bool {
        self.rect_selection.is_some()
    }

    pub fn update_rect_selection(&mut self, pos: Pos2) {
        if let Some(rect_selection) = &mut self.rect_selection {
            rect_selection.current = pos;
            self.dirty = true;
        }
    }

    pub fn rect_selection(&self) -> Option<Rect> {
        let rect = self.rect_selection?;
        Some(Rect::from_two_pos(rect.origin, rect.current))
    }
}

impl SnarlWidget {
    /// Returns list of nodes selected in the UI for the `SnarlWidget` with same id.
    ///
    /// Use same `Ui` instance that was used in [`SnarlWidget::show`].
    #[must_use]
    #[inline]
    pub fn get_selected_nodes(self, ui: &Ui) -> Vec<NodeId> {
        self.get_selected_nodes_at(ui.id(), ui.ctx())
    }

    /// Returns list of nodes selected in the UI for the `SnarlWidget` with same id.
    ///
    /// `ui_id` must be the Id of the `Ui` instance that was used in [`SnarlWidget::show`].
    #[must_use]
    #[inline]
    pub fn get_selected_nodes_at(self, ui_id: Id, ctx: &Context) -> Vec<NodeId> {
        let snarl_id = self.get_id(ui_id);

        ctx.data(|d| d.get_temp::<SelectedNodes>(snarl_id).unwrap_or_default().0)
    }
}

/// Returns nodes selected in the UI for the `SnarlWidget` with same ID.
///
/// Only works if [`SnarlWidget::id`] was used.
/// For other cases construct [`SnarlWidget`] and use [`SnarlWidget::get_selected_nodes`] or [`SnarlWidget::get_selected_nodes_at`].
#[must_use]
#[inline]
pub fn get_selected_nodes(id: Id, ctx: &Context) -> Vec<NodeId> {
    ctx.data(|d| d.get_temp::<SelectedNodes>(id).unwrap_or_default().0)
}
