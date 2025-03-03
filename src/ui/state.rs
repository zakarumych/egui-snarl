use std::hash::Hash;

use egui::{ahash::HashSet, emath::GuiRounding, style::Spacing, Context, Id, Pos2, Rect, Ui, Vec2};

use crate::{InPinId, NodeId, OutPinId, Snarl};

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
    /// Where viewport in graph's space.
    viewport: Rect,

    new_wires: Option<NewWires>,

    id: Id,

    /// Flag indicating that the graph state is dirty must be saved.
    dirty: bool,

    /// Flag indicating that the link menu is open.
    is_link_menu_open: bool,

    /// Order of nodes to draw.
    draw_order: Vec<NodeId>,

    /// Active rect selection.
    rect_selection: Option<RectSelect>,

    /// List of currently selected nodes.
    selected_nodes: Vec<NodeId>,
}

#[derive(Clone)]
struct DrawOrder(Vec<NodeId>);

#[derive(Clone)]
struct SelectedNodes(Vec<NodeId>);

struct SnarlStateData {
    viewport: Rect,
    is_link_menu_open: bool,
    draw_order: Vec<NodeId>,
    new_wires: Option<NewWires>,
    rect_selection: Option<RectSelect>,
    selected_nodes: Vec<NodeId>,
}

#[derive(Clone)]
struct SnarlStateDataHeader {
    viewport: Rect,
    is_link_menu_open: bool,
}

impl SnarlStateData {
    fn save(self, cx: &Context, id: Id) {
        cx.data_mut(|d| {
            d.insert_temp(
                id,
                SnarlStateDataHeader {
                    viewport: self.viewport,
                    is_link_menu_open: self.is_link_menu_open,
                },
            );

            if let Some(new_wires) = self.new_wires {
                d.insert_temp::<NewWires>(id, new_wires);
            } else {
                d.remove::<NewWires>(id);
            }

            if let Some(rect_selection) = self.rect_selection {
                d.insert_temp::<RectSelect>(id, rect_selection);
            } else {
                d.remove::<RectSelect>(id);
            }

            if self.selected_nodes.is_empty() {
                d.remove::<SelectedNodes>(id);
            } else {
                d.insert_temp::<SelectedNodes>(id, SelectedNodes(self.selected_nodes));
            }

            if self.draw_order.is_empty() {
                d.remove::<DrawOrder>(id);
            } else {
                d.insert_temp::<DrawOrder>(id, DrawOrder(self.draw_order));
            }
        });
    }

    fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data(|d| {
            let small = d.get_temp::<SnarlStateDataHeader>(id)?;
            let new_wires = d.get_temp(id);
            let rect_selection = d.get_temp(id);

            let selected_nodes = d.get_temp(id).unwrap_or(SelectedNodes(Vec::new())).0;
            let draw_order = d.get_temp(id).unwrap_or(DrawOrder(Vec::new())).0;

            Some(SnarlStateData {
                viewport: small.viewport,
                is_link_menu_open: small.is_link_menu_open,
                new_wires,
                rect_selection,
                selected_nodes,
                draw_order,
            })
        })
    }
}

fn prune_selected_nodes<T>(selected_nodes: &mut Vec<NodeId>, snarl: &Snarl<T>) -> bool {
    let old_size = selected_nodes.len();
    selected_nodes.retain(|node| snarl.nodes.contains(node.0));
    old_size != selected_nodes.len()
}

impl SnarlState {
    pub fn load<T>(cx: &Context, id: Id, snarl: &Snarl<T>) -> Self {
        let Some(mut data) = SnarlStateData::load(cx, id) else {
            cx.request_discard("Initial placing");
            return Self::initial(id, snarl);
        };

        let dirty = prune_selected_nodes(&mut data.selected_nodes, snarl);

        SnarlState {
            viewport: data.viewport,
            new_wires: data.new_wires,
            is_link_menu_open: data.is_link_menu_open,
            id,
            dirty,
            draw_order: data.draw_order,
            rect_selection: data.rect_selection,
            selected_nodes: data.selected_nodes,
        }
    }

    fn initial<T>(id: Id, snarl: &Snarl<T>) -> Self {
        let mut bb = Rect::NOTHING;

        for (_, node) in &snarl.nodes {
            bb.extend_with(node.pos);
        }

        bb = bb.expand(100.0);

        SnarlState {
            viewport: bb,
            new_wires: None,
            is_link_menu_open: false,
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
                viewport: self.viewport,
                new_wires: self.new_wires,
                is_link_menu_open: self.is_link_menu_open,
                draw_order: self.draw_order,
                rect_selection: self.rect_selection,
                selected_nodes: self.selected_nodes,
            };
            data.save(cx, self.id);
        }
    }

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: Rect) {
        if self.viewport != viewport {
            self.viewport = viewport;
            self.dirty = true;
        }
    }

    #[inline(always)]
    pub fn screen_pos_to_graph(&self, pos: Pos2, ui_rect: Rect) -> Pos2 {
        let x = egui::emath::remap(pos.x, ui_rect.x_range(), self.viewport.x_range());
        let y = egui::emath::remap(pos.y, ui_rect.y_range(), self.viewport.y_range());
        egui::pos2(x, y)
    }

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

    pub const fn has_new_wires(&self) -> bool {
        self.new_wires.is_some()
    }

    pub const fn has_new_wires_in(&self) -> bool {
        matches!(self.new_wires, Some(NewWires::In(_)))
    }

    pub const fn has_new_wires_out(&self) -> bool {
        matches!(self.new_wires, Some(NewWires::Out(_)))
    }

    pub const fn new_wires(&self) -> Option<&NewWires> {
        self.new_wires.as_ref()
    }

    pub fn take_wires(&mut self) -> Option<NewWires> {
        self.dirty |= self.new_wires.is_some();
        self.new_wires.take()
    }

    pub(crate) fn revert_take_wires(&mut self, wires: NewWires) {
        self.new_wires = Some(wires);
    }

    pub(crate) fn open_link_menu(&mut self) {
        self.is_link_menu_open = true;
        self.dirty = true;
    }

    pub(crate) fn close_link_menu(&mut self) {
        self.new_wires = None;
        self.is_link_menu_open = false;
        self.dirty = true;
    }

    pub(crate) const fn is_link_menu_open(&self) -> bool {
        self.is_link_menu_open
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

impl<T> Snarl<T> {
    /// Returns nodes selected in the UI.
    ///
    /// Use `id_salt` and [`Ui`] that were used in [`Snarl::show`] method.
    ///
    /// If same [`Ui`] is not available, use [`Snarl::get_selected_nodes_at`] and provide `id` of the [`Ui`] used in [`Snarl::show`] method.
    pub fn get_selected_nodes(id_salt: impl Hash, ui: &mut Ui) -> Vec<NodeId> {
        Self::get_selected_nodes_at(id_salt, ui.id(), ui.ctx())
    }

    /// Returns nodes selected in the UI.
    ///
    /// Use `id_salt` as well as [`Id`] and [`Context`] of the [`Ui`] that were used in [`Snarl::show`] method.
    pub fn get_selected_nodes_at(id_salt: impl Hash, id: Id, cx: &Context) -> Vec<NodeId> {
        let snarl_id = id.with(id_salt);

        cx.data(|d| {
            d.get_temp::<SelectedNodes>(snarl_id)
                .unwrap_or(SelectedNodes(Vec::new()))
                .0
        })
    }
}
