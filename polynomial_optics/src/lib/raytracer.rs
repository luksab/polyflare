use std::{
    fs::OpenOptions,
    hash::{Hash, Hasher},
    io::Write,
    path::Path,
};

use itertools::iproduct;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use cgmath::{num_traits::Pow, prelude::*, Vector2, Vector3};
use tiny_skia::{Color, Pixmap};

///struct DrawRay {
///  pos: vec2<f32>;
///  aperture_pos: vec2<f32>;
///  entry_pos: vec2<f32>;
///  strength: f32;
///  wavelength: f32;
///};
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct DrawRay {
    pub ghost_num: u32,
    pub init_pos: [f64; 4],
    pub pos: [f64; 2],
    pub aperture_pos: [f64; 2],
    pub entry_pos: [f64; 2],
    pub strength: f64,
    pub wavelength: f64,
}

/// ## A ray at a plane in the lens system
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// origin of the Ray, 0 in the optical axis
    pub o: cgmath::Vector3<f64>,
    /// wavelength in µm
    pub wavelength: f64,
    /// direction of the Ray, 0 if in the path of the optical axis, is a unit vector
    pub d: cgmath::Vector3<f64>,
    pub strength: f64,
    pub ghost_num: u32,
    pub init_pos: [f64; 4],
    pub aperture_pos: [f64; 2],
    pub entry_pos: [f64; 2],
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            o: Vector3::new(0., 0., 0.),
            d: Vector3::new(0., 0., 1.),
            strength: 1.,
            wavelength: 0.5,
            ghost_num: 0,
            init_pos: [0., 0., 0., 0.],
            aperture_pos: [0., 0.],
            entry_pos: [0., 0.],
        }
    }
}

impl Ray {
    fn intersect(&self, plane: f64) -> [f64; 2] {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        let intersect = self.o + self.d * num_z;
        [intersect.x, intersect.y]
    }

    fn intersect_vec(&self, plane: f64) -> Vector3<f64> {
        let diff = plane - self.o.z;
        let num_z = diff / self.d.z;

        self.o + self.d * num_z
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
    fn wave_length_to_rgb(wavelength: f64) -> (u8, u8, u8) {
        // convert from µm to nm
        let wavelength = wavelength * 1000.;
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
pub struct QuarterWaveCoating {
    pub thickness: f64,
    pub ior: f64,
}

impl Hash for QuarterWaveCoating {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let t = self.thickness.to_bits();
        t.hash(state);
        let i = self.ior.to_bits();
        i.hash(state);
    }
}

impl QuarterWaveCoating {
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
    /// let coating = QuarterWaveCoating::none();
    /// assert_eq!(coating.fresnel_ar(std::f64::consts::PI / 2., 1., 1., 1.5), 1.0);
    /// //assert_eq!(fresnel_ar(1., 1., 0.25, 1.0, 1.5, 1.5), fresnel_ar(1., 1., 0.25, 1.0, 1.0, 1.5));
    /// ```
    pub fn fresnel_ar(&self, theta0: f64, lambda: f64, n0: f64, n2: f64) -> f64 {
        // refracton angle sin coating and the 2nd medium
        let theta1 = f64::asin(f64::sin(theta0) * n0 / self.ior);
        let theta2 = f64::asin(f64::sin(theta0) * n0 / n2);
        // println!("t1: {}, t2: {}", theta1, theta2);
        // amplitude for outer refl. / transmission on topmost interface
        let rs01 = -f64::sin(theta0 - theta1) / f64::sin(theta0 + theta1);
        let rp01 = f64::tan(theta0 - theta1) / f64::tan(theta0 + theta1);
        let ts01 = 2. * f64::sin(theta1) * f64::cos(theta0) / f64::sin(theta0 + theta1);
        let tp01 = ts01 * f64::cos(theta0 - theta1);
        // amplitude for inner reflection
        let rs12 = -f64::sin(theta1 - theta2) / f64::sin(theta1 + theta2);
        let rp12 = f64::tan(theta1 - theta2) / f64::tan(theta1 + theta2);
        // after passing through first surface twice:
        // 2 transmissions and 1 reflection
        let ris = ts01 * ts01 * rs12;
        let rip = tp01 * tp01 * rp12;
        // phase difference between outer and inner reflections
        let dy = self.thickness * self.ior;
        let dx = f64::tan(theta1) * dy;
        let delay = f64::sqrt(dx * dx + dy * dy);
        let rel_phase = 4. * std::f64::consts::PI / lambda * (delay - dx * f64::sin(theta0));
        // Add up sines of different phase and amplitude
        let out_s2 = rs01 * rs01 + ris * ris + 2. * rs01 * ris * f64::cos(rel_phase);
        let out_p2 = rp01 * rp01 + rip * rip + 2. * rp01 * rip * f64::cos(rel_phase);
        (out_s2 + out_p2) / 2. // reflectivity
    }

    /// ```
    /// # use polynomial_optics::QuarterWaveCoating;
    /// let coating = QuarterWaveCoating::optimal(1.0, 1.5168, 0.5);
    /// coating.plot();
    /// panic!("done plotting");
    pub fn plot(&self) {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("python/coating_wl_sweep.csv"))
            .unwrap();

        let wl_min = 0.31;
        let wl_max = 1.1;
        let num_steps = 100;
        // BK7,1.03961212,0.231792344,1.01046945,0.00600069867,0.0200179144,103.560653
        let bk7 = Sellmeier::bk7();
        for i in 0..num_steps {
            let wavelength = wl_min + (wl_max - wl_min) * i as f64 / num_steps as f64;

            let reflectance_entry = self.fresnel_ar(0.01, wavelength, 1.0, bk7.ior(wavelength));
            let reflectance_exit = self.fresnel_ar(0.01, wavelength, bk7.ior(wavelength), 1.0);
            // let transmittance = 1.0 - reflectance;
            // let transmittance = 1.0 - reflectance;
            writeln!(file, "{}, {}, {}", wavelength, reflectance_entry, reflectance_exit).unwrap();
        }

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("python/coating_angle_sweep.csv"))
            .unwrap();

        let angle_min = 0.01;
        let angle_max = std::f64::consts::PI / 2.0;
        let num_steps = 100;
        // BK7,1.03961212,0.231792344,1.01046945,0.00600069867,0.0200179144,103.560653
        let bk7 = Sellmeier::bk7();
        for i in 0..num_steps {
            let angle = angle_min + (angle_max - angle_min) * i as f64 / num_steps as f64;
            let wavelength = 0.5;

            let reflectance_entry = self.fresnel_ar(angle, wavelength, 1.0, bk7.ior(wavelength));
            let reflectance_exit = self.fresnel_ar(angle, wavelength, bk7.ior(wavelength), 1.0);
            // let transmittance = 1.0 - reflectance;
            // let transmittance = 1.0 - reflectance;
            writeln!(file, "{}, {}, {}", angle, reflectance_entry, reflectance_exit).unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Sellmeier {
    pub b: [f64; 3],
    pub c: [f64; 3],
}

impl Hash for Sellmeier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for i in 0..3 {
            let b = self.b[i].to_bits();
            b.hash(state);
            let c = self.c[i].to_bits();
            c.hash(state);
        }
    }
}

impl Sellmeier {
    pub fn air() -> Self {
        Self {
            b: [0., 0., 0.],
            c: [0., 0., 0.],
        }
    }

    pub fn ior(&self, wavelength: f64) -> f64 {
        let wavelength_sq = wavelength * wavelength;
        let mut n_sq = 1.;
        for i in 0..3 {
            n_sq += (self.b[i] * wavelength_sq) / (wavelength_sq - self.c[i]);
        }
        n_sq.sqrt()
    }
}

impl Sellmeier {
    pub fn bk7() -> Self {
        Self {
            b: [1.03961212, 0.231792344, 1.01046945],
            c: [6.00069867e-3, 2.00179144e-2, 1.03560653e2],
        }
    }

    /// ```
    /// use polynomial_optics::Sellmeier;
    /// let now = std::time::Instant::now();
    /// Sellmeier::get_all_glasses();
    /// println!("{:?}", now.elapsed());
    /// panic!();
    /// ```
    pub fn get_all_glasses() -> Vec<(String, Sellmeier)> {
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
            let glass = Sellmeier { b, c };
            glasses.push((result[0].trim().to_string(), glass));
        }
        glasses.sort_by_key(|(name, _glass)| name.clone());
        glasses
    }
}

/// ## Properties of a particular glass
/// saves ior and coating
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct Glass {
    /// ior vs air
    pub sellmeier: Sellmeier,
    /// coating - modifies wavelength, only used for reflection
    pub coating: QuarterWaveCoating,
    pub entry: bool,
    pub outer_ior: Sellmeier,
    pub spherical: bool,
}

/// # One element in a lens system
/// ```
/// # use polynomial_optics::raytracer::*;
/// let element = Element {
///    radius: 3.,
///    properties: Properties::Glass(Glass {
///        ior: 1.5,
///        coating: QuarterWaveCoating::none(),
///        entry: true,
///        spherical: true,
///    }),
///    position: 0.,
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
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Element {
    /// One optical interface
    pub radius: f64,
    pub position: f64,
    pub properties: Properties,
}

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let r = self.radius.to_bits();
        r.hash(state);
        let p = self.position.to_bits();
        p.hash(state);
        self.properties.hash(state);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash)]
pub enum Properties {
    Glass(Glass),
    Aperture(u32),
}

impl Ray {
    pub fn new(o: Vector3<f64>, d: Vector3<f64>, init_pos: [f64; 4], wavelength: f64) -> Ray {
        Ray {
            o,
            d,
            wavelength,
            init_pos,
            ..Default::default()
        }
    }

    fn fresnel_r(t1: f64, t2: f64, n1: f64, n2: f64) -> f64 {
        let s = 0.5 * ((n1 * t1.cos() - n2 * t2.cos()) / (n1 * t1.cos() + n2 * t2.cos())).pow(2);
        let p = 0.5 * ((n1 * t2.cos() - n2 * t1.cos()) / (n1 * t2.cos() + n2 * t1.cos())).pow(2);

        s + p
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
                1.0 / glass.sellmeier.ior(self.wavelength)
            } else {
                glass.sellmeier.ior(self.wavelength)
            };

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
                    if entry {
                        1.0
                    } else {
                        glass.sellmeier.ior(self.wavelength)
                    },
                    if entry {
                        glass.sellmeier.ior(self.wavelength)
                    } else {
                        1.0
                    },
                );
        }
    }

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
                self.aperture_pos = self.intersect(element.position);
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Lens {
    pub elements: Vec<Element>,
    pub sensor_dist: f64,
}

impl Hash for Lens {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.elements.hash(state);
        let v = self.sensor_dist.to_bits();
        v.hash(state);
    }
}

impl Lens {
    /// Saves the lens to the path provided.
    /// ```
    /// # use polynomial_optics::raytracer::*;
    /// let lens = Lens::new(vec![
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
    pub fn read(path: &Path) -> Result<Lens, String> {
        if let Ok(str) = std::fs::read_to_string(path) {
            return match ron::de::from_str(str.as_str()) {
                Ok(lens) => Ok(lens),
                Err(err) => Err(format!("{}", err)),
            };
        }
        Err(String::from("problem reading file"))
    }
}

impl Lens {
    pub fn new(elements: Vec<Element>, sensor_dist: f64) -> Self {
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
                Properties::Glass(glass) => {
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
                Properties::Aperture(aperture) => {
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
                    if let Properties::Glass(_) = self.elements[i].properties {
                        if let Properties::Glass(_) = self.elements[j].properties {
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
                    if let Properties::Glass(_) = self.elements[i].properties {
                        if let Properties::Glass(_) = self.elements[j].properties {
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
    pub fn draw_rays(pixmap: &mut Pixmap, ray1: &Ray, ray2: &Ray) {
        let mut paint = tiny_skia::Paint::default();
        //paint.set_color(self.color);
        let color = ray1.get_rgb();
        paint.set_color(Color::from_rgba8(
            color.0,
            color.1,
            color.2,
            (255.0 * ray1.strength.sqrt() * 0.5) as u8,
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
    /// println!("{}", Lens::boltzmann(450./1_000_000., 3000.));
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
                    let mut ray = Ray {
                        o: Vector3 {
                            x: 0.0,
                            y: ray_num as f64 / (num_rays as f64) * width - width / 2.,
                            z: -5.,
                        },
                        d: Vector3 {
                            x: 0.0,
                            y: 0.2,
                            z: 1.0,
                        }
                        .normalize(),
                        wavelength,
                        strength,
                        ..Default::default()
                    };
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
                            // println!("strength: {}", ray.strength);
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

    pub fn trace_ray(&self, mut ray: Ray, i: usize, j: usize) -> Ray {
        for element in &self.elements {
            ray.propagate(element);
        }
        ray
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
            let wave_num = 10;
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
                            pos.y += ray_num as f64 / (num_rays as f64) * width - width / 2.;
                            let mut ray = Ray::new(pos, direction, [0., 0., 0., pos.y], wavelength);
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
                let mut ray = Ray::new(pos, direction, [0., 0., 0., pos.y], wavelength);
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
                                let mut ray_collection = vec![];
                                // make new ray
                                let mut pos = center_pos;
                                pos.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                                pos.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                                let mut ray =
                                    Ray::new(pos, direction, [0., 0., pos.x, pos.y], wavelength);
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
                    let mut ray = Ray::new(pos, direction, [0., 0., pos.x, pos.y], wavelength);
                    let mut ray_collection = vec![ray];
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

    pub fn get_ghost_dot(&self, i: usize, j: usize, mut ray: Ray, sensor_pos: f64) -> Ray {
        ray.d = ray.d.normalize();
        // ray.init_pos = ray.intersect(self.elements[0].position);

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
        ray.o = ray.intersect_vec(sensor_pos);

        ray
    }

    pub fn get_at_pos(
        &self,
        pos: Vector3<f64>,
        dir: Vector3<f64>,
        which_ghost: usize,
        sensor_pos: f64,
    ) -> Ray {
        let mut ray = Ray::new(pos, dir, [pos.x, pos.y, dir.x, dir.y], 0.5);
        let mut ghost_num = 0;
        for i in 0..self.elements.len() - 1 {
            for j in i + 1..self.elements.len() {
                ghost_num += 1;
                if ghost_num == which_ghost {
                    ray = self.get_ghost_dot(i, j, ray, sensor_pos);
                }
            }
        }
        ray
    }

    pub fn get_dots_2dgrid(
        &self,
        side_len: u32,
        pos: Vector3<f64>,
        which_ghost: u32,
        sensor_pos: f64,
        width: [f64; 2],
        filter: bool,
    ) -> Vec<DrawRay> {
        let center_dir = self.get_center_dir(pos);

        let mut rays = vec![];

        let width_d = width[0];
        let width_p = width[1];
        // if draw_mode & 1 > 0 {
        let mut ghost_num = 0;
        for i in 0..self.elements.len() - 1 {
            for j in i + 1..self.elements.len() {
                ghost_num += 1;
                if ghost_num == which_ghost {
                    rays.append(
                        &mut iproduct!(0..side_len, 0..side_len)
                            .into_iter()
                            .par_bridge()
                            .filter_map(|(z, w)| {
                                // let wave_num = 1;
                                // let ray_num = ray_num_x * num_rays + ray_num_y;
                                // let wavelen = (ray_num % wave_num) as f64;
                                // let start_wavelen = 0.38;
                                // let end_wavelen = 0.78;
                                // let wavelength = start_wavelen
                                //     + wavelen * ((end_wavelen - start_wavelen) / (wave_num as f64));
                                let (x, y, z, w) = (
                                    0.5,
                                    0.5,
                                    z as f64 / side_len as f64,
                                    w as f64 / side_len as f64,
                                );
                                let wavelength = 0.5;

                                // make new ray
                                let mut pos = pos;
                                pos.x += x * width_p - width_p / 2.;
                                pos.y += y * width_p - width_p / 2.;

                                let mut dir = center_dir;
                                dir.x += z * width_d - width_d / 2.;
                                dir.y += w * width_d - width_d / 2.;

                                let mut ray =
                                    Ray::new(pos, dir, [pos.x, pos.y, dir.x, dir.y], wavelength);
                                ray.ghost_num = ghost_num;
                                let ray = self.get_ghost_dot(i, j, ray, sensor_pos);
                                if !filter || ray.o.is_finite() {
                                    Some(ray)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                    );
                }
            }
        }
        rays.iter()
            .map(|ray| DrawRay {
                pos: ray.o.xy().into(),
                wavelength: ray.wavelength,
                strength: ray.strength,
                ghost_num: ray.ghost_num,
                init_pos: ray.init_pos,
                aperture_pos: ray.aperture_pos,
                entry_pos: ray.entry_pos,
            })
            .collect()
    }

    pub fn get_dots_grid(
        &self,
        side_len: u32,
        pos: Vector3<f64>,
        which_ghost: u32,
        sensor_pos: f64,
        width: [f64; 2],
        filter: bool,
    ) -> Vec<DrawRay> {
        let center_dir = self.get_center_dir(pos);

        let mut rays = vec![];

        let width_d = width[0];
        let width_p = width[1];
        // if draw_mode & 1 > 0 {
        let mut ghost_num = 0;
        for i in 0..self.elements.len() - 1 {
            for j in i + 1..self.elements.len() {
                ghost_num += 1;
                if ghost_num == which_ghost {
                    rays.append(
                        &mut iproduct!(0..side_len, 0..side_len, 0..side_len, 0..side_len)
                            .into_iter()
                            .par_bridge()
                            .filter_map(|(x, y, z, w)| {
                                // let wave_num = 1;
                                // let ray_num = ray_num_x * num_rays + ray_num_y;
                                // let wavelen = (ray_num % wave_num) as f64;
                                // let start_wavelen = 0.38;
                                // let end_wavelen = 0.78;
                                // let wavelength = start_wavelen
                                //     + wavelen * ((end_wavelen - start_wavelen) / (wave_num as f64));
                                let (x, y, z, w) = (
                                    x as f64 / side_len as f64,
                                    y as f64 / side_len as f64,
                                    z as f64 / side_len as f64,
                                    w as f64 / side_len as f64,
                                );
                                let wavelength = 0.5;

                                // make new ray
                                let mut pos = pos;
                                pos.x += x * width_p - width_p / 2.;
                                pos.y += y * width_p - width_p / 2.;

                                let mut dir = center_dir;
                                dir.x += z * width_d - width_d / 2.;
                                dir.y += w * width_d - width_d / 2.;

                                let mut ray =
                                    Ray::new(pos, dir, [pos.x, pos.y, dir.x, dir.y], wavelength);
                                ray.ghost_num = ghost_num;
                                let ray = self.get_ghost_dot(i, j, ray, sensor_pos);
                                if !filter || ray.o.is_finite() {
                                    Some(ray)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                    );
                }
            }
        }
        rays.iter()
            .map(|ray| DrawRay {
                pos: ray.o.xy().into(),
                wavelength: ray.wavelength,
                strength: ray.strength,
                ghost_num: ray.ghost_num,
                init_pos: ray.init_pos,
                aperture_pos: ray.aperture_pos,
                entry_pos: ray.entry_pos,
            })
            .collect()
    }

    pub fn get_dots(
        &self,
        num_rays: u32,
        pos: Vector3<f64>,
        which_ghost: u32,
        sensor_pos: f64,
        width: [f64; 2],
        filter: bool,
    ) -> Vec<DrawRay> {
        // Pick an arbitrary number as seed.
        // fastrand::seed(7);
        // let rays = self.get_paths(
        //     num::integer::Roots::sqrt(&(num_rays * 1000)),
        //     center_pos,
        //     direction,
        //     draw_mode,
        //     which_ghost,
        // );

        let center_dir = self.get_center_dir(pos);

        let mut rays = vec![];

        let width_d = width[0];
        let width_p = width[1];
        // if draw_mode & 1 > 0 {
        let mut ghost_num = 0;
        for i in 0..self.elements.len() - 1 {
            for j in i + 1..self.elements.len() {
                ghost_num += 1;
                if ghost_num == which_ghost {
                    while rays.len() < num_rays as usize {
                        // let wave_num = 1;
                        // let ray_num = ray_num_x * num_rays + ray_num_y;
                        // let wavelen = (ray_num % wave_num) as f64;
                        // let start_wavelen = 0.38;
                        // let end_wavelen = 0.78;
                        // let wavelength = start_wavelen
                        //     + wavelen * ((end_wavelen - start_wavelen) / (wave_num as f64));
                        let wavelength = 0.5;

                        // make new ray
                        let mut dir = center_dir;
                        // dir.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
                        // dir.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
                        dir.x += fastrand::f64() * width_d - width_d / 2.;
                        dir.y += fastrand::f64() * width_d - width_d / 2.;

                        let mut pos = pos;
                        pos.x += fastrand::f64() * width_p - width_p / 2.;
                        pos.y += fastrand::f64() * width_p - width_p / 2.;
                        let mut ray = Ray::new(pos, dir, [pos.x, pos.y, dir.x, dir.y], wavelength);
                        ray.ghost_num = ghost_num;
                        let ray = self.get_ghost_dot(i, j, ray, sensor_pos);
                        if !filter || ray.o.is_finite() {
                            rays.push(ray);
                        }
                    }
                }
            }
        }
        // }

        // if draw_mode & 2 > 0 {
        //     for ray_num_x in 0..num_rays {
        //         for ray_num_y in 0..num_rays {
        //             let wave_num = 1;
        //             let ray_num = ray_num_x * num_rays + ray_num_y;
        //             let wavelen = (ray_num % wave_num) as f64;
        //             let start_wavelen = 0.38;
        //             let end_wavelen = 0.78;
        //             let wavelength = start_wavelen
        //                 + wavelen * ((end_wavelen - start_wavelen) / (wave_num as f64));
        //             let mut dir = center_dir;
        //             dir.x += ray_num_x as f64 / (num_rays as f64) * width - width / 2.;
        //             dir.y += ray_num_y as f64 / (num_rays as f64) * width - width / 2.;
        //             let mut ray = Ray::new(
        //                 pos,
        //                 dir,
        //                 [
        //                     pos.x,
        //                     pos.y,
        //                     ray_num_x as f64 / (num_rays as f64),
        //                     ray_num_y as f64 / (num_rays as f64),
        //                 ],
        //                 wavelength,
        //             );
        //             // ray.init_pos = ray.intersect(self.elements[0].position);
        //             for element in &self.elements {
        //                 ray.propagate(element);
        //             }
        //             ray.o = ray.intersect_vec(sensor_pos as f64);

        //             // only return rays that have made it through
        //             if ray.d.magnitude() > 0. || !filter {
        //                 rays.push(ray);
        //             }
        //         }
        //     }
        // }

        // println!(
        //     "in: {} out:{} percent: {}",
        //     &num_rays * 100,
        //     rays.len(),
        //     rays.len() as f64 / (&num_rays * 100) as f64 * 100.
        // );
        rays.iter()
            .map(|ray| DrawRay {
                pos: ray.o.xy().into(),
                wavelength: ray.wavelength,
                strength: ray.strength,
                ghost_num: ray.ghost_num,
                init_pos: ray.init_pos,
                aperture_pos: ray.aperture_pos,
                entry_pos: ray.entry_pos,
            })
            .collect()
    }

    pub fn get_center_dir(&self, pos: Vector3<f64>) -> Vector3<f64> {
        Vector3 {
            x: 0.,
            y: 0.,
            z: self.elements[0].position,
        } - pos
    }
}
