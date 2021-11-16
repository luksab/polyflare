struct Ray {
  o: vec3<f32>;
  d : vec3<f32>;
  strength: f32;
};

[[block]]
struct SimParams {
  side_len: u32;
};

[[block]]
struct Rays {
  rays : [[stride(32)]] array<Ray>;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;
[[group(0), binding(1)]] var<storage, read_write> rays : Rays;

// fn grid_to_index(grid_pos: vec2<u32>) -> u32 {
//   return grid_pos[0] + grid_pos[1] * params.side_len;
// }

// fn index_to_grid(index: u32) -> vec2<u32> {
//   let grid_pos = vec2<u32>(index%params.side_len, index/params.side_len);
//   return grid_pos;
// }

// fn count_neighbours(pos: vec2<i32>) -> u32 {
//   var count = u32(0);
//   for (var i: i32 = -1; i <= 1; i = i + 1) {
//     for (var j: i32 = -1; j <= 1; j = j + 1) {
//       if (i != 0 || j != 0) {
//         if (cellsSrc.cells[grid_to_index(vec2<u32>(pos - vec2<i32>(i,j)))].alive > u32(0)) {
//           count = count + u32(1);
//         }
//       }
//     }
//   }
//   return count;
// }

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let total = arrayLength(&rays.rays);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  rays.rays[index].strength = 1.;

  // var old : u32 = cellsSrc.cells[index].alive;

  // var alive = u32(0);

  // let neighbours = count_neighbours(vec2<i32>(index_to_grid(index)));
  // // if (neighbours < 2u32) {
  // //   alive = u32(0);
  // // } else { if (neighbours == 3u32 && old == u32(0)) {
  // //   alive = u32(1);
  // // } else { if (neighbours == 2u32 || neighbours == 3u32) {
  // //   alive = old;
  // // } else { if (neighbours > 3u32) {
  // //   alive = u32(0);
  // // }}}}

  // if ((old != u32(0) && neighbours == u32(2) || neighbours == u32(3)) || 
  //     (old == u32(0) && neighbours == u32(3))) {
  //   alive = u32(1);
  // } else {
  //   alive = u32(0);
  // }

  // // Write back
  // cellsDst.cells[index].alive = alive;
}
