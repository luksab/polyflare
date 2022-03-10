struct VertexInput {
    [[location(0)]] position: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

struct InitRay {
  o: vec3<f32>;
  wavelength: f32;
  d: vec3<f32>;
  strength: f32;
};
// static parameters for positions
struct PosParams {
  // the Ray to be modified as a base for ray tracing
  init: InitRay;
  // position of the sensor in the optical plane
  sensor: f32;
  width: f32;
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

struct PolyParams {
    num_terms: u32;
};

struct Monomial {
    a: f32;
    b: f32;
    c: f32;
    d: f32;
    coefficient: f32;
};

struct Polynomial {
    monomials: [[stride(20)]] array<Monomial>;
};

[[group(1), binding(2)]] var<uniform> params : SimParams;
[[group(1), binding(0)]] var<uniform> posParams : PosParams;

[[group(2), binding(0)]] var<storage, read> terms : Polynomial;
[[group(2), binding(1)]] var<uniform> polyParams : PolyParams;

fn eval(x: vec4<f32>, index: u32) -> f32 {
    var res = 0.;
    for (var i = u32(index * polyParams.num_terms); i < (index + u32(1)) * polyParams.num_terms; i = i + u32(1)) {
        let term = terms.monomials[i];
        if (term.coefficient > 0. || term.coefficient < 0.) {
            var term_res = term.coefficient;
            if (term.a > 0.) {
                if (i32(term.a) % 2 == 1 && x.x < 0.) {
                    term_res = -term_res * pow(-x.x, term.a);
                } else {
                    term_res = term_res * pow(abs(x.x), term.a);
                }
            }
            if (term.b > 0.) {
                if (i32(term.b) % 2 == 1 && x.y < 0.) {
                    term_res = -term_res * pow(-x.y, term.b);
                } else {
                    term_res = term_res * pow(abs(x.y), term.b);
                }
            }
            if (term.c > 0.) {
                if (i32(term.c) % 2 == 1 && x.z < 0.) {
                    term_res = -term_res * pow(-x.z, term.c);
                } else {
                    term_res = term_res * pow(abs(x.z), term.c);
                }
            }
            if (term.d > 0.) {
                if (i32(term.d) % 2 == 1 && x.w < 0.) {
                    term_res = -term_res * pow(-x.w, term.d);
                } else {
                    term_res = term_res * pow(abs(x.w), term.d);
                }
            }
            res = res + term_res;
        }
    }
    return res;
}

fn eval_grad_zw(x: vec4<f32>, index: u32) -> f32 {
    var dz = vec2<f32>(0.);
    var dw = vec2<f32>(0.);
    let poyly_index = index * u32(2) * polyParams.num_terms;
    // dc z
    for (var i = u32(poyly_index); i < polyParams.num_terms + poyly_index; i = i + u32(1)) {
        let term = terms.monomials[i];
        if (term.coefficient != 0.) {
            var term_res = term.coefficient;
            if (term.c == 0.) {
                term_res = 0.;
            } else {
                if (term.c > 1.) {
                    if (i32(term.c - 1.) % 2 == 1 && x.z < 0.) {
                        term_res = -term_res * term.c * pow(-x.z, term.c - 1.);
                    } else {
                        term_res = term_res * term.c * pow(abs(x.z), term.c - 1.);
                    }
                }
                if (term.a > 0.) {
                    if (i32(term.a) % 2 == 1 && x.x < 0.) {
                        term_res = -term_res * pow(-x.x, term.a);
                    } else {
                        term_res = term_res * pow(abs(x.x), term.a);
                    }
                }
                if (term.b > 0.) {
                    if (i32(term.b) % 2 == 1 && x.y < 0.) {
                        term_res = -term_res * pow(-x.y, term.b);
                    } else {
                        term_res = term_res * pow(abs(x.y), term.b);
                    }
                }
                if (term.d > 0.) {
                    if (i32(term.d) % 2 == 1 && x.w < 0.) {
                        term_res = -term_res * pow(-x.w, term.d);
                    } else {
                        term_res = term_res * pow(abs(x.w), term.d);
                    }
                }
            }
            dz.x = dz.x + term_res;
        }
    }

    // dc w
    for (var i = u32(poyly_index) + polyParams.num_terms; i < u32(2) * polyParams.num_terms + poyly_index; i = i + u32(1)) {
        let term = terms.monomials[i];
        if (term.coefficient != 0.) {
            var term_res = term.coefficient;
            if (term.c == 0.) {
                term_res = 0.;
            } else {
                if (term.c > 1.) {
                    if (i32(term.c - 1.) % 2 == 1 && x.z < 0.) {
                        term_res = -term_res * term.c * pow(-x.z, term.c - 1.);
                    } else {
                        term_res = term_res * term.c * pow(abs(x.z), term.c - 1.);
                    }
                }
                if (term.a > 0.) {
                    if (i32(term.a) % 2 == 1 && x.x < 0.) {
                        term_res = -term_res * pow(-x.x, term.a);
                    } else {
                        term_res = term_res * pow(abs(x.x), term.a);
                    }
                }
                if (term.b > 0.) {
                    if (i32(term.b) % 2 == 1 && x.y < 0.) {
                        term_res = -term_res * pow(-x.y, term.b);
                    } else {
                        term_res = term_res * pow(abs(x.y), term.b);
                    }
                }
                if (term.d > 0.) {
                    if (i32(term.d) % 2 == 1 && x.w < 0.) {
                        term_res = -term_res * pow(-x.w, term.d);
                    } else {
                        term_res = term_res * pow(abs(x.w), term.d);
                    }
                }
            }
            dz.y = dz.y + term_res;
        }
    }

    // dd z
    for (var i = u32(poyly_index); i < polyParams.num_terms + poyly_index; i = i + u32(1)) {
        let term = terms.monomials[i];
        if (term.coefficient != 0.) {
            var term_res = term.coefficient;
            if (term.d == 0.) {
                term_res = 0.;
            } else {
                if (term.d > 1.) {
                    if (i32(term.d - 1.) % 2 == 1 && x.w < 0.) {
                        term_res = -term_res * term.d * pow(-x.w, term.d - 1.);
                    } else {
                        term_res = term_res * term.d * pow(abs(x.w), term.d - 1.);
                    }
                }
                if (term.a > 0.) {
                    if (i32(term.a) % 2 == 1 && x.x < 0.) {
                        term_res = -term_res * pow(-x.x, term.a);
                    } else {
                        term_res = term_res * pow(abs(x.x), term.a);
                    }
                }
                if (term.b > 0.) {
                    if (i32(term.b) % 2 == 1 && x.y < 0.) {
                        term_res = -term_res * pow(-x.y, term.b);
                    } else {
                        term_res = term_res * pow(abs(x.y), term.b);
                    }
                }
                if (term.c > 0.) {
                    if (i32(term.c) % 2 == 1 && x.z < 0.) {
                        term_res = -term_res * pow(-x.z, term.c);
                    } else {
                        term_res = term_res * pow(abs(x.z), term.c);
                    }
                }
            }
            dw.x = dw.x + term_res;
        }
    }

    // dd w
    for (var i = u32(poyly_index) + polyParams.num_terms; i < u32(2) * polyParams.num_terms + poyly_index; i = i + u32(1)) {
        let term = terms.monomials[i];
        if (term.coefficient != 0.) {
            var term_res = term.coefficient;
            if (term.d == 0.) {
                term_res = 0.;
            } else {
                if (term.d > 1.) {
                    if (i32(term.d - 1.) % 2 == 1 && x.w < 0.) {
                        term_res = -term_res * term.d * pow(-x.w, term.d - 1.);
                    } else {
                        term_res = term_res * term.d * pow(abs(x.w), term.d - 1.);
                    }
                }
                if (term.a > 0.) {
                    if (i32(term.a) % 2 == 1 && x.x < 0.) {
                        term_res = -term_res * pow(-x.x, term.a);
                    } else {
                        term_res = term_res * pow(abs(x.x), term.a);
                    }
                }
                if (term.b > 0.) {
                    if (i32(term.b) % 2 == 1 && x.y < 0.) {
                        term_res = -term_res * pow(-x.y, term.b);
                    } else {
                        term_res = term_res * pow(abs(x.y), term.b);
                    }
                }
                if (term.c > 0.) {
                    if (i32(term.c) % 2 == 1 && x.z < 0.) {
                        term_res = -term_res * pow(-x.z, term.c);
                    } else {
                        term_res = term_res * pow(abs(x.z), term.c);
                    }
                }
            }
            dw.y = dw.y + term_res;
        }
    }

    return 1. / (dz.x * dw.y - dz.y * dw.x);
}


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
    let w = params.window_width_scaled;
    let h = params.window_height_scaled;
    let ratio = max(w,h);
    var pos = vec2<f32>(in.clip_position.x - w/2., in.clip_position.y - h/2.) / ratio + 0.5;
    let debug = false;
    if (debug) {
        pos.x = clamp(pos.x, 0., 1.);
        pos.y = clamp(pos.y, 0., 1.);
        pos = pos * posParams.width - posParams.width/2.;
    }
    let sample = textureSample(t_diffuse, s_diffuse, pos);
    var bg = vec4<f32>(0.0,0.0,0.0,1.0);

    var bright = 0.;
    let num_polys = arrayLength(&terms.monomials) / polyParams.num_terms;
    // for (var i = u32(0); i < num_polys; i = i + u32(1)) {
    //     let poly_res = eval(vec4<f32>(0., 0., pos.x, pos.y), i);
    //     bright = bright + abs(poly_res);
    // }
    // let bright = 100. * pos.x * pos.y;
    if (debug) {
        let x = vec4<f32>(posParams.init.o.xy, pos.x, pos.y);
        // bright = eval(x, u32(params.which_ghost));
        bright = eval_grad_zw(x, u32(params.which_ghost)) * 1000.;
        // bright = eval(x, u32(params.which_ghost * 2.) + u32(0));
    }

    if (debug) {
        return vec4<f32>( vec3<f32>(-bright * 0.01 * params.opacity, bright * 0.01 * params.opacity, 0.), 1.);
    } else {
        return bg * (1.0 - sample.a) + sample;
    }
}
