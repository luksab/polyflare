/// one Ray with origin, direction, and strength
struct Ray {
  o: vec3<f32>;
  wavelength: f32;
  d: vec3<f32>;
  strength: f32;
};

/// one Lens Element 
/// - one optical interface between glass and air
struct Element {
  radius: f32;
  b1: f32;
  b2: f32;
  b3: f32;
  c1: f32;
  c2: f32;
  c3: f32;
  b1_2: f32;
  b2_2: f32;
  b3_2: f32;
  c1_2: f32;
  c2_2: f32;
  c3_2: f32;
  coating_ior: f32;
  coating_thickness: f32;
  position: f32;// num_blades if aperture
  entry: f32;// 0: false, 1: true, 2: aperture
  spherical: f32;// 0: false, 1: true
};

struct SimParams {
  opacity: f32;
  width_scaled: f32;
  height_scaled: f32;
  width: f32;
  height: f32;
  draw_mode: f32;
  which_ghost: f32;
  window_width_scaled: f32;
  window_height_scaled: f32;
  window_width: f32;
  window_height: f32;
  side_len: f32;
  zoom: f32;
};


// static parameters for positions
struct PosParams {
  // the Ray to be modified as a base for ray tracing
  init: Ray;
  // position of the sensor in the optical plane
  sensor: f32;
  width: f32;
};


struct Rays {
  rays: [[stride(32)]] array<Ray>;
};

/// all the Elements of the Lens under test
struct Elements {
  el: [[stride(72)]] array<Element>;
};

struct PolyParams {
    num_terms: u32;
};

struct Monomial {
    coefficient: f32;
    a: f32;
    b: f32;
    c: f32;
    d: f32;
};

struct Polynomial {
    monomials: [[stride(32)]] array<Monomial>;
};

[[group(0), binding(0)]] var<storage, read_write> rays : Rays;
[[group(0), binding(1)]] var<storage, read> terms : Polynomial;
[[group(0), binding(2)]] var<uniform> polyParams : PolyParams;

[[group(1), binding(2)]] var<uniform> params : SimParams;

[[group(1), binding(0)]] var<uniform> posParams : PosParams;

[[group(2), binding(0)]] var<storage, read> elements : Elements;

// intersect a ray with the sensor / any plane on the optical axis
fn intersect_ray_to_ray(self: Ray, plane: f32) -> Ray {
    var ray = self;
    let diff = plane - ray.o.z;
    let num_z = diff / ray.d.z;

    let intersect = ray.o + ray.d * num_z;
    ray.o = intersect;
    return ray;
}

let tpi: f32 = 6.283185307179586;
fn clip_ray_poly(self: Ray, pos: f32, num_edge: u32, size: f32) -> bool {
    let ray = intersect_ray_to_ray(self, pos);
    var clipped = false;
    for (var i = u32(0); i < num_edge; i = i + u32(1)) {
        let part = f32(i) * tpi / f32(num_edge);
        let dir = vec2<f32>(cos(part), sin(part));

        let dist = dot(dir, ray.o.xy);
        clipped = clipped || (dist > size);
    }
    return clipped;
}

// intersect a ray with the sensor / any plane on the optical axis
fn intersect_ray(self: Ray, plane: f32) -> Ray {
    let diff = plane - self.o.z;
    let num_z = diff / self.d.z;

    let intersect = self.o + self.d * num_z;
    var ray = self;
    ray.o = vec3<f32>(intersect.x, intersect.y, ray.o.z);
    return ray;
}

fn eval(x: vec4<f32>, index: u32) -> f32 {
    var res = 0.;
    for (var i = u32(index * polyParams.num_terms); i < polyParams.num_terms; i = i + u32(1)) {
        let term = terms.monomials[i];
        res = res + term.coefficient * pow(x.x, term.a) * pow(x.y, term.b) * pow(x.z, term.c) * pow(x.w, term.d);
    }
    return res;
}

fn applyPoly(ray: Ray) -> Ray {
    let x = eval(vec4<f32>(ray.o.xy, ray.d.xy), u32(0));
    let y = eval(vec4<f32>(ray.o.xy, ray.d.xy), u32(1));
    return ray;
}

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let draw_mode = u32(params.draw_mode);//u32(1);
  let which_ghost = u32(params.which_ghost);//u32(1);

  // calculate the number of dots for a given input ray
  var num_segments = u32((draw_mode & u32(2)) > u32(0));// if normal drawing
  if ((draw_mode & u32(1)) > u32(0)) { // if ghost drawing
    var ghost_num = u32(0);
    for (var i = u32(0); i < arrayLength(&elements.el) - u32(1); i = i + u32(1)) {
        for (var j = i + u32(1); j < arrayLength(&elements.el); j = j + u32(1)) {
            ghost_num = ghost_num + u32(1);
            if ((ghost_num == which_ghost || which_ghost == u32(0)) && elements.el[i].entry < 1.5 && elements.el[j].entry < 1.5) {
                num_segments = num_segments + u32(1);
            }
        }
    }
  }
  // the total number of possible shader executions
  let total = arrayLength(&rays.rays) / num_segments;
  let index = global_invocation_id.x;
  if (index >= total) { // if we don't fit in the buffer - return early
    return;
  }

  let num_rays = total;
  let ray_num = index;

  // how much to move the rays by to sample
  let width = posParams.width;

  // we need the sqrt to scale the movement in each direction by
  let sqrt_num = u32(sqrt(f32(num_rays)));
  let ray_num_x = f32(ray_num / sqrt_num);
  let ray_num_y = f32(ray_num % sqrt_num);

  let wave_num = u32(500);

  // how many dots have we added to the buffer
  var counter = u32(0);
  if ((draw_mode & u32(1)) > u32(0)) {
    // which ghost are we on
    var ghost_num = u32(0);
    // iterate through all combinations of Elements to draw the ghosts
    for (var i = u32(0); i < arrayLength(&elements.el) - u32(1); i = i + u32(1)) {
        for (var j = i + u32(1); j < arrayLength(&elements.el); j = j + u32(1)) {
            if ( elements.el[i].entry < 1.5 && elements.el[j].entry < 1.5 ){
            ghost_num = ghost_num + u32(1);
            // if we want to draw this ghost or we want to draw all ghosts
            if (ghost_num == which_ghost || which_ghost == u32(0)) {
                // make new ray
                var dir = posParams.init.d;
                // modify both directions according to our index
                dir.x = dir.x + (ray_num_x / f32(sqrt_num - u32(1)) * width - width / 2.);
                dir.y = dir.y + (ray_num_y / f32(sqrt_num - u32(1)) * width - width / 2.);
                dir = normalize(dir);
                // pos.y = pos.y + f32(ray_num) / f32(num_rays) * width - width / 2.;
                let wavelen = f32(ray_num % wave_num);
                let start_wavelen = 0.38;
                let end_wavelen = 0.78;
                let wavelength = start_wavelen + wavelen * ((end_wavelen - start_wavelen) / f32(wave_num));
                var ray = Ray(posParams.init.o, wavelength, dir, 1.);

                ray = applyPoly(ray);

                // intersect with the plane
                ray = intersect_ray(ray, posParams.sensor);

                // only return rays that have made it through
                if (length(ray.d) > 0. && ray.strength > 0.) {
                    rays.rays[ray_num * num_segments + counter] = ray;
                    counter = counter + u32(1);
                } else {
                    rays.rays[ray_num * num_segments + counter] = Ray(vec3<f32>(100., 100., 100.), 0.5, vec3<f32>(0.0, 0.0, 0.0), 0.0);
                    counter = counter + u32(1);
                }
            }
            }
        }
    }
  }
  // if we want to draw normally
  if ((draw_mode & u32(2)) > u32(0)) {
    // make new ray
    var dir = posParams.init.d;
    // modify both directions according to our index
    dir.x = dir.x + (ray_num_x / f32(sqrt_num - u32(1)) * width - width / 2.);
    dir.y = dir.y + (ray_num_y / f32(sqrt_num - u32(1)) * width - width / 2.);
    dir = normalize(dir);
    let wavelen = f32(ray_num % wave_num);
    let start_wavelen = 0.38;
    let end_wavelen = 0.78;
    let wavelength = start_wavelen + wavelen * ((end_wavelen - start_wavelen) / f32(wave_num));
    var ray = Ray(posParams.init.o, wavelength, dir, 1.);
    
    ray = applyPoly(ray);
    
    // intersect the ray with the sensor
    ray = intersect_ray(ray, posParams.sensor);
    // save the Ray in the current buffer position
    rays.rays[ray_num * num_segments + counter] = ray;
  }
}
