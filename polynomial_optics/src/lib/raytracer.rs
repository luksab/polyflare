use cgmath::{num_traits::Pow, prelude::*, Vector2, Vector3};
use tiny_skia::{Color, Pixmap};

/// ## A ray at a plane in the lens system
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// origin of the Ray, 0 in the optical axis
    pub o: cgmath::Vector3<f64>,
    /// direction of the Ray, 0 if in the path of the optical axis, is a unit vector
    pub d: cgmath::Vector3<f64>,
    pub strength: f64,
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            o: Vector3::new(0., 0., 0.),
            d: Vector3::new(0., 0., 1.),
            strength: 1.,
        }
    }
}

impl Ray {
    fn intersect(&self, plane: f64) -> (f64, f64) {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        let intersect = self.o + self.d * num_z;
        (intersect.x, intersect.y)
    }

    fn mov_plane(&mut self, plane: f64) {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        self.o += self.d * num_z;
    }
}

/// ## Properties of a particular glass
/// saves ior and coating
#[derive(Debug, Clone, Copy)]
pub struct Glass {
    /// ior vs air
    pub ior: f64,
    /// coating - modifies wavelength
    pub coating: (),
    pub entry: bool,
    pub spherical: bool,
}

/// # One element in a lens system
/// ```
/// # use polynomial_optics::raytracer::*;
/// let glass = polynomial_optics::Glass {
///     ior: 1.5,
///     coating: (),
/// };
/// let element = Element{
///    radius: 3.,
///    glass,
///    position: 2.,
///    entry: true,
///    spherical: true,     
/// };
/// let mut ray = Ray::default();
///
/// println!("space: {:?}", element);
/// println!("ray: {:?}", ray);
///
/// let ray2 = ray.propagate(&element);
/// println!("propagated ray: {:?}", ray2);
///
/// ```
#[derive(Debug)]
pub struct Element {
    /// One optical interface
    pub radius: f64,
    pub position: f64,
    pub properties: Properties,
}

#[derive(Debug)]
pub enum Properties {
    Glass(Glass),
    Aperture(u32),
}

impl Ray {
    pub fn new(o: Vector3<f64>, d: Vector3<f64>) -> Ray {
        Ray {
            o,
            d,
            ..Default::default()
        }
    }

    fn fresnel_r(t1: f64, t2: f64, n1: f64, n2: f64) -> f64 {
        let s = 0.5 * ((n1 * t1.cos() - n2 * t2.cos()) / (n1 * t1.cos() + n2 * t2.cos())).pow(2);
        let p = 0.5 * ((n1 * t2.cos() - n2 * t1.cos()) / (n1 * t2.cos() + n2 * t1.cos())).pow(2);

        return s + p;
    }

    fn propagate_element(
        &mut self,
        radius: &f64,
        glass: &Glass,
        position: f64,
        reflect: bool,
        entry: bool,
        cylindrical: bool,
    ) {
        let intersection = if cylindrical {
            // cylindrical: x is not affected by curvature

            // c: center of the lens surface if interpreted as an entire sphere
            let c = Vector2::new(
                0.,
                if entry {
                    position + *radius
                } else {
                    position - *radius
                },
            );
            let o: Vector2<f64> = Vector2 {
                x: self.o.y,
                y: self.o.z,
            };
            let d: Vector2<f64> = Vector2 {
                x: self.d.y,
                y: self.d.z,
            }
            .normalize();
            let delta: f64 = d.dot(o - c).pow(2) - ((o - c).magnitude().pow(2) - radius.pow(2));

            let d1 = -(d.dot(o - c)) - delta.sqrt();
            let d2 = -(d.dot(o - c)) + delta.sqrt();

            if (entry == (self.d.z > 0.)) == (radius > &0.) {
                self.o + self.d * d1
            } else {
                self.o + self.d * d2
            }
        } else {
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
            let d2 = -(self.d.dot(self.o - c)) + delta.sqrt();

            if (entry == (self.d.z > 0.)) == (radius > &0.) {
                self.o + self.d * d1
            } else {
                self.o + self.d * d2
            }
        };

        self.o = intersection;

        let normal = if cylindrical {
            let c = Vector2::new(
                0.,
                if entry {
                    position + *radius
                } else {
                    position - *radius
                },
            );
            let intersection: Vector2<f64> = Vector2 {
                x: intersection.y,
                y: intersection.z,
            }
            .normalize();

            let normal2d = intersection - c;

            let intersection = Vector3 {
                x: 0.0,
                y: normal2d.x,
                z: normal2d.y,
            };

            if (entry == (self.d.z > 0.)) == (radius > &0.) {
                (intersection).normalize()
            } else {
                -(intersection).normalize()
            }
        } else {
            let c = Vector3::new(
                0.,
                0.,
                if entry {
                    position + *radius
                } else {
                    position - *radius
                },
            );
            if (entry == (self.d.z > 0.)) == (radius > &0.) {
                (intersection - c).normalize()
            } else {
                -(intersection - c).normalize()
            }
        };

        if reflect {
            let d_in = self.d;

            self.d = self.d - 2.0 * normal.dot(self.d) * normal;

            self.strength *= Ray::fresnel_r(
                d_in.angle(normal).0,
                self.d.angle(-normal).0,
                if entry == (self.d.z > 0.) {
                    glass.ior
                } else {
                    1.0
                },
                if entry == (self.d.z > 0.) {
                    1.0
                } else {
                    glass.ior
                },
            );
        } else {
            let eta = if entry { 1.0 / glass.ior } else { glass.ior };

            // from https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/refract.xhtml
            let k = 1.0 - eta * eta * (1.0 - normal.dot(self.d) * normal.dot(self.d));

            let d_in = self.d;

            if k < 0.0 {
                // total reflection
                // println!("total reflection");
                self.d *= 0.0; // or genDType(0.0)
            } else {
                self.d = eta * self.d - (eta * normal.dot(self.d) + k.sqrt()) * normal;
            }

            self.strength *= 1.0
                - Ray::fresnel_r(
                    d_in.angle(-normal).0,
                    self.d.angle(-normal).0,
                    if entry { 1.0 } else { glass.ior },
                    if entry { glass.ior } else { 1.0 },
                );
        }
    }

    // let tpi: f32 = 6.283185307179586;
    fn clip_poly(&mut self, pos: f64, num_edge: u32, size: f64) {
        self.mov_plane(pos);

        let mut clipped = false;
        for i in 0..num_edge {
            let part = i as f64 * std::f64::consts::TAU / (num_edge as f64);
            let dir = Vector2 {
                x: part.cos(),
                y: part.sin(),
            };

            let dist = dir.dot(self.o.xy());
            clipped = clipped || (dist > size);
        }
        if clipped {
            self.d *= 0.;
        }
    }

    /// propagate a ray through an element
    ///
    pub fn propagate(&mut self, element: &Element) {
        match element.properties {
            Properties::Glass(glass) => self.propagate_element(
                &element.radius,
                &glass,
                element.position,
                false,
                glass.entry,
                !glass.entry,
            ),
            Properties::Aperture(properties) => {
                self.clip_poly(element.position, properties, element.radius)
            }
        };
    }

    /// reflect a Ray from an element
    ///
    pub fn reflect(&mut self, element: &Element) {
        match element.properties {
            Properties::Glass(glass) => self.propagate_element(
                &element.radius,
                &glass,
                element.position,
                true,
                glass.entry,
                !glass.entry,
            ),
            Properties::Aperture(properties) => {
                self.clip_poly(element.position, properties, element.radius)
            }
        };
    }
}

#[derive(Debug)]
pub struct Lens {
    pub elements: Vec<Element>,
}

impl Lens {
    pub fn new(elements: Vec<Element>) -> Self {
        Self { elements }
    }

    /// get elements in form:
    /// ```
    /// struct Element {
    ///   position1: f32,
    ///   radius1  : f32,
    ///   position2: f32,
    ///   radius2  : f32,
    /// };
    /// ```
    /// only works if elements are entry and exit alternatively
    pub fn get_elements_buffer(&self) -> Vec<f32> {
        let mut elements = vec![];

        for element in &self.elements {
            match element.properties {
                Properties::Glass(glass) => {
                    match glass.entry {
                        true => elements.push((element.position + element.radius) as f32),
                        false => elements.push((element.position - element.radius) as f32),
                    }
                    elements.push(element.radius as f32);
                }
                Properties::Aperture(aperture) => (), //TODO: render Aperture
            }
        }
        elements
    }

    /// get elements in form:
    /// ```
    /// struct Element {
    ///   radius: f32;
    ///   glass: Glass;
    ///   position: f32;
    ///   entry: bool;
    ///   spherical: bool;
    /// };
    /// struct Glass {
    /// ior vs air
    ///   ior: f32;
    ///   /// coating - modifies wavelength
    ///   // coating: ();
    /// };
    /// ```
    /// only works if elements are entry and exit alternatively
    pub fn get_rt_elements_buffer(&self) -> Vec<f32> {
        let mut elements = vec![];

        for element in &self.elements {
            match element.properties {
                Properties::Glass(glass) => {
                    elements.push(element.radius as f32);
                    elements.push(glass.ior as f32);
                    elements.push(element.position as f32);
                    elements.push(glass.entry as i32 as f32);
                    elements.push(glass.spherical as i32 as f32);
                }
                Properties::Aperture(aperture) => {
                    elements.push(element.radius as f32);
                    elements.push(aperture as f32);
                    elements.push(element.position as f32);
                    elements.push(2. as f32);
                    elements.push(2. as f32);
                }
            }
        }

        elements
    }

    /// draws z y of the distance
    pub fn draw_rays(pixmap: &mut Pixmap, ray1: &Ray, ray2: &Ray) {
        let mut paint = tiny_skia::Paint::default();
        //paint.set_color(self.color);
        paint.set_color(Color::from_rgba8(
            127,
            127,
            255,
            (255.0 * ray1.strength.sqrt()) as u8,
        ));
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
        stroke.width = 1.0;
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
        let num_rays = 500;
        let width = 2.0;
        for ray_num in 0..num_rays {
            // for i in 0..self.elements.len() {
            //     for j in i..self.elements.len() {
            for i in 0..=0 {
                for j in 1..=1 {
                    let mut ray = Ray::new(
                        Vector3 {
                            x: 0.0,
                            y: ray_num as f64 / (num_rays as f64) * width - width / 2.,
                            z: -5.,
                        },
                        Vector3 {
                            x: 0.0,
                            y: 0.2,
                            z: 1.0,
                        }
                        .normalize(),
                    );
                    let mut one = ray;
                    for (ele, element) in self.elements.iter().enumerate() {
                        // if we iterated through all elements up to
                        // the first reflection point

                        if ele == j {
                            // reflect at the first element,
                            // which is further down the optical path
                            ray.reflect(element);
                            Lens::draw_rays(pixmap, &one, &ray);
                            one = ray;
                            // propagate backwards through system
                            // until the second reflection
                            for k in (i + 1..j).rev() {
                                ray.propagate(&self.elements[k]);
                                Lens::draw_rays(pixmap, &one, &ray);
                                one = ray;
                            }
                            ray.reflect(&self.elements[i]);
                            Lens::draw_rays(pixmap, &one, &ray);
                            one = ray;
                            for k in i + 1..j {
                                ray.propagate(&self.elements[k]);
                                Lens::draw_rays(pixmap, &one, &ray);
                                one = ray;
                            }
                            println!("strength: {}", ray.strength);
                        } else {
                            ray.propagate(element);
                            Lens::draw_rays(pixmap, &one, &ray);
                            one = ray;
                        }
                    }
                    ray.o += ray.d * 100.;
                    Lens::draw_rays(pixmap, &one, &ray);
                }
            }
            // let mut ray = Ray::new(
            //     Vector3 {
            //         x: 0.0,
            //         y: ray_num as f64 / (num_rays as f64) * width - width / 2.,
            //         z: -5.,
            //     },
            //     Vector3 {
            //         x: 0.0,
            //         y: 0.0,
            //         z: 1.0,
            //     },
            // );
            // let mut one = ray;
            // for element in &self.elements {
            //     ray.propagate(element);
            //     Lens::draw_rays(pixmap, &one, &ray);
            //     one = ray;
            // }
            // ray.propagate(&Element::Space(100.));
            // Lens::draw_rays(pixmap, &one, &ray);
        }
    }

    pub fn get_rays(
        &self,
        num_rays: u32,
        center_pos: Vector3<f64>,
        direction: Vector3<f64>,
        draw_mode: u32,
        which_ghost: u32,
    ) -> Vec<f32> {
        let mut rays = vec![];

        let width = 2.0;
        let mut old_strength;
        for ray_num in 0..num_rays {
            if draw_mode & 1 > 0 {
                let mut ghost_num = 0;
                for i in 0..self.elements.len() - 1 {
                    for j in i + 1..self.elements.len() {
                        ghost_num += 1;
                        if ghost_num == which_ghost || which_ghost == 0 {
                            // make new ray
                            let mut pos = center_pos;
                            pos.y += ray_num as f64 / (num_rays as f64) * width - width / 2.;
                            let mut ray = Ray::new(pos, direction);
                            rays.push(ray.o.z);
                            rays.push(ray.o.y);
                            rays.push(ray.strength);
                            old_strength = ray.strength;

                            for (ele, element) in self.elements.iter().enumerate() {
                                // if we iterated through all elements up to
                                // the first reflection point

                                if ele == j {
                                    // reflect at the first element,
                                    // which is further down the optical path
                                    ray.reflect(element);
                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(old_strength);
                                    old_strength = ray.strength;

                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(ray.strength);
                                    // propagate backwards through system
                                    // until the second reflection
                                    for k in (i + 1..j).rev() {
                                        ray.propagate(&self.elements[k]);
                                        rays.push(ray.o.z);
                                        rays.push(ray.o.y);
                                        rays.push(old_strength);
                                        old_strength = ray.strength;

                                        rays.push(ray.o.z);
                                        rays.push(ray.o.y);
                                        rays.push(ray.strength);
                                    }
                                    ray.reflect(&self.elements[i]);
                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(old_strength);
                                    old_strength = ray.strength;

                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(ray.strength);
                                    for k in i + 1..=j {
                                        ray.propagate(&self.elements[k]);
                                        rays.push(ray.o.z);
                                        rays.push(ray.o.y);
                                        rays.push(old_strength);
                                        old_strength = ray.strength;

                                        rays.push(ray.o.z);
                                        rays.push(ray.o.y);
                                        rays.push(ray.strength);
                                    }
                                    // println!("strength: {}", ray.strength);
                                } else {
                                    ray.propagate(element);
                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(old_strength);
                                    old_strength = ray.strength;

                                    rays.push(ray.o.z);
                                    rays.push(ray.o.y);
                                    rays.push(ray.strength);
                                }
                            }
                            ray.o += ray.d * 100.;
                            rays.push(ray.o.z);
                            rays.push(ray.o.y);
                            rays.push(ray.strength);
                        }
                    }
                }
            }
            if draw_mode & 2 > 0 {
                let mut pos = center_pos;
                pos.y += ray_num as f64 / (num_rays as f64) * width - width / 2.;
                let mut ray = Ray::new(pos, direction);
                rays.push(ray.o.z);
                rays.push(ray.o.y);
                rays.push(ray.strength);
                old_strength = ray.strength;
                for element in &self.elements {
                    ray.propagate(element);
                    rays.push(ray.o.z);
                    rays.push(ray.o.y);
                    rays.push(old_strength);
                    old_strength = ray.strength;

                    rays.push(ray.o.z);
                    rays.push(ray.o.y);
                    rays.push(ray.strength);
                }
                ray.o += ray.d * 100.;
                rays.push(ray.o.z);
                rays.push(ray.o.y);
                rays.push(ray.strength);
            }
        }
        rays.into_iter().map(|num| num as f32).collect()
    }

    pub fn get_paths(
        &self,
        num_rays: u32,
        center_pos: Vector3<f64>,
        direction: Vector3<f64>,
        draw_mode: u32,
        which_ghost: u32,
    ) -> Vec<Vec<Ray>> {
        let mut rays = vec![];

        let width = 2.0;
        for ray_num_x in 0..num_rays {
            for ray_num_y in 0..num_rays {
                if draw_mode & 1 > 0 {
                    let mut ghost_num = 0;
                    for i in 0..self.elements.len() - 1 {
                        for j in i + 1..self.elements.len() {
                            ghost_num += 1;
                            if ghost_num == which_ghost || which_ghost == 0 {
                                let mut ray_collection = vec![];
                                // make new ray
                                let mut pos = center_pos;
                                pos.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                                pos.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                                let mut ray = Ray::new(pos, direction);
                                ray_collection.push(ray);

                                for (ele, element) in self.elements.iter().enumerate() {
                                    // if we iterated through all elements up to
                                    // the first reflection point

                                    if ele == j {
                                        // reflect at the first element,
                                        // which is further down the optical path
                                        ray.reflect(element);
                                        ray_collection.push(ray);

                                        // propagate backwards through system
                                        // until the second reflection
                                        for k in (i + 1..j).rev() {
                                            ray.propagate(&self.elements[k]);
                                            ray_collection.push(ray);
                                        }
                                        ray.reflect(&self.elements[i]);
                                        ray_collection.push(ray);

                                        for k in i + 1..=j {
                                            ray.propagate(&self.elements[k]);
                                            ray_collection.push(ray);
                                        }
                                        // println!("strength: {}", ray.strength);
                                    } else {
                                        ray.propagate(element);
                                        ray_collection.push(ray);
                                    }
                                }
                                ray.o += ray.d * 100.;
                                ray_collection.push(ray);

                                // only return rays that have made it through
                                if ray.d.magnitude() > 0. {
                                    rays.push(ray_collection);
                                }
                            }
                        }
                    }
                }
                if draw_mode & 2 > 0 {
                    let mut pos = center_pos;
                    pos.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                    pos.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                    let mut ray = Ray::new(pos, direction);
                    let mut ray_collection = vec![];
                    ray_collection.push(ray);
                    for element in &self.elements {
                        ray.propagate(element);
                        ray_collection.push(ray);
                    }
                    ray.o += ray.d * 100.;
                    ray_collection.push(ray);

                    // only return rays that have made it through
                    if ray.d.magnitude() > 0. {
                        rays.push(ray_collection);
                    }
                }
            }
        }
        rays
    }

    pub fn get_dots(
        &self,
        num_rays: u32,
        center_pos: Vector3<f64>,
        direction: Vector3<f64>,
        draw_mode: u32,
        which_ghost: u32,
        sensor_pos: f64,
    ) -> Vec<f32> {
        // let rays = self.get_paths(
        //     num::integer::Roots::sqrt(&(num_rays * 1000)),
        //     center_pos,
        //     direction,
        //     draw_mode,
        //     which_ghost,
        // );

        let mut rays = vec![];

        let width = 2.0;
        for ray_num_x in 0..num_rays {
            for ray_num_y in 0..num_rays {
                if draw_mode & 1 > 0 {
                    let mut ghost_num = 0;
                    for i in 0..self.elements.len() - 1 {
                        for j in i + 1..self.elements.len() {
                            ghost_num += 1;
                            if ghost_num == which_ghost || which_ghost == 0 {
                                // make new ray
                                let mut pos = center_pos;
                                pos.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                                pos.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                                let mut ray = Ray::new(pos, direction);

                                for (ele, element) in self.elements.iter().enumerate() {
                                    // if we iterated through all elements up to
                                    // the first reflection point

                                    if ele == j {
                                        // reflect at the first element,
                                        // which is further down the optical path
                                        ray.reflect(element);

                                        // propagate backwards through system
                                        // until the second reflection
                                        for k in (i + 1..j).rev() {
                                            ray.propagate(&self.elements[k]);
                                        }
                                        ray.reflect(&self.elements[i]);

                                        for k in i + 1..=j {
                                            ray.propagate(&self.elements[k]);
                                        }
                                        // println!("strength: {}", ray.strength);
                                    } else {
                                        ray.propagate(element);
                                    }
                                }
                                ray.o += ray.d * 100.;

                                // only return rays that have made it through
                                if ray.d.magnitude() > 0. {
                                    rays.push(ray);
                                }
                            }
                        }
                    }
                }
                if draw_mode & 2 > 0 {
                    let mut pos = center_pos;
                    pos.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                    pos.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                    let mut ray = Ray::new(pos, direction);
                    for element in &self.elements {
                        ray.propagate(element);
                    }
                    ray.o += ray.d * 100.;

                    // only return rays that have made it through
                    if ray.d.magnitude() > 0. {
                        rays.push(ray);
                    }
                }
            }
        }

        // println!(
        //     "in: {} out:{} percent: {}",
        //     &num_rays * 100,
        //     rays.len(),
        //     rays.len() as f64 / (&num_rays * 100) as f64 * 100.
        // );

        let mut dots = vec![];
        for ray in rays {
            let intersection = ray.intersect(sensor_pos);
            dots.push(intersection.0 as f32);
            dots.push(intersection.1 as f32);
            dots.push(ray.strength as f32);
        }
        dots
    }
}
