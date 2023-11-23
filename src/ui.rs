use egui::*;

use crate::{Pin, Remote, Snarl, SnarlView, SnarlViewer};

impl<T> Snarl<T> {
    pub fn show<V>(&mut self, viewer: &mut V, ui: &mut Ui) -> Response
    where
        V: SnarlViewer<T>,
    {
        SnarlView {
            snarl: self,
            viewer,
            pos: Pos2::ZERO,
            scale: 1.0,
        }
        .show(ui)
    }
}

impl<T, V> SnarlView<'_, T, V>
where
    V: SnarlViewer<T>,
{
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        Frame::none()
            .fill(Color32::DARK_GRAY)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                let max_rect = ui.max_rect();

                let mut responses = Vec::new();
                let mut nodes_moved = Vec::new();

                for (node_idx, node) in &self.snarl.nodes {
                    let area = Area::new(Id::new(node_idx))
                        .current_pos(node.rect.min + vec2(max_rect.min.x, max_rect.min.y))
                        // .constrain_to(max_rect)
                        ;

                    let r = area.show(ui.ctx(), |ui| {
                        Frame::window(ui.style()).show(ui, |ui| {
                            ui.set_clip_rect(max_rect);
                            ui.set_max_size(node.rect.max - node.rect.min);

                            ui.vertical_centered(|ui| {
                                ui.label(self.viewer.title(&node.value.borrow()));

                                ui.horizontal(|ui| {
                                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                        let inputs = self.viewer.inputs(&node.value.borrow());

                                        for input_idx in 0..inputs {
                                            let pin = Pin {
                                                local: input_idx,
                                                remote: self
                                                    .snarl
                                                    .wires
                                                    .wired_outputs(node_idx, input_idx)
                                                    .map(|(n, o)| Remote {
                                                        node: &self.snarl.nodes[n].value,
                                                        idx: o,
                                                    })
                                                    .collect(),
                                            };

                                            ui.horizontal(|ui| {
                                                ui.allocate_space(vec2(10.0, 10.0));

                                                let r =
                                                    self.viewer.show_input(&node.value, pin, ui);
                                                responses.push(r.inner);

                                                let x = r.response.rect.left()
                                                    - 5.0
                                                    - ui.style().spacing.item_spacing.x;

                                                let y = (r.response.rect.top()
                                                    + r.response.rect.bottom())
                                                    / 2.0;

                                                let r = ui.allocate_rect(
                                                    Rect::from_center_size(
                                                        pos2(x, y),
                                                        vec2(10.0, 10.0),
                                                    ),
                                                    Sense::click(),
                                                );

                                                ui.painter().circle(
                                                    r.rect.center(),
                                                    5.0,
                                                    Color32::BLUE,
                                                    Stroke::new(1.0, Color32::BLACK),
                                                );
                                            });
                                        }
                                    });

                                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                        let outputs = self.viewer.outputs(&node.value.borrow());

                                        for idx in 0..outputs {
                                            let pin = Pin {
                                                local: idx,
                                                remote: self
                                                    .snarl
                                                    .wires
                                                    .wired_inputs(node_idx, idx)
                                                    .map(|(n, i)| Remote {
                                                        node: &self.snarl.nodes[n].value,
                                                        idx: i,
                                                    })
                                                    .collect(),
                                            };

                                            ui.horizontal(|ui| {
                                                let r =
                                                    self.viewer.show_output(&node.value, pin, ui);
                                                responses.push(r.inner);

                                                ui.allocate_space(vec2(10.0, 10.0));

                                                let x = r.response.rect.right()
                                                    + 5.0
                                                    + ui.style().spacing.item_spacing.x;
                                                let y = (r.response.rect.top()
                                                    + r.response.rect.bottom())
                                                    / 2.0;

                                                let r = ui.allocate_rect(
                                                    Rect::from_center_size(
                                                        pos2(x, y),
                                                        vec2(10.0, 10.0),
                                                    ),
                                                    Sense::click(),
                                                );

                                                ui.painter().circle(
                                                    r.rect.center(),
                                                    5.0,
                                                    Color32::BLUE,
                                                    Stroke::new(1.0, Color32::BLACK),
                                                );
                                            });
                                        }
                                    });
                                });
                            });
                        });
                    });

                    if r.response.dragged() {
                        nodes_moved.push((node_idx, r.response.drag_delta()));
                    }
                }

                let leftover = ui.available_size();
                ui.allocate_exact_size(leftover, Sense::hover());

                for (node_idx, delta) in nodes_moved {
                    let node = &mut self.snarl.nodes[node_idx];
                    node.rect.min += delta;
                    node.rect.max += delta;
                }

                for e in responses {
                    self.apply_effects(e);
                }
            })
            .response
    }
}
