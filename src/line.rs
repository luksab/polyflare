use tiny_skia::{Color, LineCap, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

pub struct Line {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub dashed: bool,
    pub color: Color,
    pub width: f32,
}

impl Line {
    pub fn draw(&self, pixmap: &mut Pixmap) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(0, 127, 0, 200);
        paint.anti_alias = true;

        let path = {
            let mut pb = PathBuilder::new();
            pb.move_to(self.x1, self.y1);

            pb.line_to(self.x2, self.y2);
            pb.finish().unwrap()
        };

        let mut stroke = Stroke::default();
        stroke.width = self.width;
        stroke.line_cap = LineCap::Round;
        if self.dashed {
            stroke.dash = StrokeDash::new(vec![20.0, 40.0], 0.0);
        }

        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            ..Default::default()
        }
    }

    pub fn new_dashed(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            dashed: true,
            ..Default::default()
        }
    }
}

impl Default for Line {
    fn default() -> Self {
        Self {
            x1: Default::default(),
            y1: Default::default(),
            x2: Default::default(),
            y2: Default::default(),
            dashed: Default::default(),
            color: Color::from_rgba8(0, 127, 0, 200),
            width: 6.0,
        }
    }
}
