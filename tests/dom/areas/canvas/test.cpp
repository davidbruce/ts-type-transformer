// A canvas's 2D context from C++: rectangles, pixels, paths, text, state and
// transforms. Every CHECK must hold.
#include <string>
#include <vector>

#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"
#include "areas/canvas/CanvasRenderingContext2D.hpp"
#include "areas/canvas/ImageData.hpp"
#include "areas/canvas/TextMetrics.hpp"
#include "areas/html/HTMLCanvasElement.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

struct Pixel {
    int r, g, b, a;
    bool operator==(const Pixel&) const = default;
};

// One pixel of the canvas, through getImageData.
static Pixel pixel(const CanvasRenderingContext2D& ctx, double x, double y) {
    ImageData img = ctx.getImageData(x, y, 1, 1);
    uint8_t rgba[4] = {};
    img.data().read(rgba, 4);
    return {rgba[0], rgba[1], rgba[2], rgba[3]};
}

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLCanvasElement canvas = doc.createElement(HTMLElementTagNameMapKeys::canvas);
    canvas.width(100);
    canvas.height(80);
    doc.body().appendChild(canvas);
    CHECK(canvas.width() == 100 && canvas.height() == 80);

    // The 2D context, and the same one again.
    std::optional<CanvasRenderingContext2D> got = canvas.getContext_2d();
    CHECK(got.has_value());
    CanvasRenderingContext2D ctx = *got;
    CHECK(ctx.canvas() == canvas);
    CHECK(*canvas.getContext_2d() == ctx);

    // A blank canvas is transparent black.
    CHECK((pixel(ctx, 5, 5) == Pixel{0, 0, 0, 0}));

    // fillStyle and fillRect.
    ctx.fillStyle(std::string("#ff0000"));
    CHECK(std::get<std::string>(ctx.fillStyle()) == "#ff0000");
    ctx.fillRect(10, 10, 20, 20);
    CHECK((pixel(ctx, 15, 15) == Pixel{255, 0, 0, 255}));
    CHECK((pixel(ctx, 5, 5) == Pixel{0, 0, 0, 0}));
    CHECK((pixel(ctx, 35, 35) == Pixel{0, 0, 0, 0}));
    ctx.fillStyle(std::string("rgb(0, 128, 255)"));
    ctx.fillRect(40, 10, 10, 10);
    CHECK((pixel(ctx, 45, 15) == Pixel{0, 128, 255, 255}));

    // clearRect puts transparent back.
    ctx.clearRect(10, 10, 5, 5);
    CHECK((pixel(ctx, 12, 12) == Pixel{0, 0, 0, 0}));
    CHECK((pixel(ctx, 20, 20) == Pixel{255, 0, 0, 255}));

    // strokeRect draws an outline: its middle is untouched.
    ctx.strokeStyle(std::string("#00ff00"));
    ctx.lineWidth(4);
    CHECK(ctx.lineWidth() == 4);
    ctx.strokeRect(60, 40, 30, 30);
    CHECK((pixel(ctx, 60, 55) == Pixel{0, 255, 0, 255}));
    CHECK((pixel(ctx, 75, 55) == Pixel{0, 0, 0, 0}));

    // getImageData: size and bytes.
    ImageData block = ctx.getImageData(10, 10, 20, 20);
    CHECK(block.width() == 20 && block.height() == 20 && block.data().size() == 20 * 20 * 4);
    std::vector<uint8_t> bytes = block.data().to_vector();
    CHECK(bytes[(10 * 20 + 10) * 4] == 255 && bytes[(10 * 20 + 10) * 4 + 3] == 255);

    // putImageData: pixels written from here.
    ImageData made = ctx.createImageData(2, 2);
    CHECK(made.width() == 2 && made.height() == 2 && made.data().size() == 16);
    std::vector<uint8_t> blue;
    for (int i = 0; i < 4; i++) blue.insert(blue.end(), {0, 0, 255, 255});
    made.data().write(blue.data(), blue.size());
    ctx.putImageData(made, 80, 5);
    CHECK((pixel(ctx, 80, 5) == Pixel{0, 0, 255, 255}));
    CHECK((pixel(ctx, 81, 6) == Pixel{0, 0, 255, 255}));
    CHECK((pixel(ctx, 82, 5) == Pixel{0, 0, 0, 0}));
    ImageData fromBytes = ImageData::create(Uint8ClampedArray<ArrayBuffer>::from(blue), 2, 2);
    CHECK(fromBytes.width() == 2 && fromBytes.height() == 2);

    // A path: lineTo, closePath, fill.
    ctx.clearRect(0, 0, 100, 80);
    ctx.fillStyle(std::string("black"));
    ctx.beginPath();
    ctx.moveTo(10, 10);
    ctx.lineTo(50, 10);
    ctx.lineTo(50, 50);
    ctx.closePath();
    ctx.fill();
    CHECK((pixel(ctx, 40, 20) == Pixel{0, 0, 0, 255}));
    CHECK((pixel(ctx, 15, 40) == Pixel{0, 0, 0, 0}));
    CHECK(ctx.isPointInPath(40, 20) && !ctx.isPointInPath(15, 40));

    // An arc: a filled circle.
    ctx.clearRect(0, 0, 100, 80);
    ctx.fillStyle(std::string("#0000ff"));
    ctx.beginPath();
    ctx.arc(50, 40, 20, 0, 6.283185307179586);
    ctx.fill();
    CHECK((pixel(ctx, 50, 40) == Pixel{0, 0, 255, 255}));
    CHECK((pixel(ctx, 50, 25) == Pixel{0, 0, 255, 255}));
    CHECK((pixel(ctx, 50, 10) == Pixel{0, 0, 0, 0}));
    CHECK((pixel(ctx, 32, 22) == Pixel{0, 0, 0, 0}));

    // A stroked line, and its dash.
    ctx.clearRect(0, 0, 100, 80);
    ctx.strokeStyle(std::string("#ff00ff"));
    ctx.lineWidth(6);
    ctx.lineCap(CanvasLineCap::butt);
    ctx.beginPath();
    ctx.moveTo(10, 40);
    ctx.lineTo(90, 40);
    ctx.stroke();
    CHECK((pixel(ctx, 50, 40) == Pixel{255, 0, 255, 255}));
    CHECK(ctx.isPointInStroke(50, 41) && !ctx.isPointInStroke(50, 60));
    ctx.setLineDash(ts::Array<double>{4, 2});
    CHECK(ctx.getLineDash().size() == 2 && ctx.getLineDash()[0] == 4);
    // FINDING 10: `ts::Array<double>{}` is the default constructor (a null handle),
    // not an empty array; setLineDash then threw "cannot be converted to a
    // sequence". Array::create() makes an empty one.
    ctx.setLineDash(ts::Array<double>::create());

    // save and restore bring the state back.
    ctx.clearRect(0, 0, 100, 80);
    ctx.fillStyle(std::string("#112233"));
    ctx.globalAlpha(0.5);
    ctx.save();
    ctx.fillStyle(std::string("#ffffff"));
    ctx.globalAlpha(1);
    CHECK(std::get<std::string>(ctx.fillStyle()) == "#ffffff" && ctx.globalAlpha() == 1);
    ctx.restore();
    CHECK(std::get<std::string>(ctx.fillStyle()) == "#112233" && ctx.globalAlpha() == 0.5);
    ctx.globalAlpha(1);

    // Transforms: translate and scale move what is drawn.
    ctx.fillStyle(std::string("#ff0000"));
    ctx.save();
    ctx.translate(50, 40);
    ctx.scale(2, 2);
    ctx.fillRect(0, 0, 10, 10);
    DOMMatrix m = ctx.getTransform();
    CHECK(m.a() == 2 && m.d() == 2 && m.e() == 50 && m.f() == 40);
    ctx.restore();
    CHECK((pixel(ctx, 55, 45) == Pixel{255, 0, 0, 255}));
    CHECK((pixel(ctx, 72, 58) == Pixel{0, 0, 0, 0}));
    CHECK((pixel(ctx, 69, 59) == Pixel{255, 0, 0, 255}));
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    CHECK(ctx.getTransform().a() == 1 && ctx.getTransform().e() == 0);

    // Text measured: wider with more letters, and with a larger font.
    ctx.font("20px sans-serif");
    CHECK(ctx.font() == "20px sans-serif");
    double w1 = ctx.measureText("i").width();
    double w5 = ctx.measureText("iiiii").width();
    CHECK(w1 > 0 && w5 > w1);
    TextMetrics tm = ctx.measureText("Hello");
    CHECK(tm.width() > 0 && tm.actualBoundingBoxAscent() > 0);
    ctx.font("40px sans-serif");
    CHECK(ctx.measureText("iiiii").width() > w5);

    // Text drawn leaves pixels.
    ctx.clearRect(0, 0, 100, 80);
    ctx.fillStyle(std::string("black"));
    ctx.font("40px sans-serif");
    ctx.fillText("M", 10, 60);
    ImageData text = ctx.getImageData(0, 0, 100, 80);
    std::vector<uint8_t> tb = text.data().to_vector();
    bool inked = false;
    for (size_t i = 3; i < tb.size(); i += 4) inked = inked || tb[i] != 0;
    CHECK(inked);

    // toDataURL: a PNG by default, a JPEG on request.
    CHECK(canvas.toDataURL().starts_with("data:image/png;base64,"));
    CHECK(canvas.toDataURL("image/jpeg", 0.8).starts_with("data:image/jpeg;base64,"));

    // Resizing a canvas clears it.
    canvas.width(50);
    CHECK(canvas.width() == 50);
    CHECK((pixel(ctx, 20, 20) == Pixel{0, 0, 0, 0}));
}
