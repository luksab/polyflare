use autodiff::*;
use std::{fs::OpenOptions, io::Write, path::Path};

use serde::{Deserialize, Serialize};

use cgmath::{num_traits::Pow, prelude::*, Vector2, Vector3};
use tiny_skia::{Color, Pixmap};

/// ## A ray at a plane in the lens system
#[derive(Debug, Clone, Copy)]
pub struct RayAutodiff {
    /// origin of the Ray, 0 in the optical axis
    pub o: cgmath::Vector3<F<f64, f64>>,
    /// wavelength in µm
    pub wavelength: F<f64, f64>,
    /// direction of the Ray, 0 if in the path of the optical axis, is a unit vector
    pub d: cgmath::Vector3<F<f64, f64>>,
    pub strength: F<f64, f64>,
}

impl Default for RayAutodiff {
    fn default() -> Self {
        Self {
            o: Vector3::new(F1::var(0.0), F1::var(0.0), F1::var(0.0)),
            d: Vector3::new(F1::var(0.0), F1::var(0.0), F1::var(1.0)),
            strength: F1::var(1.0),
            wavelength: F1::var(0.5),
        }
    }
}

impl RayAutodiff {
    fn intersect(&self, plane: f64) -> Vector2<F<f64, f64>> {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        let intersect = self.o + self.d * num_z;
        Vector2::new(intersect.x, intersect.y)
    }

    fn mov_plane(&mut self, plane: f64) {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        self.o += self.d * num_z;
    }

    pub fn get_rgb(&self) -> (u8, u8, u8) {
        Self::wave_length_to_rgb(self.wavelength)
    }

    /**
     * Taken from Earl F. Glynn's web page:
     * <a href="http://www.efg2.com/Lab/ScienceAndEngineering/Spectra.htm">Spectra Lab Report</a>
     */
    fn wave_length_to_rgb(wavelength: F<f64, f64>) -> (u8, u8, u8) {
        // convert from µm to nm
        let wavelength = wavelength.x * 1000.;
        let gamma = 0.80;
        let intensity_max = 255.;
        let factor;
        let red;
        let green;
        let blue;

        if (380. ..440.).contains(&wavelength) {
            red = -(wavelength - 440.) / (440. - 380.);
            green = 0.0;
            blue = 1.0;
        } else if (440. ..490.).contains(&wavelength) {
            red = 0.0;
            green = (wavelength - 440.) / (490. - 440.);
            blue = 1.0;
        } else if (490. ..510.).contains(&wavelength) {
            red = 0.0;
            green = 1.0;
            blue = -(wavelength - 510.) / (510. - 490.);
        } else if (510. ..580.).contains(&wavelength) {
            red = (wavelength - 510.) / (580. - 510.);
            green = 1.0;
            blue = 0.0;
        } else if (580. ..645.).contains(&wavelength) {
            red = 1.0;
            green = -(wavelength - 645.) / (645. - 580.);
            blue = 0.0;
        } else if (645. ..781.).contains(&wavelength) {
            red = 1.0;
            green = 0.0;
            blue = 0.0;
        } else {
            red = 0.0;
            green = 0.0;
            blue = 0.0;
        }

        // Let the intensity fall off near the vision limits

        if (380. ..420.).contains(&wavelength) {
            factor = 0.3 + 0.7 * (wavelength - 380.) / (420. - 380.);
        } else if (420. ..701.).contains(&wavelength) {
            factor = 1.0;
        } else if (701. ..781.).contains(&wavelength) {
            factor = 0.3 + 0.7 * (780. - wavelength) / (780. - 700.);
        } else {
            factor = 0.0;
        }

        // Don't want 0^x = 1 for x <> 0
        (
            if red == 0.0 {
                0
            } else {
                num::Float::round(intensity_max * (red * factor).pow(gamma)) as u8
            },
            if green == 0.0 {
                0
            } else {
                num::Float::round(intensity_max * (green * factor).pow(gamma)) as u8
            },
            if blue == 0.0 {
                0
            } else {
                num::Float::round(intensity_max * (blue * factor).pow(gamma)) as u8
            },
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuarterWaveCoatingAutodiff {
    pub thickness: f64,
    pub ior: f64,
}

impl QuarterWaveCoatingAutodiff {
    /// return a QuarterWaveCoating that is infinitely thin and so does nothing
    pub fn none() -> Self {
        Self {
            thickness: 0.,
            ior: 1.,
        }
    }

    /// return the optimal QuarterWaveCoating for the given parameters
    pub fn optimal(n0: f64, n2: f64, lambda0: f64) -> Self {
        let n1 = f64::max((n0 * n2).sqrt(), 1.23); // 1.38 = lowest achievable
        let d1 = lambda0 / 4. / n1; // phasedelay
        Self {
            thickness: d1,
            ior: n1,
        }
    }

    /// calculate the reflectivity R(λ, θ) of a surface coated with a single dielectric layer
    /// from Physically-Based Real-Time Lens Flare Rendering: Hullin 2011
    ///
    /// theta0: angle of incidence;
    /// lambda: wavelength of ray;
    /// d1: thickness of AR coating;
    /// n0: RI ( refr. index ) of 1st medium;
    /// n1: RI of coating layer;
    /// n2: RI of the 2nd medium;
    ///
    /// n1 = cmp::max((n0*n2).sqrt() , 1.38); // 1.38 = lowest achievable
    /// d1 = lambda0 / 4 / n1 ; // phasedelay
    ///
    /// ```
    /// # use polynomial_optics::QuarterWaveCoating;
    /// let coating = QuarterWaveCoatingAutodiff::none();
    /// assert_eq!(coating.fresnel_ar(std::f64::consts::PI / 2., 1., 1., 1.5), 1.0);
    /// //assert_eq!(fresnel_ar(1., 1., 0.25, 1.0, 1.5, 1.5), fresnel_ar(1., 1., 0.25, 1.0, 1.0, 1.5));
    /// ```
    pub fn fresnel_ar(
        &self,
        theta0: F<f64, f64>,
        lambda: F<f64, f64>,
        n0: F<f64, f64>,
        n2: F<f64, f64>,
    ) -> F<f64, f64> {
        // refracton angle sin coating and the 2nd medium
        let theta1 = (theta0.sin() * n0 / self.ior).asin();
        let theta2 = (theta0.sin() * n0 / n2).asin();
        println!("t1: {}, t2: {}", theta1, theta2);
        // amplitude for outer refl. / transmission on topmost interface
        let rs01 = -(theta0 - theta1).sin() / (theta0 + theta1).sin();
        let rp01 = (theta0 - theta1).tan() / (theta0 + theta1).tan();
        let ts01 = 2. * theta1.sin() * theta0.cos() / (theta0 + theta1).sin();
        let tp01 = ts01 * (theta0 - theta1).cos();
        // amplitude for inner reflection
        let rs12 = -(theta1 - theta2).sin() / (theta1 + theta2).sin();
        let rp12 = (theta1 - theta2).tan() / (theta1 + theta2).tan();
        // after passing through first surface twice:
        // 2 transmissions and 1 reflection
        let ris = ts01 * ts01 * rs12;
        let rip = tp01 * tp01 * rp12;
        // phasedifference between outer and inner reflections
        let dy = self.thickness * self.ior;
        let dx = theta1.tan() * dy;
        let delay = (dx * dx + dy * dy).sqrt();
        let rel_phase = 4. * std::f64::consts::PI / lambda * (delay - dx * theta0.sin());
        // Add up sines of different phase and amplitude
        let out_s2 = rs01 * rs01 + ris * ris + F1::var(2.) * rs01 * ris * rel_phase.cos();
        let out_p2 = rp01 * rp01 + rip * rip + F1::var(2.) * rp01 * rip * rel_phase.cos();
        (out_s2 + out_p2) / F1::var(2.) // reflectivity
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SellmeierAutodiff {
    pub b: [f64; 3],
    pub c: [f64; 3],
}

impl SellmeierAutodiff {
    pub fn air() -> Self {
        Self {
            b: [0., 0., 0.],
            c: [0., 0., 0.],
        }
    }

    pub fn ior(&self, wavelength: F<f64, f64>) -> F<f64, f64> {
        let wavelength_sq = wavelength * wavelength;
        let mut n_sq = F1::var(0.);
        for i in 0..3 {
            n_sq += (self.b[i] * wavelength_sq) / (wavelength_sq - self.c[i]);
        }
        n_sq.sqrt()
    }
}

impl SellmeierAutodiff {
    pub fn bk7() -> Self {
        Self {
            b: [1.03961212, 0.231792344, 1.01046945],
            c: [6.00069867e-3, 2.00179144e-2, 1.03560653e2],
        }
    }

    /// ```
    /// use polynomial_optics::SellmeierAutodiff;
    /// let now = std::time::Instant::now();
    /// SellmeierAutodiff::get_all_glasses();
    /// println!("{:?}", now.elapsed());
    /// panic!();
    /// ```
    pub fn get_all_glasses() -> Vec<(String, SellmeierAutodiff)> {
        let mut glasses = vec![];

        let mut rdr =
            csv::ReaderBuilder::new().from_reader(include_str!("../../LaCroix.csv").as_bytes());
        for result in rdr.records().flatten() {
            let b = [
                result[1].parse().unwrap(),
                result[2].parse().unwrap(),
                result[3].parse().unwrap(),
            ];
            let c = [
                result[4].parse().unwrap(),
                result[5].parse().unwrap(),
                result[6].parse().unwrap(),
            ];
            let glass = SellmeierAutodiff { b, c };
            glasses.push((result[0].trim().to_string(), glass));
        }
        glasses.sort_by_key(|(name, _glass)| name.clone());
        glasses
    }
}

/// ## Properties of a particular glass
/// saves ior and coating
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GlassAutodiff {
    /// ior vs air
    pub sellmeier: SellmeierAutodiff,
    /// coating - modifies wavelength, only used for reflection
    pub coating: QuarterWaveCoatingAutodiff,
    pub entry: bool,
    pub outer_ior: SellmeierAutodiff,
    pub spherical: bool,
}

/// # One element in a lens system
/// ```
/// # use polynomial_optics::raytracer::*;
/// let element = ElementAutodiff {
///    radius: 3.,
///    properties: Properties::Glass(GlassAutodiff {
///        ior: 1.5,
///        coating: QuarterWaveCoating::none(),
///        entry: true,
///        spherical: true,
///    }),
///    position: 0.,
/// };
/// let mut ray = RayAutodiff::default();
///
/// println!("space: {:?}", element);
/// println!("ray: {:?}", ray);
///
/// let ray2 = ray.propagate(&element);
/// println!("propagated ray: {:?}", ray2);
///
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ElementAutodiff {
    /// One optical interface
    pub radius: f64,
    pub position: f64,
    pub properties: PropertiesAutodiff,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum PropertiesAutodiff {
    Glass(GlassAutodiff),
    Aperture(u32),
}

impl RayAutodiff {
    pub fn new(
        o: Vector3<F<f64, f64>>,
        d: Vector3<F<f64, f64>>,
        wavelength: F<f64, f64>,
    ) -> RayAutodiff {
        RayAutodiff {
            o,
            d,
            wavelength,
            ..Default::default()
        }
    }

    fn fresnel_r(
        t1: F<f64, f64>,
        t2: F<f64, f64>,
        n1: F<f64, f64>,
        n2: F<f64, f64>,
    ) -> F<f64, f64> {
        let s = 0.5
            * ((n1 * t1.cos() - n2 * t2.cos()) / (n1 * t1.cos() + n2 * t2.cos())).pow(F::var(2.));
        let p = 0.5
            * ((n1 * t2.cos() - n2 * t1.cos()) / (n1 * t2.cos() + n2 * t1.cos())).pow(F::var(2.));

        s + p
    }

    fn propagate_element(
        &mut self,
        radius: f64,
        glass: &GlassAutodiff,
        position: f64,
        reflect: bool,
        entry: bool,
        cylindrical: bool,
    ) {
        let position = F1::var(position);
        let radius = F1::var(radius);
        let intersection = if cylindrical {
            // cylindrical: x is not affected by curvature

            // c: center of the lens surface if interpreted as an entire sphere
            let c = Vector2::new(
                F1::var(0.0),
                if entry {
                    position + radius
                } else {
                    position - radius
                },
            );
            let o: Vector2<F<f64, f64>> = Vector2 {
                x: self.o.y,
                y: self.o.z,
            };
            let d: Vector2<F<f64, f64>> = Vector2 {
                x: self.d.y,
                y: self.d.z,
            }
            .normalize();
            let delta = d.dot(o - c).pow(F1::var(2.0))
                - ((o - c).magnitude().pow(F1::var(2.0)) - radius.pow(F1::var(2.0)));

            let d1 = -(d.dot(o - c)) - delta.sqrt();
            let d2 = -(d.dot(o - c)) + delta.sqrt();

            if (entry == (self.d.z > F1::var(0.0))) == (radius > F1::var(0.0)) {
                self.o + self.d * d1
            } else {
                self.o + self.d * d2
            }
        } else {
            // c: center of the lens surface if interpreted as an entire sphere
            let c = Vector3::new(
                F1::var(0.0),
                F1::var(0.0),
                if entry {
                    position + radius
                } else {
                    position - radius
                },
            );

            let delta = self.d.dot(self.o - c).pow(F1::var(2.0))
                - ((self.o - c).magnitude().pow(F1::var(2.0)) - radius.pow(F1::var(2.0)));

            let d1 = -(self.d.dot(self.o - c)) - delta.sqrt();
            let d2 = -(self.d.dot(self.o - c)) + delta.sqrt();

            if (entry == (self.d.z > F1::var(0.0))) == (radius > F1::var(0.0)) {
                self.o + self.d * d1
            } else {
                self.o + self.d * d2
            }
        };

        self.o = intersection;

        let normal = if cylindrical {
            let c = Vector2::new(
                F1::var(0.0),
                if entry {
                    position + radius
                } else {
                    position - radius
                },
            );
            let intersection = Vector2 {
                x: intersection.y,
                y: intersection.z,
            }
            .normalize();

            let normal2d = intersection - c;

            let intersection = Vector3 {
                x: F1::var(0.0),
                y: normal2d.x,
                z: normal2d.y,
            };

            if (entry == (self.d.z > F1::var(0.0))) == (radius > F1::var(0.0)) {
                (intersection).normalize()
            } else {
                -(intersection).normalize()
            }
        } else {
            let c = Vector3::new(
                F1::var(0.0),
                F1::var(0.0),
                if entry {
                    position + radius
                } else {
                    position - radius
                },
            );
            if (entry == (self.d.z > F1::var(0.0))) == (radius > F1::var(0.0)) {
                (intersection - c).normalize()
            } else {
                -(intersection - c).normalize()
            }
        };

        if reflect {
            let d_in = self.d;

            self.d = self.d - normal * normal.dot(self.d) * F1::var(2.);

            self.strength *= glass.coating.fresnel_ar(
                d_in.angle(-normal).0,
                self.wavelength,
                if entry {
                    glass.outer_ior.ior(self.wavelength)
                } else {
                    glass.sellmeier.ior(self.wavelength)
                },
                if entry {
                    glass.sellmeier.ior(self.wavelength)
                } else {
                    glass.outer_ior.ior(self.wavelength)
                },
            );
            //     d_in.angle(normal).0,
            //     self.d.angle(-normal).0,
            //     if entry == (self.d.z > 0.) {
            //         glass.sellmeier.ior(self.wavelength)
            //     } else {
            //         glass.outerIOR.ior(self.wavelength)
            //     },
            //     if entry == (self.d.z > 0.) {
            //         glass.outerIOR.ior(self.wavelength)
            //     } else {
            //         glass.sellmeier.ior(self.wavelength)
            //     },
            // );
        } else {
            let eta = if entry {
                F1::var(1.0) / glass.sellmeier.ior(self.wavelength)
            } else {
                glass.sellmeier.ior(self.wavelength)
            };

            // from https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/refract.xhtml
            let k =
                F1::var(1.0) - eta * eta * (F1::var(1.0) - normal.dot(self.d) * normal.dot(self.d));

            let d_in = self.d;

            if k.x < 0.0 {
                // total reflection
                // println!("total reflection");
                self.d *= F1::var(0.); // or genDType(0.0)
            } else {
                self.d = self.d * eta - normal * (eta * normal.dot(self.d) + k.sqrt());
            }

            self.strength *= 1.0
                - RayAutodiff::fresnel_r(
                    d_in.angle(-normal).0,
                    self.d.angle(-normal).0,
                    if entry {
                        F::var(1.)
                    } else {
                        glass.sellmeier.ior(self.wavelength)
                    },
                    if entry {
                        glass.sellmeier.ior(self.wavelength)
                    } else {
                        F::var(1.)
                    },
                );
        }
    }

    fn clip_poly(&mut self, pos: f64, num_edge: u32, size: f64) {
        self.mov_plane(pos);

        let mut clipped = false;
        for i in 0..num_edge {
            let part = i as f64 * std::f64::consts::TAU / F1::var(num_edge as f64);
            let dir = Vector2 {
                x: part.cos(),
                y: part.sin(),
            };

            let dist = dir.dot(self.o.xy());
            clipped = clipped || (dist > F1::var(size));
        }
        if clipped {
            self.d *= F1::var(0.);
        }
    }

    /// propagate a ray through an element
    ///
    pub fn propagate(&mut self, element: &ElementAutodiff) {
        match element.properties {
            PropertiesAutodiff::Glass(glass) => self.propagate_element(
                element.radius,
                &glass,
                element.position,
                false,
                glass.entry,
                !glass.entry,
            ),
            PropertiesAutodiff::Aperture(properties) => {
                self.clip_poly(element.position, properties, element.radius)
            }
        };
    }

    /// reflect a Ray from an element
    ///
    pub fn reflect(&mut self, element: &ElementAutodiff) {
        match element.properties {
            PropertiesAutodiff::Glass(glass) => self.propagate_element(
                element.radius,
                &glass,
                element.position,
                true,
                glass.entry,
                !glass.entry,
            ),
            PropertiesAutodiff::Aperture(properties) => {
                self.clip_poly(element.position, properties, element.radius)
            }
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LensAutodiff {
    pub elements: Vec<ElementAutodiff>,
    pub sensor_dist: f32,
}

impl LensAutodiff {
    /// Saves the lens to the path provided.
    /// ```
    /// # use polynomial_optics::raytracer::*;
    /// let lens = LensAutodiff::new(vec![
    ///    Element {
    ///    radius: 1.,
    ///    properties: Properties::Glass(Glass {
    ///        ior: 1.5,
    ///        coating: (),
    ///        entry: true,
    ///        spherical: true,
    ///    }),
    ///    position: 0.,
    /// }]);
    /// lens.save(std::path::Path::new("./test.lens"));
    /// ```
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;
        let pretty_config = ron::ser::PrettyConfig::new();
        file.write_all(
            ron::ser::to_string_pretty(self, pretty_config)
                .unwrap()
                .as_bytes(),
        )?;
        // handle errors
        file.sync_all()?;
        Ok(())
    }

    /// Reads the lens from the path provided.
    /// ```
    /// # use polynomial_optics::raytracer::*;
    /// println!("{:?}", Lens::read(std::path::Path::new("./test.lens")));
    /// ```
    pub fn read(path: &Path) -> Result<LensAutodiff, String> {
        if let Ok(str) = std::fs::read_to_string(path) {
            return match ron::de::from_str(str.as_str()) {
                Ok(lens) => Ok(lens),
                Err(err) => Err(format!("{}", err)),
            };
        }
        Err(String::from("problem reading file"))
    }
}

impl LensAutodiff {
    pub fn new(elements: Vec<ElementAutodiff>, sensor_dist: f32) -> Self {
        Self {
            elements,
            sensor_dist,
        }
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
                PropertiesAutodiff::Glass(glass) => {
                    match glass.entry {
                        true => elements.push((element.position + element.radius) as f32),
                        false => elements.push((element.position - element.radius) as f32),
                    }
                    elements.push(element.radius as f32);
                }
                PropertiesAutodiff::Aperture(aperture) => (), //TODO: render Aperture
            }
        }
        elements
    }

    /// get elements in form:
    /// ```
    /// struct Element {
    ///   radius: f32;
    ///   glass: Sellmeier;
    ///   position: f32;
    ///   entry: bool;
    ///   spherical: bool;
    /// };
    /// pub struct Sellmeier {
    ///     pub b: [f64; 3],
    ///     pub c: [f64; 3],
    /// }
    /// ```
    /// only works if elements are entry and exit alternatively
    pub fn get_rt_elements_buffer(&self) -> Vec<f32> {
        let mut elements = vec![];

        for element in &self.elements {
            match element.properties {
                PropertiesAutodiff::Glass(glass) => {
                    elements.push(element.radius as f32);
                    elements.push(glass.sellmeier.b[0] as f32);
                    elements.push(glass.sellmeier.b[1] as f32);
                    elements.push(glass.sellmeier.b[2] as f32);
                    elements.push(glass.sellmeier.c[0] as f32);
                    elements.push(glass.sellmeier.c[1] as f32);
                    elements.push(glass.sellmeier.c[2] as f32);
                    elements.push(glass.outer_ior.b[0] as f32);
                    elements.push(glass.outer_ior.b[1] as f32);
                    elements.push(glass.outer_ior.b[2] as f32);
                    elements.push(glass.outer_ior.c[0] as f32);
                    elements.push(glass.outer_ior.c[1] as f32);
                    elements.push(glass.outer_ior.c[2] as f32);
                    elements.push(glass.coating.ior as f32);
                    elements.push(glass.coating.thickness as f32);
                    elements.push(element.position as f32);
                    elements.push(glass.entry as i32 as f32);
                    elements.push(glass.spherical as i32 as f32);
                }
                PropertiesAutodiff::Aperture(aperture) => {
                    elements.push(element.radius as f32);
                    elements.push(aperture as f32);
                    // placeholder
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);

                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);
                    elements.push(0_f32);

                    elements.push(0_f32);
                    elements.push(0_f32);
                    // placeholder end
                    elements.push(element.position as f32);
                    elements.push(2_f32);
                    elements.push(2_f32);
                }
            }
        }

        elements
    }

    /// get the indices of all distinct ghosts
    pub fn get_ghost_index(&self, draw_mode: usize, which_ghost: usize) -> Option<[u32; 2]> {
        if draw_mode & 1 > 0 {
            let mut ghost_num = 0;
            for i in 0..self.elements.len() - 1 {
                for j in i + 1..self.elements.len() {
                    if let PropertiesAutodiff::Glass(_) = self.elements[i].properties {
                        if let PropertiesAutodiff::Glass(_) = self.elements[j].properties {
                            ghost_num += 1;
                            if ghost_num == which_ghost || which_ghost == 0 {
                                return Some([i as u32, j as u32]);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// get the indices of all the elements at which to change direction for all distinct ghosts
    pub fn get_ghosts_indicies(&self, draw_mode: usize, which_ghost: usize) -> Vec<[u32; 2]> {
        let mut rays = vec![];
        if draw_mode & 1 > 0 {
            let mut ghost_num = 0;
            for i in 0..self.elements.len() - 1 {
                for j in i + 1..self.elements.len() {
                    if let PropertiesAutodiff::Glass(_) = self.elements[i].properties {
                        if let PropertiesAutodiff::Glass(_) = self.elements[j].properties {
                            ghost_num += 1;
                            if ghost_num == which_ghost || which_ghost == 0 {
                                rays.push([i as u32, j as u32]);
                            }
                        }
                    }
                }
            }
        }
        if draw_mode & 2 > 0 {
            rays.push([0, 0]);
        }
        rays
    }

    /// draws z y of the distance
    pub fn draw_rays(pixmap: &mut Pixmap, ray1: &RayAutodiff, ray2: &RayAutodiff) {
        let mut paint = tiny_skia::Paint::default();
        //paint.set_color(self.color);
        let color = ray1.get_rgb();
        paint.set_color(Color::from_rgba8(
            color.0,
            color.1,
            color.2,
            (255.0 * ray1.strength.sqrt().x * 0.5) as u8,
        ));
        paint.anti_alias = true;

        let middle = ((pixmap.width() / 4) as f32, (pixmap.height() / 2) as f32);

        let scale = (pixmap.height() / 10) as f32;

        let path = {
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(
                middle.0 + scale * ray1.o.z.x as f32,
                middle.1 + scale * ray1.o.y.x as f32,
            );

            pb.line_to(
                middle.0 + scale * (ray2.o.z.x) as f32,
                middle.1 + scale * (ray2.o.y.x) as f32,
            );
            pb.finish().unwrap()
        };

        let mut stroke = tiny_skia::Stroke {
            width: 1.0,
            ..Default::default()
        };
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

    /// Planks law in SI units
    /// ```
    /// # use polynomial_optics::raytracer::*;
    /// println!("{}", LensAutodiff::boltzmann(450./1_000_000., 3000.));
    /// panic!();
    /// ```
    fn plank(wavelen: f64, temp: f64) -> f64 {
        let h = 6.62607015e-34; // J/Hz
        let b = 1.380649e-23; // J/K
        let c = 299792458.; // m/s
        (2. * h * c.pow(2))
            / (wavelen.pow(5))
            / (std::f64::consts::E.pow((h * c) / (wavelen * b * temp)) - 1.)
    }

    fn str_from_wavelen(wavelen: f64) -> f64 {
        // -((wavelen - 0.45) * 4.) * ((wavelen - 0.45) * 4.) + 1.
        // std::f64::consts::E
        //     .pow(-((wavelen - 0.55) * 8.) * (wavelen - 0.55) * 8. - 15. * (wavelen - 0.55))
        //     / 2.5
        Self::plank(wavelen / 1_000., 5_000.) / 250.
    }

    pub fn draw(&self, pixmap: &mut Pixmap) {
        let num_rays = 5000;
        let width = 2.0;
        let wave_num = 20;
        for ray_num in 0..num_rays {
            let wavelen = ray_num % wave_num;
            let start_wavelen = 0.38;
            let end_wavelen = 0.78;
            let wavelength =
                start_wavelen + wavelen as f64 * ((end_wavelen - start_wavelen) / wave_num as f64);
            let strength = Self::str_from_wavelen(wavelength) / 10.;
            // for i in 0..self.elements.len() {
            //     for j in i..self.elements.len() {
            for i in 0..=0 {
                for j in 1..=1 {
                    let mut ray = RayAutodiff {
                        o: Vector3 {
                            x: F1::var(0.),
                            y: F1::var(ray_num as f64 / (num_rays as f64) * width - width / 2.),
                            z: F1::var(-5.),
                        },
                        d: Vector3 {
                            x: F1::var(0.0),
                            y: F1::var(0.2),
                            z: F1::var(1.0),
                        }
                        .normalize(),
                        wavelength: F1::var(wavelength),
                        strength: F1::var(strength),
                    };
                    let mut one = ray;
                    for (ele, element) in self.elements.iter().enumerate() {
                        // if we iterated through all elements up to
                        // the first reflection point

                        if ele == j {
                            // reflect at the first element,
                            // which is further down the optical path
                            ray.reflect(element);
                            LensAutodiff::draw_rays(pixmap, &one, &ray);
                            one = ray;
                            // propagate backwards through system
                            // until the second reflection
                            for k in (i + 1..j).rev() {
                                ray.propagate(&self.elements[k]);
                                LensAutodiff::draw_rays(pixmap, &one, &ray);
                                one = ray;
                            }
                            ray.reflect(&self.elements[i]);
                            LensAutodiff::draw_rays(pixmap, &one, &ray);
                            one = ray;
                            for k in i + 1..j {
                                ray.propagate(&self.elements[k]);
                                LensAutodiff::draw_rays(pixmap, &one, &ray);
                                one = ray;
                            }
                            // println!("strength: {}", ray.strength);
                        } else {
                            ray.propagate(element);
                            LensAutodiff::draw_rays(pixmap, &one, &ray);
                            one = ray;
                        }
                    }
                    ray.o += ray.d * F1::var(100.);
                    LensAutodiff::draw_rays(pixmap, &one, &ray);
                }
            }
            // let mut ray = Ray {
            //     o: Vector3 {
            //         x: 0.0,
            //         y: ray_num as f64 / (num_rays as f64) * width - width / 2.,
            //         z: -5.,
            //     },
            //     d: Vector3 {
            //         x: 0.0,
            //         y: 0.2,
            //         z: 1.0,
            //     }
            //     .normalize(),
            //     wavelength,
            //     strength,
            // };
            // let mut one = ray;
            // for element in &self.elements {
            //     ray.propagate(element);
            //     Lens::draw_rays(pixmap, &one, &ray);
            //     one = ray;
            // }
            // ray.o += ray.d * 100.;
            // Lens::draw_rays(pixmap, &one, &ray);
        }
    }

    pub fn get_dots(
        &self,
        num_rays: u32,
        center_pos: Vector3<f64>,
        direction: Vector3<f64>,
        draw_mode: u32,
        which_ghost: u32,
        sensor_pos: f64,
    ) -> Vec<F<f64, f64>> {
        // let rays = self.get_paths(
        //     num::integer::Roots::sqrt(&(num_rays * 1000)),
        //     center_pos,
        //     direction,
        //     draw_mode,
        //     which_ghost,
        // );

        let direction = Vector3 {
            x: F1::var(direction.x),
            y: F1::var(direction.y),
            z: F1::var(direction.z),
        };

        let center_pos = Vector3 {
            x: F1::var(center_pos.x),
            y: F1::var(center_pos.y),
            z: F1::var(center_pos.z),
        };

        let mut rays = vec![];

        let width = 2.0;
        for ray_num_x in 0..num_rays {
            for ray_num_y in 0..num_rays {
                let wave_num = 10;
                let ray_num = ray_num_x * num_rays + ray_num_y;
                let wavelen = (ray_num % wave_num) as f64;
                let start_wavelen = 0.38;
                let end_wavelen = 0.78;
                let wavelength =
                    start_wavelen + wavelen * ((end_wavelen - start_wavelen) / (wave_num as f64));
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
                                let mut ray = RayAutodiff::new(pos, direction, F1::var(wavelength));

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
                                ray.o += ray.d * F1::var(100.);

                                // only return rays that have made it through
                                if ray.d.magnitude().x > 0. {
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
                    let mut ray = RayAutodiff::new(pos, direction, F1::var(wavelength));
                    for element in &self.elements {
                        ray.propagate(element);
                    }
                    ray.o += ray.d * F1::var(100.);

                    // only return rays that have made it through
                    if ray.d.magnitude().x > 0. {
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
            dots.push(intersection[0]);
            dots.push(intersection[1]);
            dots.push(ray.strength);
        }
        dots
    }
}
