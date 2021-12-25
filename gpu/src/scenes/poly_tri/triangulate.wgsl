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

[[block]]
struct SimParams {
  opacity: f32;
  /// scaled for high dpi screens
  width_scaled: f32;
  height_scaled: f32;
  width: f32;
  height: f32;
  draw_mode: f32;
  which_ghost: f32;
};

[[block]]
// static parameters for positions
struct PosParams {
  // the Ray to be modified as a base for ray tracing
  init: Ray;
  // position of the sensor in the optical plane
  sensor: f32;
  width: f32;
};

[[block]]
struct Rays {
  rays: [[stride(32)]] array<Ray>;
};

[[block]]
/// all the Elements of the Lens under test
struct Elements {
  el: [[stride(72)]] array<Element>;
};

[[group(0), binding(0)]] var<storage, read_write> rays : Rays;
[[group(1), binding(2)]] var<uniform> params : SimParams;

[[group(1), binding(0)]] var<uniform> posParams : PosParams;

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
    ray.o.x = intersect.x;
    ray.o.y = intersect.y;
    return ray;
}

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let draw_mode = u32(params.draw_mode);//u32(1);
  let which_ghost = u32(params.which_ghost);//u32(1);

  // the total number of possible shader executions
  let total = arrayLength(&rays.rays);
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
}
