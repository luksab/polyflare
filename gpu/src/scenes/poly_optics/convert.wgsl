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

[[block]]
struct SimParams {
  opacity: f32;
  width_scaled: f32;
  height_scaled: f32;
  width: f32;
  height: f32;
};

[[block]]
struct Elements {
  el : [[stride(16)]] array<Element>;
};

[[group(0), binding(2)]] var<uniform> params : SimParams;
[[group(0), binding(3)]] var<storage, read> elements : Elements;

[[stage(vertex)]]
fn main(
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
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let pos = vec2<f32>(in.clip_position.x / params.width, in.clip_position.y / params.height);
    let sample = textureSample(t_diffuse, s_diffuse, pos);
    var bg = vec4<f32>(0.2,0.2,0.3,1.0);

    let screenAspect = normalize(vec2<f32>(params.height, params.width));
    let pos_lens = (pos - vec2<f32>(0.5,0.5))/screenAspect * 2.;

    for (var i: u32 = u32(0); i <= arrayLength(&elements.el); i = i + u32(1)) {
        let element = elements.el[i];
        let c1 = vec2<f32>(element.position1/4.0, 0.0);
        let c2 = vec2<f32>(element.position2/4.0, 0.0);
        if ( distance(c1, pos_lens) <= element.radius1 / 4.0 
            && distance(c2, pos_lens) <= element.radius2 / 4.0 ) {
            bg = vec4<f32>(0.1,0.1,0.15,1.0);
        }
    }

    let color = bg * (1.0 - sample.a) + sample;
    return color;
}
