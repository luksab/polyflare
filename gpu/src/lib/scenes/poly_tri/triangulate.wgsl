/// one Ray with origin, direction, and strength
struct Ray {
  o: vec3<f32>;
  wavelength: f32;
  d: vec3<f32>;
  strength: f32;
};

struct DrawRay {
  pos: vec2<f32>;
  aperture_pos: vec2<f32>;
  entry_pos: vec2<f32>;
  strength: f32;
  wavelength: f32;
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
  rays: [[stride(32)]] array<DrawRay>;
};

/// all the Elements of the Lens under test
struct Elements {
  el: [[stride(72)]] array<Element>;
};

[[group(0), binding(0)]] var<storage, read_write> rays : Rays;
[[group(1), binding(2)]] var<uniform> params : SimParams;

[[group(1), binding(0)]] var<uniform> posParams : PosParams;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let index = global_invocation_id.x % (u32(params.side_len) * u32(params.side_len));
  let ghost_num = global_invocation_id.x / (u32(params.side_len) * u32(params.side_len));
  let offset = ghost_num * (u32(params.side_len) * u32(params.side_len));

  if (ghost_num >= arrayLength(&rays.rays)) {
    return;
  }

  let ray_num = index;

  // we need the sqrt to scale the movement in each direction by
  let dot_side_len = u32(params.side_len);
  let x = ray_num / dot_side_len;
  let y = ray_num % dot_side_len;

  var averageArea = 0.;
  var num_areas = 0;
  let self = rays.rays[(x + y * dot_side_len) + offset].pos;
  // 1:(x  ,y  ),(x-1,y-1),(x  ,y-1)
  // 2:(x  ,y  ),(x  ,y-1),(x+1,y  )
  // 3:(x  ,y  ),(x+1,y  ),(x+1,y+1)
  // 4:(x  ,y  ),(x+1,y+1),(x  ,y+1)
  // 5:(x  ,y  ),(x  ,y+1),(x-1,y  )
  // 7:(x  ,y  ),(x-1,y  ),(x-1,y-1)

  // basically irrelevant for 8 elements in terms of performance
  // whether to use one or all if statements
  if (x > u32(0) && y > u32(0)){// 1
    let b = rays.rays[(x - u32(1) + (y - u32(1)) * dot_side_len) + offset].pos;
    let c = rays.rays[(x + (y - u32(1)) * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }   
  }

  if (x < dot_side_len - u32(1) && y > u32(0)){// 2
    let b = rays.rays[(x + (y - u32(1)) * dot_side_len) + offset].pos;
    let c = rays.rays[(x + u32(1) + y * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }   
  }

  if (x < dot_side_len - u32(1) && y < dot_side_len - u32(1)){// 3
    let b = rays.rays[(x + u32(1) + y * dot_side_len) + offset].pos;
    let c = rays.rays[(x + u32(1) + (y + u32(1)) * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }
  }

  if (x < dot_side_len - u32(1) && y < dot_side_len - u32(1)){// 4
    let b = rays.rays[(x + u32(1) + (y + u32(1)) * dot_side_len) + offset].pos;
    let c = rays.rays[(x + (y + u32(1)) * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }
  }

  if (x > u32(0) && y < dot_side_len - u32(1)){// 5
    let b = rays.rays[(x + (y + u32(1)) * dot_side_len) + offset].pos;
    let c = rays.rays[(x - u32(1) + y * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }
  }

  if (x > u32(0) && y > u32(0)){// 6
    let b = rays.rays[(x - u32(1) + y * dot_side_len) + offset].pos;
    let c = rays.rays[(x - u32(1) + (y - u32(1)) * dot_side_len) + offset].pos;

    let s1 = self-b;
    let s2 = self-c;
    let area = abs(s1.x * s2.y - s1.y * s2.x);
    if (area > 0.){
      averageArea = averageArea + area;
      num_areas = num_areas + 1;  
    }
  }

  averageArea = averageArea / f32(num_areas);

  if (!(rays.rays[(x + y * dot_side_len) + offset].strength > 0.)){
    rays.rays[(x + y * dot_side_len) + offset].strength = 0.;
  }

  rays.rays[(x + y * dot_side_len) + offset].strength = rays.rays[(x + y * dot_side_len) + offset].strength / averageArea;
}
