struct Cell {
  number: u32;
  alive : u32;
};

[[block]]
struct SimParams {
  side_len: u32;
  width: u32;
  height: u32;
};

[[block]]
struct Cells {
  cells : [[stride(8)]] array<Cell>;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;
[[group(0), binding(1)]] var<storage, read> cellsSrc : Cells;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(3)]] alive: u32;
};

[[stage(vertex)]]
fn main(
    [[location(0)]] position: vec2<f32>,
) -> VertexOutput {
    // let grid_pos = vec2<u32>(cell_num%params.side_len, cell_num/params.side_len);
    // let offset = vec2<f32>(
    //     f32(grid_pos[0])/f32(params.side_len), 
    //     f32(grid_pos[1])/f32(params.side_len)) * 2.f32;
    // let pos = (position + offset - vec2<f32>(0.9,0.9));

    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 0.1);
    out.alive = 1u32;
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let x = u32(in.clip_position.x * f32(params.side_len) / f32(params.width));
    let y = u32(in.clip_position.y * f32(params.side_len) / f32(params.height));
    if (cellsSrc.cells[(x + y * params.side_len) % (params.side_len*params.side_len)].alive != 0u32){
        return vec4<f32>(1.0,1.0,1.0,1.0);
    } else {
        return vec4<f32>(0.0,0.0,0.0,1.0);
    }
}