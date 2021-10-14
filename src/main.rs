use tiny_skia::*;

mod line;
use line::*;

mod polynom;

fn main() {
    let mut pixmap = Pixmap::new(500, 500).unwrap();

    let line = Line::new_dashed(10., 10., 400., 300.);
    line.draw(&mut pixmap);

    pixmap.save_png("image.png").unwrap();
}
