struct Ray {
  o: vec3<f32>;
  d: vec3<f32>;
  strength: f32;
};

struct Element {
  radius: f32;
  glass: f32;
  position: f32;
  entry: f32;// 0: false, 1: true
  spherical: f32;// 0: false, 1: true
};

[[block]]
struct SimParams {
  opacity: f32;
  width: f32;
  height: f32;
};

[[block]]
struct Rays {
  rays : [[stride(32)]] array<Ray>;
};

[[block]]
struct Elements {
  el : [[stride(20)]] array<Element>;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;
[[group(0), binding(1)]] var<storage, read_write> rays : Rays;
[[group(0), binding(2)]] var<storage, read> elements : Elements;

fn fresnel_r(t1: f32, t2: f32, n1: f32, n2: f32) -> f32 {
  let s = 0.5 * ((n1 * cos(t1) - n2 * cos(t2)) / (n1 * cos(t1) + n2 * cos(t2))) * ((n1 * cos(t1) - n2 * cos(t2)) / (n1 * cos(t1) + n2 * cos(t2)));
  let p = 0.5 * ((n1 * cos(t2) - n2 * cos(t1)) / (n1 * cos(t2) + n2 * cos(t1))) * ((n1 * cos(t2) - n2 * cos(t1)) / (n1 * cos(t2) + n2 * cos(t1)));
  return s + p;
}

fn propagate_element(
    self: Ray,
    radius: f32,
    glass: f32,
    position: f32,
    reflect: bool,
    entry: bool,
    cylindrical: bool,
) -> Ray{
    var ray = self;
    var intersection: vec3<f32>;
    if (cylindrical) {
        // cylindrical: x is not affected by curvature

        // c: center of the lens surface if interpreted as an entire sphere
        var cy: f32;
        if (entry) {
            cy = position + radius;
        } else {
            cy = position - radius;
        };
        let c = vec2<f32>(0., cy);
        let o = vec2<f32>(ray.o.y,ray.o.z);
        let d = normalize(vec2<f32>(ray.d.y, ray.d.z));
        let delta = dot(d, o - c) * dot(d, o - c)
                    - (length(o - c) * length(o - c) - radius * radius);

        let d1 = -(dot(d, o - c)) - sqrt(delta);
        let d2 = -(dot(d, o - c)) + sqrt(delta);

        if ((entry == (ray.d.z > 0.)) == (radius > 0.)) {
            intersection = ray.o + ray.d * d1;
        } else {
            intersection = ray.o + ray.d * d2;
        }
    } else {
        // c: center of the lens surface if interpreted as an entire sphere
        var cz: f32;
        if (entry) {
            cz = position + radius;
        } else {
            cz = position - radius;
        };
        let c = vec3<f32>(0., 0., cz);

        let delta = dot(ray.d, ray.o - c) * dot(ray.d, ray.o - c)
                    - (length(ray.o - c) * length(ray.o - c) - radius * radius);

        let d1 = -(dot(ray.d, ray.o - c)) - sqrt(delta);
        let d2 = -(dot(ray.d, ray.o - c)) + sqrt(delta);

        if ((entry == (ray.d.z > 0.)) == (radius > 0.)) {
            intersection = ray.o + ray.d * d1;
        } else {
            intersection = ray.o + ray.d * d2;
        }
    };

    ray.o = intersection;

    var normal: vec3<f32>;
    if (cylindrical) {
        var cy: f32;
        if (entry) {
            cy = position + radius;
        } else {
            cy = position - radius;
        };
        let c = vec2<f32>(0., cy);

        let intersection_2d = normalize(vec2<f32>(intersection.y, intersection.z));

        let normal2d = intersection_2d - c;

        let intersection_3d = vec3<f32> (0.0, normal2d.x, normal2d.y);

        if ((entry == (ray.d.z > 0.)) == (radius > 0.)) {
            normal = normalize(intersection_3d);
        } else {
            normal = -(normalize(intersection_3d));
        }
    } else {
        var cz: f32;
        if (entry) {
            cz = position + radius;
        } else {
            cz = position - radius;
        };
        let c = vec3<f32>(0., 0., cz);

        if ((entry == (ray.d.z > 0.)) == (radius > 0.)) {
            normal = normalize((intersection - c));
        } else {
            normal = -(normalize(intersection - c));
        }
    };

    if (reflect) {
        let d_in = ray.d;

        ray.d = ray.d - 2.0 * dot(normal, ray.d) * normal;

        var a: f32;
        if (entry == (ray.d.z > 0.)) {
            a = glass;
        } else {
            a = 1.0;
        };
        var b: f32;
        if (entry == (ray.d.z > 0.)) {
            b = 1.0;
        } else {
            b = glass;
        }

        ray.strength = ray.strength * fresnel_r(
            acos(dot(normalize(d_in), normal)),
            acos(dot(normalize(ray.d), -normal)),
            a,
            b,
        );
    } else {
        var eta: f32;
        if (entry) { eta = 1.0 / glass; } else { eta = glass; };

        // from https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/refract.xhtml
        let k = 1.0 - eta * eta * (1.0 - dot(normal, ray.d) * dot(normal, ray.d));

        let d_in = ray.d;

        if (k < 0.0) {
            // total reflection
            // println!("total reflection");
            ray.d = ray.d * 0.0; // or genDType(0.0)
        } else {
            ray.d = eta * ray.d - (eta * dot(normal, ray.d) + sqrt(k)) * normal;
        }

        var a: f32;
        if (entry == (ray.d.z > 0.)) {
            a = glass;
        } else {
            a = 1.0;
        };
        var b: f32;
        if (entry == (ray.d.z > 0.)) {
            b = 1.0;
        } else {
            b = glass;
        }
        ray.strength = ray.strength * (1.0
            - fresnel_r(
                acos(dot(normalize(d_in), -normal)),
                acos(dot(normalize(ray.d), -normal)),
                b,
                a,
            ));
    }
    return ray;
}

/// propagate a ray through an element
///
fn propagate(self: Ray, element: Element) -> Ray {
    return propagate_element(
        self,
        element.radius,
        element.glass,
        element.position,
        false,
        element.entry > 0.,
        !(element.spherical > 0.),
    );
}

/// reflect a Ray from an element
///
fn reflect(self: Ray, element: Element) -> Ray {
    return propagate_element(
        self,
        element.radius,
        element.glass,
        element.position,
        true,
        element.entry > 0.,
        !(element.spherical > 0.),
    );
}

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let num_segments = arrayLength(&elements.el) * u32(2) + u32(2);
  let total = arrayLength(&rays.rays) / num_segments;
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

//   rays.rays[index].o = vec3<f32>(params.opacity * f32(index) * 0.01, 1., 1.);
//   rays.rays[index].d = vec3<f32>(1., 1., 1.);
//   rays.rays[index].strength = 1.;

  let num_rays = total;
  let ray_num = index;
  let width = 2.;

  let center_pos = vec3<f32>(0.,0.7,-10.);
  let direction = vec3<f32>(0.,0.,1.);

  var pos = center_pos;
  pos.y = pos.y + f32(ray_num) / f32(num_rays) * width - width / 2.;
  // struct Ray {
  //   o: vec3<f32>;
  //   d : vec3<f32>;
  //   strength: f32;
  // };
  var ray = Ray(pos, direction, 1.0);
//   rays.rays[ray_num * num_segments] = ray;
  for (var i: u32 = u32(0); i < arrayLength(&elements.el); i = i + u32(1)) {
    let element = elements.el[i];
    rays.rays[ray_num * num_segments + i * u32(2)] = ray;
    ray = propagate(ray, element);
    rays.rays[ray_num * num_segments + i * u32(2) + u32(1)] = ray;
  }
  rays.rays[(ray_num + u32(1)) * num_segments - u32(2)] = ray;
  ray.o = ray.o + ray.d * 100.;
  rays.rays[(ray_num + u32(1)) * num_segments - u32(1)] = ray;
//   rays.rays[(ray_num + u32(1)) * num_segments - u32(1)] = ray;

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
