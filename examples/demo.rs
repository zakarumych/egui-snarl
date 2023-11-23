use std::cell::RefCell;

use eframe::App;
use egui::{pos2, vec2, Color32, InnerResponse, Ui};
use egui_snarl::{
    ui::{Effects, Forbidden, NodeInPin, NodeOutPin, Pin, SnarlViewer},
    InPin, OutPin, Snarl,
};

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    /// The value is editable in UI.
    Integer(i32),

    /// Value node with a single output.
    ///
    /// It has two inputs, ediable if not connected.
    Add([i32; 2]),
}

struct DemoViewer;

impl SnarlViewer<DemoNode> for DemoViewer {
    fn node_picker(&mut self, _ui: &mut Ui) -> egui::InnerResponse<Option<DemoNode>> {
        todo!()
    }

    #[inline]
    fn connect(
        &mut self,
        _from: NodeOutPin<DemoNode>,
        to: NodeInPin<DemoNode>,
        effects: &mut Effects<DemoNode>,
    ) -> Result<(), Forbidden> {
        for remote in &to.remotes {
            effects.disconnect(
                OutPin {
                    node: remote.node_idx,
                    output: remote.pin_idx,
                },
                to.in_pin,
            );
        }

        Ok(())
    }

    fn title(&mut self, node: &DemoNode) -> std::borrow::Cow<'static, str> {
        match node {
            DemoNode::Sink => "Sink".into(),
            DemoNode::Integer(_) => "Integer".into(),
            DemoNode::Add { .. } => "Add".into(),
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 1,
            DemoNode::Integer(_) => 0,
            DemoNode::Add { .. } => 2,
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::Integer(_) => 1,
            DemoNode::Add { .. } => 1,
        }
    }

    fn show_input(
        &mut self,
        node: &RefCell<DemoNode>,
        pin: Pin<DemoNode>,
        ui: &mut Ui,
        _effects: &mut Effects<DemoNode>,
    ) -> egui::InnerResponse<Color32> {
        let demo_node = *node.borrow();
        match demo_node {
            DemoNode::Sink => {
                assert_eq!(pin.pin_idx, 0, "Sink node has only one input");

                match &*pin.remotes {
                    [] => {
                        let r = ui.label("None");
                        InnerResponse::new(Color32::GRAY, r)
                    }
                    [remote] => match *remote.node.borrow() {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(value) => {
                            assert_eq!(remote.pin_idx, 0, "Integer node has only one output");
                            let r = ui.label(format!("{}", value));
                            InnerResponse::new(Color32::RED, r)
                        }
                        DemoNode::Add([a, b]) => {
                            assert_eq!(remote.pin_idx, 0, "Integer node has only one output");
                            let r = ui.label(format!("{}", a + b));
                            InnerResponse::new(Color32::RED, r)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Integer(_) => {
                unreachable!("Integer node has no inputs")
            }
            DemoNode::Add(_) => match &*pin.remotes {
                [] => match &mut *node.borrow_mut() {
                    DemoNode::Add(values) => {
                        let r = ui.add(egui::DragValue::new(&mut values[pin.pin_idx]));
                        InnerResponse::new(Color32::GREEN, r)
                    }
                    _ => unreachable!(),
                },
                [remote] => {
                    let remote_node = *remote.node.borrow();
                    match remote_node {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(value) => {
                            assert_eq!(remote.pin_idx, 0, "Integer node has only one output");
                            match &mut *node.borrow_mut() {
                                DemoNode::Add(values) => {
                                    values[pin.pin_idx] = value;
                                }
                                _ => unreachable!(),
                            }
                            let r = ui.label(format!("{}", value));
                            InnerResponse::new(Color32::RED, r)
                        }
                        DemoNode::Add([a, b]) => {
                            assert_eq!(remote.pin_idx, 0, "Integer node has only one output");
                            match &mut *node.borrow_mut() {
                                DemoNode::Add(values) => {
                                    values[pin.pin_idx] = a + b;
                                }
                                _ => unreachable!(),
                            }
                            let r = ui.label(format!("{}", a + b));
                            InnerResponse::new(Color32::RED, r)
                        }
                    }
                }
                _ => unreachable!("Add node has only one wire"),
            },
        }
    }

    fn show_output(
        &mut self,
        node: &RefCell<DemoNode>,
        pin: Pin<DemoNode>,
        ui: &mut Ui,
        _effects: &mut Effects<DemoNode>,
    ) -> egui::InnerResponse<Color32> {
        match *node.borrow_mut() {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Integer(ref mut value) => {
                assert_eq!(pin.pin_idx, 0, "Integer node has only one output");
                let r = ui.add(egui::DragValue::new(value));
                InnerResponse::new(Color32::RED, r)
            }
            DemoNode::Add([a, b]) => {
                assert_eq!(pin.pin_idx, 0, "Add node has only one output");
                let r = ui.label(format!("{}", a + b));
                InnerResponse::new(Color32::RED, r)
            }
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DemoApp {
    snarl: Snarl<DemoNode>,
}

impl DemoApp {
    pub fn new() -> Self {
        let mut snarl = Snarl::new();

        let i = snarl.add_node(
            DemoNode::Integer(42),
            egui::Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 50.0)),
        );

        let a = snarl.add_node(
            DemoNode::Add([0, 0]),
            egui::Rect::from_min_size(pos2(30.0, 80.0), vec2(100.0, 50.0)),
        );

        let s = snarl.add_node(
            DemoNode::Sink,
            egui::Rect::from_min_size(pos2(190.0, 60.0), vec2(100.0, 50.0)),
        );

        DemoApp { snarl }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(&mut DemoViewer, egui::Id::new("snarl"), ui);
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|_| Box::new(DemoApp::new())),
    )
}
