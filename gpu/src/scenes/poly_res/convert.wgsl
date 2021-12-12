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

[[group(0), binding(2)]] var<uniform> params : SimParams;

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
    let screenAspect = normalize(vec2<f32>(params.height, params.width)) * 3.;

    let pos = (vec2<f32>(in.clip_position.x / params.width, in.clip_position.y / params.height) + vec2<f32>(0.55,0.5)) / screenAspect;
    let sample = textureSample(t_diffuse, s_diffuse, pos);
    var bg = vec4<f32>(0.0,0.0,0.0,1.0);

    let color = bg * (1.0 - sample.a) + sample;
    return color;
}
