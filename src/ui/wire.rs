use egui::{epaint::PathShape, pos2, Color32, Pos2, Rect, Shape, Stroke, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireLayer {
    BehindNodes,
    AboveNodes,
}

/// Returns 6th degree bezier curve for the wire
fn wire_bezier(
    mut frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
) -> [Pos2; 6] {
    if upscale {
        frame_size = frame_size.max((from - to).length() / 4.0);
    }
    if downscale {
        frame_size = frame_size.min((from - to).length() / 4.0);
    }

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
        let t =
            (between - (to_2.y - from_2.y).abs()) / (frame_size * 2.0 - (to_2.y - from_2.y).abs());

        let mut middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let mut middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        if from_2.y >= to_2.y + frame_size {
            let u = (from_2.y - to_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x + (1.0 - u) * frame_size, from_2.y - frame_size * u);
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if from_2.y >= to_2.y {
            let u = (from_2.y - to_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x + u * frame_size, from_2.y + frame_size * (1.0 - u));
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y + frame_size {
            let u = (to_2.y - from_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(to_2.x - (1.0 - u) * frame_size, to_2.y - frame_size * u);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y {
            let u = (to_2.y - from_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(to_2.x - u * frame_size, to_2.y + frame_size * (1.0 - u));

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else {
            unreachable!();
        }

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size * 2.0 {
        let middle_1 = pos2(from_2.x, from_2.y - frame_size);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size {
        let t = (from_2.y - to_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x + (1.0 - t) * frame_size, from_2.y - frame_size * t);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y {
        let t = (from_2.y - to_2.y) / frame_size;

        let middle_1 = pos2(from_2.x + t * frame_size, from_2.y + frame_size * (1.0 - t));
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size * 2.0 {
        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x, to_2.y - frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size {
        let t = (to_2.y - from_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x - (1.0 - t) * frame_size, to_2.y - frame_size * t);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y {
        let t = (to_2.y - from_2.y) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x - t * frame_size, to_2.y + frame_size * (1.0 - t));

        [from, from_2, middle_1, middle_2, to_2, to]
    } else {
        unreachable!();
    }
}

pub fn draw_wire(
    ui: &mut Ui,
    shapes: &mut Vec<Shape>,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
) {
    let points = wire_bezier(frame_size, upscale, downscale, from, to);

    let bb = Rect::from_points(&points);
    if ui.is_rect_visible(bb) {
        draw_bezier(shapes, &points, stroke);
    }
}

pub fn hit_wire(
    pos: Pos2,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    threshold: f32,
) -> bool {
    let points = wire_bezier(frame_size, upscale, downscale, from, to);
    hit_bezier(pos, &points, threshold)
}

fn bezier_reference_size(points: &[Pos2; 6]) -> f32 {
    let [p0, p1, p2, p3, p4, p5] = *points;

    (p1 - p0).length()
        + (p2 - p1).length()
        + (p3 - p2).length()
        + (p4 - p3).length()
        + (p5 - p4).length()
}

const MAX_BEZIER_SAMPLES: usize = 100;

fn bezier_samples_number(points: &[Pos2; 6], threshold: f32) -> usize {
    let reference_size = bezier_reference_size(points);
    ((reference_size / threshold).ceil() as usize).min(MAX_BEZIER_SAMPLES)
}

fn draw_bezier(shapes: &mut Vec<Shape>, points: &[Pos2; 6], mut stroke: Stroke) {
    assert!(!points.is_empty());

    if stroke.width < 1.0 {
        stroke.color = stroke.color.gamma_multiply(stroke.width);
        stroke.width = 1.0;
    }

    let samples = bezier_samples_number(points, stroke.width);

    let mut path = Vec::new();

    for i in 0..samples {
        let t = i as f32 / (samples - 1) as f32;
        path.push(sample_bezier(points, t));
    }

    let shape = Shape::Path(PathShape {
        points: path,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke,
    });

    shapes.push(shape);
}

#[allow(clippy::let_and_return)]
fn sample_bezier(points: &[Pos2; 6], t: f32) -> Pos2 {
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

    p0_5
}

fn split_bezier(points: &[Pos2; 6], t: f32) -> [[Pos2; 6]; 2] {
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

fn hit_bezier(pos: Pos2, points: &[Pos2; 6], threshold: f32) -> bool {
    let aabb = Rect::from_points(points);

    if pos.x + threshold < aabb.left() {
        return false;
    }
    if pos.x - threshold > aabb.right() {
        return false;
    }
    if pos.y + threshold < aabb.top() {
        return false;
    }
    if pos.y - threshold > aabb.bottom() {
        return false;
    }

    let samples = bezier_samples_number(points, threshold);
    if samples > 16 {
        let [points1, points2] = split_bezier(points, 0.5);

        return hit_bezier(pos, &points1, threshold) || hit_bezier(pos, &points2, threshold);
    }

    for i in 0..samples {
        let t = i as f32 / (samples - 1) as f32;
        let p = sample_bezier(points, t);
        if (p - pos).length() < threshold {
            return true;
        }
    }

    false
}

pub fn mix_colors(a: Color32, b: Color32) -> Color32 {
    let [or, og, ob, oa] = a.to_array();
    let [ir, ig, ib, ia] = b.to_array();

    Color32::from_rgba_premultiplied(
        or / 2 + ir / 2,
        og / 2 + ig / 2,
        ob / 2 + ib / 2,
        oa / 2 + ia / 2,
    )
}
