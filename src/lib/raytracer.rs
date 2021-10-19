use cgmath::{num_traits::Pow, prelude::*, Vector3};

/// ## A ray at a plane in the lens system
#[derive(Debug)]
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
    },
    SphericalLensExit {
        radius: f64,
        glass: Glass,
    },
    Space(f64),
}

impl Ray {
    pub fn new(o: Vector3<f64>, d: Vector3<f64>) -> Ray {
        Ray { o, d }
    }

    fn refract_lens(&mut self, radius: &f64, glass: &Glass, entry: bool) {
        let offset = self.o.z;
        self.o.z = 0.;

        // c: center of the lens surface if interpreted as an entire sphere
        let c = Vector3::new(0., 0., *radius);
        let delta: f64 =
            self.d.dot(self.o - c).pow(2) - ((self.o - c).magnitude().pow(2) - radius.pow(2));

        let d1 = -(self.d.dot(self.o - c)) - delta.sqrt();
        println!("d1: {}", d1);
        let d2 = -(self.d.dot(self.o - c)) + delta.sqrt();

        let intersection = if entry {
            self.o + self.d * d1
        } else {
            self.o + self.d * d2
        };

        self.o = intersection;
        self.o.z += offset;

        let normal = (intersection - c).normalize();

        let eta = if entry { 1.0 / glass.ior } else { glass.ior };

        // from https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/refract.xhtml
        let k = 1.0 - eta * eta * (1.0 - normal.dot(self.d) * normal.dot(self.d));

        if k < 0.0 {
            // total reflexion
            println!("total reflexion");
            self.d *= 0.0; // or genDType(0.0)
        } else {
            self.d = eta * self.d - (eta * normal.dot(self.d) + k.sqrt()) * normal;
        }
    }

    /// propagate a ray through an element
    ///
    /// the z coordinate has to be relative to the element
    pub fn propagate(&mut self, element: &Element) {
        match element {
            Element::SphericalLensEntry { radius, glass } => {
                // propagate by the distance between the first part of the lens
                // and the actual intersection
                self.refract_lens(radius, glass, true);
                //ray.d = ray.d - 2.0 * (ray.d.dot(normal)) * normal;
            }
            Element::Space(space) => {
                self.o += self.d * *space;
            }
            Element::SphericalLensExit { radius, glass } => self.refract_lens(radius, glass, false),
        }
    }
}

#[derive(Debug)]
struct Lens {
    elements: Vec<Element>,
}
