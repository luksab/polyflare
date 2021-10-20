use polynomial_optics::Ray;
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
        paint.set_color(self.color);
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

    /// draws z y of the distance
    pub fn draw_ray(&self, pixmap: &mut Pixmap, ray: &Ray, distance: f64) {
        let mut paint = Paint::default();
        paint.set_color(self.color);
        paint.anti_alias = true;

        let middle = ((pixmap.width() / 4) as f32, (pixmap.height() / 2) as f32);

        let scale = (pixmap.height() / 10) as f32;

        let path = {
            let mut pb = PathBuilder::new();
            pb.move_to(
                middle.0 + scale * ray.o.z as f32,
                middle.1 + scale * ray.o.y as f32,
            );

            pb.line_to(
                middle.0 + scale * (ray.o.z + ray.d.z * distance) as f32,
                middle.1 + scale * (ray.o.y + ray.d.y * distance) as f32,
            );
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

    /// draws z y of the distance
    pub fn draw_vecs(&self, pixmap: &mut Pixmap, ray1: &Ray, ray2: &Ray) {
        let mut paint = Paint::default();
        paint.set_color(self.color);
        paint.anti_alias = true;

        let middle = ((pixmap.width() / 4) as f32, (pixmap.height() / 2) as f32);

        let scale = (pixmap.height() / 10) as f32;

        let path = {
            let mut pb = PathBuilder::new();
            pb.move_to(
                middle.0 + scale * ray1.o.z as f32,
                middle.1 + scale * ray1.o.y as f32,
            );

            pb.line_to(
                middle.0 + scale * (ray2.o.z) as f32,
                middle.1 + scale * (ray2.o.y) as f32,
            );
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

    /// draws z y of the distance
    pub fn draw_circle(&self, pixmap: &mut Pixmap, x: f32, y: f32, r: f32) {
        let mut paint = Paint::default();
        paint.set_color(self.color);
        paint.anti_alias = true;

        let middle = ((pixmap.width() / 4) as f32, (pixmap.height() / 2) as f32);

        let scale = (pixmap.height() / 10) as f32;

        let path = {
            let mut pb = PathBuilder::new();
            pb.move_to(middle.0 + scale * x as f32, middle.1 + scale * y as f32);

            pb.push_circle(
                middle.0 + scale * x as f32 + 2. * r * scale,
                middle.1 + scale * y as f32,
                r * scale,
            );
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
            width: 0.1,
        }
    }
}
