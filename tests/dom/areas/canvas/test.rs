// A canvas's 2D context from Rust: shapes drawn and their pixels read back.
// Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

// The RGBA of the pixel at (x, y), read back.
fn pixel(ctx: &CanvasRenderingContext2D, x: f64, y: f64) -> [u8; 4] {
    let data = ctx.get_image_data(x, y, 1.0, 1.0).data();
    let mut rgba = [0u8; 4];
    data.read_bytes(0, &mut rgba);
    rgba
}

fn style_of(s: CanvasFillStrokeStylesFillStyle) -> String {
    match s {
        CanvasFillStrokeStylesFillStyle::String(s) => s,
        CanvasFillStrokeStylesFillStyle::CanvasGradient(_) => "<gradient>".into(),
        CanvasFillStrokeStylesFillStyle::CanvasPattern(_) => "<pattern>".into(),
    }
}

const RED: [u8; 4] = [255, 0, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let canvas: HTMLCanvasElement = doc.create_element(HTMLElementTagNameMap::CANVAS, None);
    check!(canvas.width() == 300.0 && canvas.height() == 150.0);
    canvas.set_width(100.0);
    canvas.set_height(80.0);
    check!(canvas.width() == 100.0 && canvas.height() == 80.0);
    doc.body().append_child(canvas.clone());
    let ctx = canvas.get_context_2d().unwrap();
    check!(ctx.canvas() == canvas);
    check!(!ctx.is_context_lost());

    // A fresh canvas is transparent; fillStyle is kept as a color string.
    check!(pixel(&ctx, 5.0, 5.0) == CLEAR);
    check!(style_of(ctx.fill_style()) == "#000000");
    ctx.set_fill_style("red");
    check!(style_of(ctx.fill_style()) == "#ff0000");
    ctx.set_fill_style("rgba(0, 0, 255, 0.5)");
    check!(style_of(ctx.fill_style()) == "rgba(0, 0, 255, 0.5)");
    ctx.set_fill_style("#00ff00");
    check!(style_of(ctx.fill_style()) == "#00ff00");
    // A color that does not parse is ignored.
    ctx.set_fill_style("not a color");
    check!(style_of(ctx.fill_style()) == "#00ff00");

    // fillRect, and the edges of it.
    ctx.set_fill_style("red");
    ctx.fill_rect(10.0, 10.0, 20.0, 20.0);
    check!(pixel(&ctx, 15.0, 15.0) == RED);
    check!(pixel(&ctx, 10.0, 10.0) == RED && pixel(&ctx, 29.0, 29.0) == RED);
    check!(pixel(&ctx, 9.0, 15.0) == CLEAR && pixel(&ctx, 30.0, 15.0) == CLEAR);
    // clearRect makes holes.
    ctx.clear_rect(15.0, 15.0, 5.0, 5.0);
    check!(pixel(&ctx, 16.0, 16.0) == CLEAR && pixel(&ctx, 22.0, 22.0) == RED);

    // strokeRect: the line is centered on the edge.
    ctx.set_stroke_style("#0000ff");
    ctx.set_line_width(4.0);
    check!(ctx.line_width() == 4.0);
    ctx.stroke_rect(50.0, 10.0, 30.0, 30.0);
    check!(pixel(&ctx, 65.0, 10.0) == [0, 0, 255, 255]);
    check!(pixel(&ctx, 65.0, 9.0) == [0, 0, 255, 255] && pixel(&ctx, 65.0, 11.0) == [0, 0, 255, 255]);
    check!(pixel(&ctx, 65.0, 25.0) == CLEAR);
    check!(pixel(&ctx, 65.0, 6.0) == CLEAR);

    // Alpha blends: half-transparent blue on clear is half-alpha blue.
    ctx.set_fill_style("rgba(0, 0, 255, 0.5)");
    ctx.fill_rect(0.0, 50.0, 10.0, 10.0);
    let p = pixel(&ctx, 5.0, 55.0);
    check!(p[0] == 0 && p[1] == 0 && p[2] == 255 && (127..=128).contains(&p[3]));
    ctx.set_global_alpha(0.5);
    check!(ctx.global_alpha() == 0.5);
    ctx.set_global_alpha(1.0);

    // putImageData: pixels written are the pixels read.
    let data = Uint8ClampedArray::<ArrayBuffer>::from_slice(&[1, 2, 3, 255, 10, 20, 30, 255]);
    let image = ImageData::new_with_data_with(&data, 2.0, 1.0, None);
    check!(image.width() == 2.0 && image.height() == 1.0 && image.data().len() == 8);
    ctx.put_image_data(&image, 80.0, 60.0);
    check!(pixel(&ctx, 80.0, 60.0) == [1, 2, 3, 255]);
    check!(pixel(&ctx, 81.0, 60.0) == [10, 20, 30, 255]);
    let region = ctx.get_image_data(10.0, 10.0, 20.0, 20.0);
    check!(region.width() == 20.0 && region.data().len() == 20 * 20 * 4);
    let blank = ctx.create_image_data(3.0, 2.0);
    check!(blank.data().len() == 24);
    let mut bytes = [1u8; 24];
    blank.data().read_bytes(0, &mut bytes);
    check!(bytes.iter().all(|b| *b == 0));
    // The image written is a copy: changing its data does not touch the canvas.
    image.data().write_bytes(0, &[9, 9, 9, 255]);
    check!(pixel(&ctx, 80.0, 60.0) == [1, 2, 3, 255]);

    // Paths: a filled circle, and a line.
    ctx.set_fill_style("#ff0000");
    ctx.begin_path();
    ctx.arc(60.0, 60.0, 10.0, 0.0, std::f64::consts::PI * 2.0);
    ctx.fill();
    check!(pixel(&ctx, 60.0, 60.0) == RED && pixel(&ctx, 66.0, 60.0) == RED);
    check!(pixel(&ctx, 60.0, 73.0) == CLEAR && pixel(&ctx, 71.0, 60.0) == CLEAR);
    check!(ctx.is_point_in_path(60.0, 60.0) && !ctx.is_point_in_path(60.0, 75.0));
    ctx.clear_rect(0.0, 0.0, 100.0, 80.0);
    ctx.begin_path();
    ctx.move_to(10.0, 40.5);
    ctx.line_to(90.0, 40.5);
    ctx.set_stroke_style("#000000");
    ctx.set_line_width(1.0);
    ctx.stroke();
    check!(pixel(&ctx, 50.0, 40.0) == [0, 0, 0, 255]);
    check!(pixel(&ctx, 50.0, 38.0) == CLEAR && pixel(&ctx, 5.0, 40.0) == CLEAR);
    check!(ctx.is_point_in_stroke(50.0, 40.5));
    // A closed triangle path filled with the even-odd rule, and a Path2D.
    ctx.clear_rect(0.0, 0.0, 100.0, 80.0);
    ctx.begin_path();
    ctx.move_to(10.0, 10.0);
    ctx.line_to(60.0, 10.0);
    ctx.line_to(10.0, 60.0);
    ctx.close_path();
    ctx.set_fill_style("red");
    ctx.fill_with(CanvasFillRule::Evenodd);
    check!(pixel(&ctx, 20.0, 20.0) == RED && pixel(&ctx, 55.0, 55.0) == CLEAR);
    let shape = Path2D::new();
    shape.rect(70.0, 10.0, 10.0, 10.0);
    ctx.fill_with_path(&shape);
    check!(pixel(&ctx, 75.0, 15.0) == RED);
    check!(ctx.is_point_in_path_with_path(&shape, 75.0, 15.0));
    // An SVG path string.
    let from_text = Path2D::new_with("M 0 0 h 5 v 5 h -5 z");
    ctx.save();
    ctx.translate(70.0, 40.0);
    ctx.fill_with_path(&from_text);
    ctx.restore();
    check!(pixel(&ctx, 72.0, 42.0) == RED && pixel(&ctx, 2.0, 2.0) == CLEAR);

    // save and restore bring back style and transform.
    ctx.clear_rect(0.0, 0.0, 100.0, 80.0);
    ctx.set_fill_style("#112233");
    ctx.set_line_width(3.0);
    ctx.save();
    ctx.set_fill_style("#abcdef");
    ctx.set_line_width(9.0);
    ctx.translate(5.0, 5.0);
    check!(style_of(ctx.fill_style()) == "#abcdef" && ctx.line_width() == 9.0);
    check!(ctx.get_transform().e() == 5.0);
    ctx.restore();
    check!(style_of(ctx.fill_style()) == "#112233" && ctx.line_width() == 3.0);
    check!(ctx.get_transform().e() == 0.0 && ctx.get_transform().a() == 1.0);

    // Transforms move and scale what is drawn.
    ctx.set_fill_style("red");
    ctx.save();
    ctx.translate(40.0, 20.0);
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
    check!(pixel(&ctx, 45.0, 25.0) == RED && pixel(&ctx, 5.0, 5.0) == CLEAR);
    ctx.scale(2.0, 2.0);
    ctx.fill_rect(10.0, 10.0, 5.0, 5.0);
    check!(pixel(&ctx, 62.0, 42.0) == RED && pixel(&ctx, 55.0, 35.0) == CLEAR);
    let m = ctx.get_transform();
    check!(m.a() == 2.0 && m.d() == 2.0 && m.e() == 40.0 && m.f() == 20.0);
    ctx.reset_transform();
    check!(ctx.get_transform().a() == 1.0 && ctx.get_transform().e() == 0.0);
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 3.0, 4.0);
    check!(ctx.get_transform().e() == 3.0 && ctx.get_transform().f() == 4.0);
    ctx.restore();
    check!(ctx.get_transform().e() == 0.0);
    // A rotation by a quarter turn turns a wide rect into a tall one.
    ctx.clear_rect(0.0, 0.0, 100.0, 80.0);
    ctx.save();
    ctx.translate(50.0, 40.0);
    ctx.rotate(std::f64::consts::FRAC_PI_2);
    ctx.fill_rect(0.0, 0.0, 20.0, 4.0);
    ctx.restore();
    check!(pixel(&ctx, 48.0, 55.0) == RED && pixel(&ctx, 65.0, 41.0) == CLEAR);

    // Gradients are made and used as styles.
    ctx.clear_rect(0.0, 0.0, 100.0, 80.0);
    let gradient = ctx.create_linear_gradient(0.0, 0.0, 100.0, 0.0);
    gradient.add_color_stop(0.0, "#000000");
    gradient.add_color_stop(1.0, "#ffffff");
    ctx.set_fill_style(gradient);
    check!(style_of(ctx.fill_style()) == "<gradient>");
    ctx.fill_rect(0.0, 0.0, 100.0, 10.0);
    let (left, right) = (pixel(&ctx, 2.0, 5.0), pixel(&ctx, 97.0, 5.0));
    check!(left[0] < 20 && right[0] > 235 && left[3] == 255);

    // Text: measured, and drawn.
    ctx.set_font("20px monospace");
    check!(ctx.font() == "20px monospace");
    let one = ctx.measure_text("H").width();
    let five = ctx.measure_text("HHHHH").width();
    check!(one > 5.0 && (five - one * 5.0).abs() < 0.01);
    check!(ctx.measure_text("").width() == 0.0);
    ctx.set_font("40px monospace");
    check!(ctx.measure_text("H").width() > one * 1.9);
    ctx.set_text_align(AlignSetting::Center);
    check!(ctx.text_align() == AlignSetting::Center);
    ctx.set_fill_style("#000000");
    ctx.fill_text("I", 50.0, 70.0);
    let mut any = false;
    let strip = ctx.get_image_data(30.0, 40.0, 40.0, 32.0).data();
    let mut buf = vec![0u8; strip.len()];
    strip.read_bytes(0, &mut buf);
    for px in buf.chunks(4) {
        any |= px[3] > 0;
    }
    check!(any);

    // Line styles and the dash.
    ctx.set_line_cap(CanvasLineCap::Round);
    check!(ctx.line_cap() == CanvasLineCap::Round);
    ctx.set_line_dash([4.0, 2.0]);
    check!(ctx.get_line_dash().iter().collect::<Vec<f64>>() == [4.0, 2.0]);
    ctx.set_global_composite_operation(GlobalCompositeOperation::Copy);
    check!(ctx.global_composite_operation() == GlobalCompositeOperation::Copy);

    // The canvas as an image.
    let url = canvas.to_data_url();
    check!(url.starts_with("data:image/png;base64,") && url.len() > 100);
    check!(canvas.to_data_url_with("image/jpeg", 0.5).starts_with("data:image/jpeg;base64,"));
    // Resizing a canvas clears it.
    ctx.reset_transform();
    ctx.set_global_composite_operation(GlobalCompositeOperation::SourceOver);
    ctx.set_fill_style("red");
    ctx.fill_rect(0.0, 0.0, 100.0, 80.0);
    check!(pixel(&ctx, 50.0, 40.0) == RED);
    canvas.set_width(100.0);
    check!(pixel(&ctx, 50.0, 40.0) == CLEAR);
    ctx.reset();
    check!(style_of(ctx.fill_style()) == "#000000" && ctx.line_width() == 1.0);
}
