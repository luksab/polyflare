/// one Ray with origin, direction, and strength
struct Ray {
  o: vec3<f32>;
  d: vec3<f32>;
  strength: f32;
};

struct Element {
  position1: f32;
  radius1  : f32;
  position2: f32;
  radius2  : f32;
};

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
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


struct Elements {
  el : [[stride(16)]] array<Element>;
};


// static parameters for positions
struct PosParams {
  // the Ray to be modified as a base for ray tracing
  init: Ray;
  // position of the sensor in the optical plane
  sensor: f32;
  width: f32;
};

[[group(1), binding(2)]] var<uniform> params : SimParams;
[[group(2), binding(1)]] var<storage, read> elements : Elements;

[[group(1), binding(0)]] var<uniform> posParams : PosParams;

[[stage(vertex)]]
fn mainv(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.,1.);
    return out;
}

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn mainf(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let pos = vec2<f32>(in.clip_position.x / params.width, in.clip_position.y / params.height);
    let sample = textureSample(t_diffuse, s_diffuse, pos);
    var bg = vec4<f32>(0.15,0.15,0.25,1.0);

    let screenAspect = normalize(vec2<f32>(params.height, params.width));
    let pos_lens = (pos - vec2<f32>(0.5,0.5))/screenAspect * 2.;

    for (var i: u32 = u32(0); i < arrayLength(&elements.el); i = i + u32(1)) {
        let element = elements.el[i];
        let c1 = vec2<f32>(element.position1/4.0, 0.0);
        let c2 = vec2<f32>(element.position2/4.0, 0.0);
        if (element.radius1 > 0. && element.radius2 > 0.) {
            if ( distance(c1, pos_lens) <= element.radius1 / 4.0 
                && distance(c2, pos_lens) <= element.radius2 / 4.0 ) {
                bg = vec4<f32>(0.3,0.3,0.4,1.0);
            }
        } 
        if (element.radius1 <= 0. && element.radius2 > 0.) {
            if ( distance(c1, pos_lens) > - element.radius1 / 4.0 
                && distance(c2, pos_lens) <= element.radius2 / 4.0 ) {
                bg = vec4<f32>(0.3,0.3,0.4,1.0);
            }
        }
        if (element.radius1 > 0. && element.radius2 <= 0.) {
            if ( distance(c1, pos_lens) <= element.radius1 / 4.0 
                && distance(c2, pos_lens) > - element.radius2 / 4.0 ) {
                bg = vec4<f32>(0.3,0.3,0.4,1.0);
            }
        }
        if (element.radius1 <= 0. && element.radius2 <= 0.) {
            if ( distance(c1, pos_lens) > - element.radius1 / 4.0 
                && distance(c2, pos_lens) > - element.radius2 / 4.0 
                && (pos_lens.y > element.radius1/4.0 && pos_lens.y < -element.radius1/4.0)
                && (pos_lens.x - element.position1/4.0 > 0.) && (pos_lens.x - element.position2/4.0 < 0.)) {
                bg = vec4<f32>(0.3,0.3,0.4,1.0);
            }
        }
    }

    let plane = posParams.sensor / 4.;//(posParams.sensor + 1.8) / 4.5;
    if (plane > pos_lens.x && plane < pos_lens.x + .01){
        bg = vec4<f32>(0.3,0.3,0.4,1.0);
    }

    let color = bg * (1.0 - sample.a) + sample;
    return color;
}
