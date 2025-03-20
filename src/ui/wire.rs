use core::f32;

use egui::{
    ahash::HashMap, cache::CacheTrait, epaint::PathShape, pos2, Color32, Id, Pos2, Rect, Shape,
    Stroke, Ui,
};

use crate::Wire;

const MAX_CURVE_SAMPLES: usize = 100;

/// Layer where wires are rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
#[derive(Default)]
pub enum WireLayer {
    /// Wires are rendered behind nodes.
    /// This is default.
    #[default]
    BehindNodes,

    /// Wires are rendered above nodes.
    AboveNodes,
}

/// Controls style in which wire is rendered.
///
/// Variants are given in order of precedence when two pins require different styles.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
#[derive(Default)]
pub enum WireStyle {
    /// Straight line from one endpoint to another.
    Line,

    /// Draw wire as straight lines with 90 degree turns.
    /// Corners has radius of `corner_radius`.
    AxisAligned {
        /// Radius of corners in wire.
        corner_radius: f32,
    },

    /// Draw wire as 3rd degree Bezier curve.
    Bezier3,

    /// Draw wire as 5th degree Bezier curve.
    #[default]
    Bezier5,
}

pub fn pick_wire_style(left: WireStyle, right: WireStyle) -> WireStyle {
    match (left, right) {
        (WireStyle::Line, _) | (_, WireStyle::Line) => WireStyle::Line,
        (
            WireStyle::AxisAligned { corner_radius: a },
            WireStyle::AxisAligned { corner_radius: b },
        ) => WireStyle::AxisAligned {
            corner_radius: f32::max(a, b),
        },
        (WireStyle::AxisAligned { corner_radius }, _)
        | (_, WireStyle::AxisAligned { corner_radius }) => WireStyle::AxisAligned { corner_radius },
        (WireStyle::Bezier3, _) | (_, WireStyle::Bezier3) => WireStyle::Bezier3,
        (WireStyle::Bezier5, WireStyle::Bezier5) => WireStyle::Bezier5,
    }
}

fn adjust_frame_size(
    mut frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
) -> f32 {
    let length = (from - to).length();
    if upscale {
        frame_size = frame_size.max(length / 6.0);
    }
    if downscale {
        frame_size = frame_size.min(length / 6.0);
    }
    frame_size
}

/// Returns 5th degree bezier curve control points for the wire
fn wire_bezier_5(frame_size: f32, from: Pos2, to: Pos2) -> [Pos2; 6] {
    let from_norm_x = frame_size;
    let from_2 = pos2(from.x + from_norm_x, from.y);
    let to_norm_x = -from_norm_x;
    let to_2 = pos2(to.x + to_norm_x, to.y);

    let between = (from_2 - to_2).length();

    if from_2.x <= to_2.x && between >= frame_size * 2.0 {
        let middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.x <= to_2.x {
        let t = (between - (to_2.y - from_2.y).abs())
            / frame_size.mul_add(2.0, -(to_2.y - from_2.y).abs());

        let mut middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let mut middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        if from_2.y >= to_2.y + frame_size {
            let u = (from_2.y - to_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(
                (1.0 - u).mul_add(frame_size, from_2.x),
                frame_size.mul_add(-u, from_2.y),
            );
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if from_2.y >= to_2.y {
            let u = (from_2.y - to_2.y) / frame_size;

            let t0_middle_1 = pos2(
                u.mul_add(frame_size, from_2.x),
                frame_size.mul_add(1.0 - u, from_2.y),
            );
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y + frame_size {
            let u = (to_2.y - from_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(
                (1.0 - u).mul_add(-frame_size, to_2.x),
                frame_size.mul_add(-u, to_2.y),
            );

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y {
            let u = (to_2.y - from_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(
                u.mul_add(-frame_size, to_2.x),
                frame_size.mul_add(1.0 - u, to_2.y),
            );

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else {
            unreachable!();
        }

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= frame_size.mul_add(2.0, to_2.y) {
        let middle_1 = pos2(from_2.x, from_2.y - frame_size);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size {
        let t = (from_2.y - to_2.y - frame_size) / frame_size;

        let middle_1 = pos2(
            (1.0 - t).mul_add(frame_size, from_2.x),
            frame_size.mul_add(-t, from_2.y),
        );
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y {
        let t = (from_2.y - to_2.y) / frame_size;

        let middle_1 = pos2(
            t.mul_add(frame_size, from_2.x),
            frame_size.mul_add(1.0 - t, from_2.y),
        );
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= frame_size.mul_add(2.0, from_2.y) {
        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x, to_2.y - frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size {
        let t = (to_2.y - from_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(
            (1.0 - t).mul_add(-frame_size, to_2.x),
            frame_size.mul_add(-t, to_2.y),
        );

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y {
        let t = (to_2.y - from_2.y) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(
            t.mul_add(-frame_size, to_2.x),
            frame_size.mul_add(1.0 - t, to_2.y),
        );

        [from, from_2, middle_1, middle_2, to_2, to]
    } else {
        unreachable!();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_wire(
    ui: &Ui,
    snarl_id: Id,
    wire: Option<Wire>,
    shapes: &mut Vec<Shape>,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    mut stroke: Stroke,
    threshold: f32,
    style: WireStyle,
) {
    if stroke.width < 1.0 {
        stroke.color = stroke.color.gamma_multiply(stroke.width);
        stroke.width = 1.0;
    }

    let frame_size = adjust_frame_size(frame_size, upscale, downscale, from, to);
    match style {
        WireStyle::Line => {
            let bb = Rect::from_two_pos(from, to);
            if ui.is_rect_visible(bb) {
                shapes.push(Shape::line_segment([from, to], stroke));
            }
        }
        WireStyle::Bezier3 => {
            let [a, b, _, _, c, d] = wire_bezier_5(frame_size, from, to);
            let points = [a, b, c, d];

            draw_bezier_3(ui, snarl_id, wire, &points, stroke, threshold, shapes);
        }

        WireStyle::Bezier5 => {
            let points = wire_bezier_5(frame_size, from, to);

            draw_bezier_5(ui, snarl_id, wire, &points, stroke, threshold, shapes);
        }

        WireStyle::AxisAligned { corner_radius } => {
            draw_axis_aligned(
                corner_radius,
                frame_size,
                from,
                to,
                stroke,
                threshold,
                shapes,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn hit_wire(
    pos: Pos2,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    threshold: f32,
    style: WireStyle,
) -> bool {
    let frame_size = adjust_frame_size(frame_size, upscale, downscale, from, to);
    match style {
        WireStyle::Line => {
            let aabb = Rect::from_two_pos(from, to);
            let aabb_e = aabb.expand(threshold);
            if !aabb_e.contains(pos) {
                return false;
            }

            let a = to - from;
            let b = pos - from;

            let dot = b.dot(a);
            let dist2 = b.length_sq() - dot * dot / a.length_sq();

            dist2 < threshold * threshold
        }
        WireStyle::Bezier3 => {
            let [a, b, _, _, c, d] = wire_bezier_5(frame_size, from, to);
            let points = [a, b, c, d];
            hit_bezier_3(&points, pos, threshold)
        }
        WireStyle::Bezier5 => {
            let points = wire_bezier_5(frame_size, from, to);
            hit_bezier_5(&points, pos, threshold)
        }
        WireStyle::AxisAligned { corner_radius } => {
            hit_axis_aligned(pos, corner_radius, frame_size, from, to, threshold)
        }
    }
}

#[inline]
fn bezier_arc_length_upper_bound(points: &[Pos2]) -> f32 {
    let mut size = 0.0;
    for i in 1..points.len() {
        size += (points[i] - points[i - 1]).length();
    }
    size
}

fn bezier_hit_samples_number(points: &[Pos2], threshold: f32) -> usize {
    let arc_length = bezier_arc_length_upper_bound(points);

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    ((arc_length / threshold).ceil().max(0.0) as usize)
}

fn bezier_derivative_3(points: &[Pos2; 4]) -> [Pos2; 3] {
    let [p0, p1, p2, p3] = *points;

    let factor = 3.0;

    [
        (factor * (p1 - p0)).to_pos2(),
        (factor * (p2 - p1)).to_pos2(),
        (factor * (p3 - p2)).to_pos2(),
    ]
}

fn bezier_derivative_5(points: &[Pos2; 6]) -> [Pos2; 5] {
    let [p0, p1, p2, p3, p4, p5] = *points;

    let factor = 5.0;

    [
        (factor * (p1 - p0)).to_pos2(),
        (factor * (p2 - p1)).to_pos2(),
        (factor * (p3 - p2)).to_pos2(),
        (factor * (p4 - p3)).to_pos2(),
        (factor * (p5 - p4)).to_pos2(),
    ]
}

fn bezier_draw_samples_number_3(points: &[Pos2; 4], threshold: f32) -> usize {
    let d = bezier_derivative_3(points);

    lower_bound(2, MAX_CURVE_SAMPLES, |n| {
        let mut prev = points[0];
        for i in 1..n {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / (n - 1) as f32;
            let next = sample_bezier(points, t);

            let m = t - 0.5 / (n - 1) as f32;

            // Compare absolute error of mid point
            let mid_line = ((prev.to_vec2() + next.to_vec2()) * 0.5).to_pos2();
            let mid_curve = sample_bezier(points, m);

            let error_sq = (mid_curve - mid_line).length_sq();
            if error_sq > threshold * threshold {
                return false;
            }

            // Compare angular error of mid point
            let mid_line_dx = next.x - prev.x;
            let mid_line_dy = next.y - prev.y;

            let line_w = f32::hypot(mid_line_dx, mid_line_dy);

            let d_curve = sample_bezier(&d, m);
            let mid_curve_dx = d_curve.x;
            let mid_curve_dy = d_curve.y;

            let curve_w = f32::hypot(mid_curve_dx, mid_curve_dy);

            let error = f32::max(
                (mid_curve_dx / curve_w * line_w - mid_line_dx).abs(),
                (mid_curve_dy / curve_w * line_w - mid_line_dy).abs(),
            );
            if error > threshold * 2.0 {
                return false;
            }

            prev = next;
        }

        true
    })
}

fn bezier_draw_samples_number_5(points: &[Pos2; 6], threshold: f32) -> usize {
    let d = bezier_derivative_5(points);

    lower_bound(2, MAX_CURVE_SAMPLES, |n| {
        let mut prev = points[0];
        for i in 1..n {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / (n - 1) as f32;
            let next = sample_bezier(points, t);

            let m = t - 0.5 / (n - 1) as f32;

            // Compare absolute error of mid point
            let mid_line = ((prev.to_vec2() + next.to_vec2()) * 0.5).to_pos2();
            let mid_curve = sample_bezier(points, m);

            let error_sq = (mid_curve - mid_line).length_sq();
            if error_sq > threshold * threshold {
                return false;
            }

            // Compare angular error of mid point
            let mid_line_dx = next.x - prev.x;
            let mid_line_dy = next.y - prev.y;

            let line_w = f32::hypot(mid_line_dx, mid_line_dy);

            let d_curve = sample_bezier(&d, m);
            let mid_curve_dx = d_curve.x;
            let mid_curve_dy = d_curve.y;

            let curve_w = f32::hypot(mid_curve_dx, mid_curve_dy);

            let error = f32::max(
                (mid_curve_dx / curve_w * line_w - mid_line_dx).abs(),
                (mid_curve_dy / curve_w * line_w - mid_line_dy).abs(),
            );
            if error > threshold * 2.0 {
                return false;
            }

            prev = next;
        }

        true
    })
}

#[derive(Default)]
struct WiresCache {
    generation: u32,
    bezier_3: HashMap<(Id, Option<Wire>), (u32, [Pos2; 4], f32, Vec<Pos2>)>,
    bezier_5: HashMap<(Id, Option<Wire>), (u32, [Pos2; 6], f32, Vec<Pos2>)>,
}

impl CacheTrait for WiresCache {
    fn update(&mut self) {
        self.bezier_3
            .retain(|_, (generation, _, _, _)| *generation == self.generation);
        self.bezier_5
            .retain(|_, (generation, _, _, _)| *generation == self.generation);

        self.generation = self.generation.wrapping_add(1);
    }

    fn len(&self) -> usize {
        self.bezier_3.len() + self.bezier_5.len()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl WiresCache {
    pub fn get_3(
        &mut self,
        snarl_id: Id,
        wire: Option<Wire>,
        points: &[Pos2; 4],
        threshold: f32,
    ) -> Vec<Pos2> {
        let (generation, cached_points, cached_threshold, cached_line) = self
            .bezier_3
            .entry((snarl_id, wire))
            .or_insert_with(|| (0, [Pos2::ZERO; 4], threshold, Vec::new()));

        *generation = self.generation;

        if cached_points == points && *cached_threshold == threshold {
            return cached_line.clone();
        }

        // dbg!("Calculating new bezier 3");

        let samples = bezier_draw_samples_number_3(points, threshold);

        let line = (0..samples)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 / (samples - 1) as f32;
                sample_bezier(points, t)
            })
            .collect::<Vec<Pos2>>();

        *cached_points = *points;
        *cached_threshold = threshold;
        *cached_line = line.clone();

        line
    }

    pub fn get_5(
        &mut self,
        snarl_id: Id,
        wire: Option<Wire>,
        points: &[Pos2; 6],
        threshold: f32,
    ) -> Vec<Pos2> {
        let (generation, cached_points, cached_threshold, cached_line) = self
            .bezier_5
            .entry((snarl_id, wire))
            .or_insert_with(|| (0, [Pos2::ZERO; 6], threshold, Vec::new()));

        *generation = self.generation;

        if cached_points == points && *cached_threshold == threshold {
            return cached_line.clone();
        }

        // dbg!("Calculating new bezier 5");

        let samples = bezier_draw_samples_number_5(points, threshold);

        let line = (0..samples)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 / (samples - 1) as f32;
                sample_bezier(points, t)
            })
            .collect::<Vec<Pos2>>();

        *cached_points = *points;
        *cached_threshold = threshold;
        *cached_line = line.clone();

        line
    }
}

#[inline(never)]
fn draw_bezier_3(
    ui: &Ui,
    snarl_id: Id,
    wire: Option<Wire>,
    points: &[Pos2; 4],
    stroke: Stroke,
    threshold: f32,
    shapes: &mut Vec<Shape>,
) {
    let bb = Rect::from_points(points);
    if !ui.is_rect_visible(bb) {
        return;
    }

    let line = ui.memory_mut(|m| {
        m.caches
            .cache::<WiresCache>()
            .get_3(snarl_id, wire, points, threshold)
    });

    shapes.push(Shape::line(line, stroke));

    // {
    //     let samples = bezier_draw_samples_number_3(points, threshold);
    //     // dbg!(samples, bezier_hit_samples_number(points, threshold));
    //     shapes.push(Shape::line(
    //         points.to_vec(),
    //         Stroke::new(1.0, Color32::PLACEHOLDER),
    //     ));

    //     let samples = 100;
    //     shapes.push(Shape::line(
    //         (0..samples)
    //             .map(|i| {
    //                 #[allow(clippy::cast_precision_loss)]
    //                 let t = i as f32 / (samples - 1) as f32;
    //                 sample_bezier(points, t)
    //             })
    //             .collect(),
    //         Stroke::new(1.0, Color32::PLACEHOLDER),
    //     ));
    // }
}

fn draw_bezier_5(
    ui: &Ui,
    snarl_id: Id,
    wire: Option<Wire>,
    points: &[Pos2; 6],
    stroke: Stroke,
    threshold: f32,
    shapes: &mut Vec<Shape>,
) {
    let bb = Rect::from_points(points);
    if !ui.is_rect_visible(bb) {
        return;
    }

    let line = ui.memory_mut(|m| {
        m.caches
            .cache::<WiresCache>()
            .get_5(snarl_id, wire, points, threshold)
    });

    shapes.push(Shape::line(line, stroke));

    // {
    //     let samples = bezier_draw_samples_number_5(points, threshold);
    //     // dbg!(samples, bezier_hit_samples_number(points, threshold));
    //     shapes.push(Shape::line(
    //         points.to_vec(),
    //         Stroke::new(1.0, Color32::PLACEHOLDER),
    //     ));

    //     let samples = 100;
    //     shapes.push(Shape::line(
    //         (0..samples)
    //             .map(|i| {
    //                 #[allow(clippy::cast_precision_loss)]
    //                 let t = i as f32 / (samples - 1) as f32;
    //                 sample_bezier(points, t)
    //             })
    //             .collect(),
    //         Stroke::new(1.0, Color32::PLACEHOLDER),
    //     ));
    // }
}

// #[allow(clippy::let_and_return)]
fn sample_bezier(points: &[Pos2], t: f32) -> Pos2 {
    match *points {
        [] => unimplemented!(),
        [p0] => p0,
        [p0, p1] => p0.lerp(p1, t),
        [p0, p1, p2] => {
            let p0_0 = p0;
            let p1_0 = p1;
            let p2_0 = p2;

            let p0_1 = p0_0.lerp(p1_0, t);
            let p1_1 = p1_0.lerp(p2_0, t);

            let p0_2 = p0_1.lerp(p1_1, t);

            p0_2
        }
        [p0, p1, p2, p3] => {
            let p0_0 = p0;
            let p1_0 = p1;
            let p2_0 = p2;
            let p3_0 = p3;

            let p0_1 = p0_0.lerp(p1_0, t);
            let p1_1 = p1_0.lerp(p2_0, t);
            let p2_1 = p2_0.lerp(p3_0, t);

            sample_bezier(&[p0_1, p1_1, p2_1], t)
        }
        [p0, p1, p2, p3, p4] => {
            let p0_0 = p0;
            let p1_0 = p1;
            let p2_0 = p2;
            let p3_0 = p3;
            let p4_0 = p4;

            let p0_1 = p0_0.lerp(p1_0, t);
            let p1_1 = p1_0.lerp(p2_0, t);
            let p2_1 = p2_0.lerp(p3_0, t);
            let p3_1 = p3_0.lerp(p4_0, t);

            sample_bezier(&[p0_1, p1_1, p2_1, p3_1], t)
        }
        [p0, p1, p2, p3, p4, p5] => {
            let p0_0 = p0;
            let p1_0 = p1;
            let p2_0 = p2;
            let p3_0 = p3;
            let p4_0 = p4;
            let p5_0 = p5;

            let p0_1 = p0_0.lerp(p1_0, t);
            let p1_1 = p1_0.lerp(p2_0, t);
            let p2_1 = p2_0.lerp(p3_0, t);
            let p3_1 = p3_0.lerp(p4_0, t);
            let p4_1 = p4_0.lerp(p5_0, t);

            sample_bezier(&[p0_1, p1_1, p2_1, p3_1, p4_1], t)
        }
        _ => unimplemented!(),
    }
}

fn split_bezier_3(points: &[Pos2; 4], t: f32) -> [[Pos2; 4]; 2] {
    let [p0, p1, p2, p3] = *points;

    let p0_0 = p0;
    let p1_0 = p1;
    let p2_0 = p2;
    let p3_0 = p3;

    let p0_1 = p0_0.lerp(p1_0, t);
    let p1_1 = p1_0.lerp(p2_0, t);
    let p2_1 = p2_0.lerp(p3_0, t);

    let p0_2 = p0_1.lerp(p1_1, t);
    let p1_2 = p1_1.lerp(p2_1, t);

    let p0_3 = p0_2.lerp(p1_2, t);

    [[p0_0, p0_1, p0_2, p0_3], [p0_3, p1_2, p2_1, p3_0]]
}

fn hit_bezier_3(points: &[Pos2; 4], pos: Pos2, threshold: f32) -> bool {
    let aabb = Rect::from_points(points);

    let aabb_e = aabb.expand(threshold);
    if !aabb_e.contains(pos) {
        return false;
    }

    let samples = bezier_hit_samples_number(points, threshold);
    if samples > 8 {
        let [points1, points2] = split_bezier_3(points, 0.5);

        return hit_bezier_3(&points1, pos, threshold) || hit_bezier_3(&points2, pos, threshold);
    }

    let threshold_sq = threshold * threshold;

    for i in 0..samples {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / (samples - 1) as f32;
        let p = sample_bezier(points, t);
        if p.distance_sq(pos) <= threshold_sq {
            return true;
        }
    }

    false
}

fn split_bezier_5(points: &[Pos2; 6], t: f32) -> [[Pos2; 6]; 2] {
    let [p0, p1, p2, p3, p4, p5] = *points;

    let p0_0 = p0;
    let p1_0 = p1;
    let p2_0 = p2;
    let p3_0 = p3;
    let p4_0 = p4;
    let p5_0 = p5;

    let p0_1 = p0_0.lerp(p1_0, t);
    let p1_1 = p1_0.lerp(p2_0, t);
    let p2_1 = p2_0.lerp(p3_0, t);
    let p3_1 = p3_0.lerp(p4_0, t);
    let p4_1 = p4_0.lerp(p5_0, t);

    let p0_2 = p0_1.lerp(p1_1, t);
    let p1_2 = p1_1.lerp(p2_1, t);
    let p2_2 = p2_1.lerp(p3_1, t);
    let p3_2 = p3_1.lerp(p4_1, t);

    let p0_3 = p0_2.lerp(p1_2, t);
    let p1_3 = p1_2.lerp(p2_2, t);
    let p2_3 = p2_2.lerp(p3_2, t);

    let p0_4 = p0_3.lerp(p1_3, t);
    let p1_4 = p1_3.lerp(p2_3, t);

    let p0_5 = p0_4.lerp(p1_4, t);

    [
        [p0_0, p0_1, p0_2, p0_3, p0_4, p0_5],
        [p0_5, p1_4, p2_3, p3_2, p4_1, p5_0],
    ]
}

fn hit_bezier_5(points: &[Pos2; 6], pos: Pos2, threshold: f32) -> bool {
    let aabb = Rect::from_points(points);

    let aabb_e = aabb.expand(threshold);
    if !aabb_e.contains(pos) {
        return false;
    }

    let samples = bezier_hit_samples_number(points, threshold);
    if samples > 16 {
        let [points1, points2] = split_bezier_5(points, 0.5);

        return hit_bezier_5(&points1, pos, threshold) || hit_bezier_5(&points2, pos, threshold);
    }

    let threshold_sq = threshold * threshold;

    for i in 0..samples {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / (samples - 1) as f32;
        let p = sample_bezier(points, t);

        if p.distance_sq(pos) <= threshold_sq {
            return true;
        }
    }

    false
}

struct AxisAlignedWire {
    aabb: Rect,
    turns: usize,
    segments: [(Pos2, Pos2); 5],
    turn_centers: [Pos2; 4],
    turn_radii: [f32; 4],
}

#[allow(clippy::too_many_lines)]
fn wire_axis_aligned(
    corner_radius: f32,
    frame_size: f32,
    from: Pos2,
    to: Pos2,
    threshold: f32,
) -> AxisAlignedWire {
    let corner_radius = corner_radius.max(0.0);

    let half_height = f32::abs(from.y - to.y) / 2.0;
    let max_radius = (half_height / 2.0).min(corner_radius);

    let frame_size = frame_size.max(max_radius * 2.0);

    let zero_segment = (Pos2::ZERO, Pos2::ZERO);

    if from.x + frame_size <= to.x - frame_size {
        if f32::abs(from.y - to.y) < threshold {
            // Single segment case.
            AxisAlignedWire {
                aabb: Rect::from_two_pos(from, to),
                segments: [
                    (from, to),
                    zero_segment,
                    zero_segment,
                    zero_segment,
                    zero_segment,
                ],
                turns: 0,
                turn_centers: [Pos2::ZERO; 4],
                turn_radii: [f32::NAN; 4],
            }
        } else {
            // Two turns case.
            let mid_x = (from.x + to.x) / 2.0;
            let half_width = (to.x - from.x) / 2.0;

            let turn_radius = max_radius.min(half_width);

            let turn_vert_len = if from.y < to.y {
                turn_radius
            } else {
                -turn_radius
            };

            let segments = [
                (from, pos2(mid_x - turn_radius, from.y)),
                (
                    pos2(mid_x, from.y + turn_vert_len),
                    pos2(mid_x, to.y - turn_vert_len),
                ),
                (pos2(mid_x + turn_radius, to.y), to),
                zero_segment,
                zero_segment,
            ];

            let turn_centers = [
                pos2(mid_x - turn_radius, from.y + turn_vert_len),
                pos2(mid_x + turn_radius, to.y - turn_vert_len),
                Pos2::ZERO,
                Pos2::ZERO,
            ];

            let turn_radii = [turn_radius, turn_radius, f32::NAN, f32::NAN];

            AxisAlignedWire {
                aabb: Rect::from_two_pos(from, to),
                turns: 2,
                segments,
                turn_centers,
                turn_radii,
            }
        }
    } else {
        // Four turns case.
        let mid = (from.y + to.y) / 2.0;

        let right = from.x + frame_size;
        let left = to.x - frame_size;

        let half_width = f32::abs(right - left) / 2.0;

        let ends_turn_radius = max_radius;
        let middle_turn_radius = max_radius.min(half_width);

        let ends_turn_vert_len = if from.y < to.y {
            ends_turn_radius
        } else {
            -ends_turn_radius
        };

        let middle_turn_vert_len = if from.y < to.y {
            middle_turn_radius
        } else {
            -middle_turn_radius
        };

        let segments = [
            (from, pos2(right - ends_turn_radius, from.y)),
            (
                pos2(right, from.y + ends_turn_vert_len),
                pos2(right, mid - middle_turn_vert_len),
            ),
            (
                pos2(right - middle_turn_radius, mid),
                pos2(left + middle_turn_radius, mid),
            ),
            (
                pos2(left, mid + middle_turn_vert_len),
                pos2(left, to.y - ends_turn_vert_len),
            ),
            (pos2(left + ends_turn_radius, to.y), to),
        ];

        let turn_centers = [
            pos2(right - ends_turn_radius, from.y + ends_turn_vert_len),
            pos2(right - middle_turn_radius, mid - middle_turn_vert_len),
            pos2(left + middle_turn_radius, mid + middle_turn_vert_len),
            pos2(left + ends_turn_radius, to.y - ends_turn_vert_len),
        ];

        let turn_radii = [
            ends_turn_radius,
            middle_turn_radius,
            middle_turn_radius,
            ends_turn_radius,
        ];

        AxisAlignedWire {
            aabb: Rect::from_min_max(
                pos2(f32::min(left, from.x), f32::min(from.y, to.y)),
                pos2(f32::max(right, to.x), f32::max(from.y, to.y)),
            ),
            turns: 4,
            segments,
            turn_centers,
            turn_radii,
        }
    }
}

fn hit_axis_aligned(
    pos: Pos2,
    corner_radius: f32,
    frame_size: f32,
    from: Pos2,
    to: Pos2,
    threshold: f32,
) -> bool {
    let wire = wire_axis_aligned(corner_radius, frame_size, from, to, threshold);

    // Check AABB first
    if !wire.aabb.expand(threshold).contains(pos) {
        return false;
    }

    // Check all straight segments first
    // Number of segments is number of turns + 1
    for i in 0..wire.turns + 1 {
        let (start, end) = wire.segments[i];

        // Segments are always axis aligned
        // So we can use AABB for checking
        if Rect::from_two_pos(start, end)
            .expand(threshold)
            .contains(pos)
        {
            return true;
        }
    }

    // Check all turns
    for i in 0..wire.turns {
        if wire.turn_radii[i] > 0.0 {
            let turn = wire.turn_centers[i];
            let turn_aabb = Rect::from_two_pos(wire.segments[i].1, wire.segments[i + 1].0);
            if !turn_aabb.contains(pos) {
                continue;
            }

            // Avoid sqrt
            let dist2 = (turn - pos).length_sq();
            let min = wire.turn_radii[i] - threshold;
            let max = wire.turn_radii[i] + threshold;

            if dist2 <= max * max && dist2 >= min * min {
                return true;
            }
        }
    }

    false
}

fn turn_samples_number(radius: f32, threshold: f32) -> usize {
    let reference_size = radius * std::f32::consts::FRAC_PI_2;

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    ((reference_size / threshold).ceil().max(0.0) as usize).min(MAX_CURVE_SAMPLES / 4)
}

#[allow(clippy::too_many_arguments)]
fn draw_axis_aligned(
    corner_radius: f32,
    frame_size: f32,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
    threshold: f32,
    shapes: &mut Vec<Shape>,
) {
    let wire = wire_axis_aligned(corner_radius, frame_size, from, to, threshold);

    let mut path = Vec::new();

    for i in 0..wire.turns {
        // shapes.push(Shape::line_segment(
        //     [wire.segments[i].0, wire.segments[i].1],
        //     stroke,
        // ));

        // Draw segment first
        path.push(wire.segments[i].0);
        path.push(wire.segments[i].1);

        if wire.turn_radii[i] > 0.0 {
            let turn = wire.turn_centers[i];
            let samples = turn_samples_number(wire.turn_radii[i], stroke.width);

            let start = wire.segments[i].1;
            let end = wire.segments[i + 1].0;

            let sin_x = end.x - turn.x;
            let cos_x = start.x - turn.x;

            let sin_y = end.y - turn.y;
            let cos_y = start.y - turn.y;

            for j in 1..samples {
                #[allow(clippy::cast_precision_loss)]
                let a = std::f32::consts::FRAC_PI_2 * (j as f32 / samples as f32);

                let (sin_a, cos_a) = a.sin_cos();

                let point: Pos2 = pos2(
                    turn.x + sin_x * sin_a + cos_x * cos_a,
                    turn.y + sin_y * sin_a + cos_y * cos_a,
                );
                path.push(point);
            }
        }
    }

    // shapes.push(Shape::line_segment(
    //     [wire.segments[wire.turns].0, wire.segments[wire.turns].1],
    //     stroke,
    // ));

    path.push(wire.segments[wire.turns].0);
    path.push(wire.segments[wire.turns].1);

    let shape = Shape::Path(PathShape {
        points: path,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: stroke.into(),
    });

    shapes.push(shape);
}

fn lower_bound(min: usize, max: usize, f: impl Fn(usize) -> bool) -> usize {
    let mut min = min;
    let mut max = max;

    while min < max {
        let mid = (min + max) / 2;
        if f(mid) {
            max = mid;
        } else {
            min = mid + 1;
        }
    }

    min

    // for i in min..max {
    //     if f(i) {
    //         return i;
    //     }
    // }
    // max
}
