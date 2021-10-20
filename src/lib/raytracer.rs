use cgmath::{num_traits::Pow, prelude::*, Vector3};
use tiny_skia::{Color, Pixmap};

/// ## A ray at a plane in the lens system
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// origin of the Ray, 0 in the optical axis
    pub o: cgmath::Vector3<f64>,
    /// direction of the Ray, 0 if in the path of the optical axis, is a unit vector
    pub d: cgmath::Vector3<f64>,
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            o: Vector3::new(0., 0., 0.),
            d: Vector3::new(0., 0., 1.),
        }
    }
}

/// ## Properties of a particular glass
/// saves ior and coating
#[derive(Debug)]
pub struct Glass {
    /// ior vs air
    pub ior: f64,
    /// coating - modifies wavelength
    pub coating: (),
}

/// # One element in a lens system
/// ```
/// # use polynomial_optics::raytracer::*;
/// let space = Element::Space(1.0);
/// let mut ray = Ray::default();
///
/// println!("space: {:?}", space);
/// println!("ray: {:?}", ray);
///
/// let ray2 = space.propagate(ray);
/// println!("propagated ray: {:?}", ray2);
///
/// ```
#[derive(Debug)]
pub enum Element {
    /// One optical interface
    SphericalLensEntry {
        radius: f64,
        glass: Glass,
        position: f64,
    },
    SphericalLensExit {
        radius: f64,
        glass: Glass,
        position: f64,
    },
    SphericalLensReflexion {
        radius: f64,
        glass: Glass,
        position: f64,
        front: bool,
    },
    Space(f64),
}

impl Ray {
    pub fn new(o: Vector3<f64>, d: Vector3<f64>) -> Ray {
        Ray { o, d }
    }

    fn refract_lens(&mut self, radius: &f64, glass: &Glass, position: f64, entry: bool) {
        // c: center of the lens surface if interpreted as an entire sphere
        let c = Vector3::new(
            0.,
            0.,
            if entry {
                position + *radius
            } else {
                position - *radius
            },
        );
        let delta: f64 =
            self.d.dot(self.o - c).pow(2) - ((self.o - c).magnitude().pow(2) - radius.pow(2));

        let d1 = -(self.d.dot(self.o - c)) - delta.sqrt();
        // println!("d1: {}", d1);
        let d2 = -(self.d.dot(self.o - c)) + delta.sqrt();

        let intersection = if entry {
            self.o + self.d * d1
        } else {
            self.o + self.d * d2
        };

        self.o = intersection;

        let normal = if entry {
            (intersection - c).normalize()
        } else {
            -(intersection - c).normalize()
        };

        let eta = if entry { 1.0 / glass.ior } else { glass.ior };

        // from https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/refract.xhtml
        let k = 1.0 - eta * eta * (1.0 - normal.dot(self.d) * normal.dot(self.d));

        if k < 0.0 {
            // total reflection
            println!("total reflection");
            self.d *= 0.0; // or genDType(0.0)
        } else {
            self.d = eta * self.d - (eta * normal.dot(self.d) + k.sqrt()) * normal;
        }
    }

    fn reflect_lens(&mut self, radius: &f64, glass: &Glass, position: f64, entry: bool) {
        // c: center of the lens surface if interpreted as an entire sphere
        let c = Vector3::new(
            0.,
            0.,
            if entry {
                position + *radius
            } else {
                position - *radius
            },
        );
        let delta: f64 =
            self.d.dot(self.o - c).pow(2) - ((self.o - c).magnitude().pow(2) - radius.pow(2));

        let d1 = -(self.d.dot(self.o - c)) - delta.sqrt();
        // println!("d1: {}", d1);
        let d2 = -(self.d.dot(self.o - c)) + delta.sqrt();

        let intersection = if entry {
            self.o + self.d * d1
        } else {
            self.o + self.d * d2
        };

        self.o = intersection;

        let normal = if entry {
            (intersection - c).normalize()
        } else {
            -(intersection - c).normalize()
        };

        self.d = intersection - 2.0 * normal.dot(intersection) * normal;
    }

    /// propagate a ray through an element
    ///
    /// the z coordinate has to be relative to the element
    pub fn propagate(&mut self, element: &Element) {
        match element {
            Element::SphericalLensEntry {
                radius,
                glass,
                position,
            } => {
                // propagate by the distance between the first part of the lens
                // and the actual intersection
                self.refract_lens(radius, glass, *position, true);
                //ray.d = ray.d - 2.0 * (ray.d.dot(normal)) * normal;
            }
            Element::Space(space) => {
                self.o += self.d * *space;
            }
            Element::SphericalLensExit {
                radius,
                glass,
                position,
            } => self.refract_lens(radius, glass, *position, false),
            Element::SphericalLensReflexion {
                radius,
                glass,
                position,
                front,
            } => self.reflect_lens(radius, glass, *position, *front),
        }
    }
}

#[derive(Debug)]
pub struct Lens {
    elements: Vec<Element>,
}

impl Lens {
    pub fn new(elements: Vec<Element>) -> Self {
        Self { elements }
    }

    /// draws z y of the distance
    pub fn draw_rays(pixmap: &mut Pixmap, ray1: &Ray, ray2: &Ray) {
        let mut paint = tiny_skia::Paint::default();
        //paint.set_color(self.color);
        paint.set_color(Color::from_rgba8(127, 127, 255, 255));
        paint.anti_alias = true;

        let middle = ((pixmap.width() / 4) as f32, (pixmap.height() / 2) as f32);

        let scale = (pixmap.height() / 10) as f32;

        let path = {
            let mut pb = tiny_skia::PathBuilder::new();
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

        let mut stroke = tiny_skia::Stroke::default();
        stroke.width = 0.1;
        stroke.line_cap = tiny_skia::LineCap::Round;
        // if self.dashed {
        //     stroke.dash = tiny_skia::StrokeDash::new(vec![20.0, 40.0], 0.0);
        // }

        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    pub fn draw(&self, pixmap: &mut Pixmap) {
        let num_rays = 700;
        let width = 2.0;
        for i in 0..=num_rays {
            let mut ray = Ray::new(
                Vector3 {
                    x: 0.0,
                    y: i as f64 / (num_rays as f64) * width - width / 2.,
                    z: -5.,
                },
                Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
            );
            let mut one = ray;
            for element in &self.elements {
                ray.propagate(element);
                Lens::draw_rays(pixmap, &one, &ray);
                one = ray;
            }
            ray.propagate(&Element::Space(100.));
            Lens::draw_rays(pixmap, &one, &ray);
        }
    }
}
