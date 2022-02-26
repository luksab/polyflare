use tiny_skia::*;

mod line;
use line::*;

use polynomial_optics::*;

fn main() {
    // let mut pixmap = Pixmap::new(500, 500).unwrap();

    // let line = Line::new_dashed(10., 10., 400., 300.);
    // line.draw(&mut pixmap);

    // pixmap.save_png("image.png").unwrap();

    /*
    let f = Polynom2d {
        coefficients: [[3.0, 2.0], [1.0, 4.0]],
    };

    let g = Polynom2d {
        coefficients: [[382., 47.], [3.86285, 1.0]],
    };

    println!("          f(x) = {}", f);
    println!("          g(x) = {}", g);
    println!("       f(3, 2) = {}", f.eval(3., 2.));

    println!("     f(x)+g(x) = {}", f + g);
    println!("     f(x)-g(x) = {}", f - g);

    println!("f(x)+g(x)-g(x) = {}", f + g - g);

    assert_eq!(f, f + g - g);

    */

    let basis = LegendreBasis::new(4);
    println!("basis = {}", basis);

    println!("lut: {:?}", basis.get_luts(10));

    for index in 0..Legendre4d::num_polys(3) {
        let (i, j, k, l) = Legendre4d::poly_index_to_multi_index(index, 3).unwrap();
        println!("index: {}, i: {}, j: {}, k: {}, l: {}", index, i, j, k, l);
        println!(
            "index: {}",
            Legendre4d::poly_multi_index_to_index(i, j, k, l, 3).unwrap()
        );
    }

    let points = vec![
        (-1., -1., -1., -1., 0.),
        (0., 0., 0., 0., 0.5),
        (1., 1., 1., 1., 1.),
    ];

    let mut legendre = Legendre4d::new(1);

    println!("legendre: {}", legendre);
    legendre.fit(&points);
    println!("legendre: {}", legendre);

    for point in points {
        println!("point: {:?}", point);
        let point = (point.0, point.1, point.2, point.3);
        println!("eval: {}", legendre.eval(&point));
    }

    return;
    let part = Monomial {
        coefficient: 1.0,
        exponents: [2, 4, 3],
    };
    let part2 = Monomial {
        coefficient: 0.5,
        exponents: [1, 3, 0],
    };
    println!("part: {}", part);
    println!("part2: {}", part2);

    let pol = Polynomial::new(vec![part, part2]);
    println!("pol: {}", pol);

    println!("multiplied with itself: {}", &pol * &pol);

    // ray tracing
    // let mut pixmap = Pixmap::new(400, 200).unwrap();
    let mut pixmap = Pixmap::new(4000, 2000).unwrap();
    pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

    let mut line = Line::new(10., 10., 400., 300.);

    let radius = 3.0;
    let lens_entry = Element {
        radius,
        properties: Properties::Glass(Glass {
            sellmeier: Sellmeier::bk7(),
            coating: QuarterWaveCoating::none(), //optimal(1.5, 1.0, 0.5),
            entry: true,
            outer_ior: Sellmeier::air(),
            spherical: true,
        }),
        position: -2.0,
    };
    let lens_exit_pos = 1.0;
    let lens_exit = Element {
        radius,
        properties: Properties::Glass(Glass {
            sellmeier: Sellmeier::bk7(),
            coating: QuarterWaveCoating::none(), //optimal(1.5, 1.0, 0.5),
            entry: false,
            outer_ior: Sellmeier::air(),
            spherical: true,
        }),
        position: lens_exit_pos,
    };
    line.width = 3.0;
    // lens entry
    line.draw_circle(&mut pixmap, -radius as f32 - 2.0, 0., radius as f32);

    // lens exit
    line.color = Color::from_rgba8(127, 127, 127, 255);
    line.draw_circle(
        &mut pixmap,
        (-3.) * radius as f32 + lens_exit_pos as f32,
        0.,
        radius as f32,
    );
    line.width = 1.;

    println!("lens: {:?}", lens_entry);
    // let wave_num = 10;
    // for wavelen in 0..wave_num {
    //     let wavelength = 0.38 + wavelen as f64 * ((0.78 - 0.38) / wave_num as f64);
    //     println!("l: {}, ior: {}", wavelength, Sellmeier::BK7().ior(wavelength));
    // }
    //println!("ray: {:?}", ray);

    let lens = Lens::new(vec![lens_entry, lens_exit], 3.);
    lens.draw(&mut pixmap);

    pixmap.save_png("image.png").unwrap();

    println!("{:?}", QuarterWaveCoating::optimal(1.5, 1.0, 0.5));
}
