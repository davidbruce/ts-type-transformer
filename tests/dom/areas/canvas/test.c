// A canvas's 2D context from C: rectangles, colours, pixels read and written,
// paths, text metrics, state and transforms. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static CanvasFillStrokeStyles_fillStyle color(const char* css) {
    return CanvasFillStrokeStyles_fillStyle_of_string(JsString_from(css));
}

typedef struct { uint8_t r, g, b, a; } Pixel;

static Pixel pixel(CanvasRenderingContext2D ctx, int x, int y) {
    ImageData one = CanvasImageData_getImageData(ctx, x, y, 1, 1);
    Pixel p = { 9, 9, 9, 9 };
    Uint8ClampedArray_readBytes(ImageData_get_data(one), 0, &p, 4);
    return p;
}
static bool is(Pixel p, int r, int g, int b, int a) { return p.r == r && p.g == g && p.b == b && p.a == a; }

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLCanvasElement canvas = Document_createElement_canvas(doc);
    HTMLCanvasElement_set_width(canvas, 60);
    HTMLCanvasElement_set_height(canvas, 40);
    Node_appendChild(Document_get_body(doc), canvas);
    CHECK(HTMLCanvasElement_get_width(canvas) == 60 && HTMLCanvasElement_get_height(canvas) == 40);
    CanvasRenderingContext2D ctx = HTMLCanvasElement_getContext_2d(canvas);
    CHECK(ctx.h != 0 && js_same(CanvasRenderingContext2D_get_canvas(ctx), canvas));
    CHECK(!CanvasState_isContextLost(ctx));

    // A fresh canvas is transparent black.
    CHECK(is(pixel(ctx, 5, 5), 0, 0, 0, 0));

    // fillRect with fillStyle: colours are read back normalised.
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("red"));
    CanvasFillStrokeStyles_fillStyle style = CanvasFillStrokeStyles_get_fillStyle(ctx);
    CHECK(style.tag == CanvasFillStrokeStyles_fillStyle_string && STR(style.as.string, "#ff0000"));
    CanvasRect_fillRect(ctx, 0, 0, 10, 10);
    CHECK(is(pixel(ctx, 5, 5), 255, 0, 0, 255));
    CHECK(is(pixel(ctx, 9, 9), 255, 0, 0, 255));
    CHECK(is(pixel(ctx, 10, 10), 0, 0, 0, 0));
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("rgba(0, 0, 255, 0.5)"));
    CHECK(STR(CanvasFillStrokeStyles_get_fillStyle(ctx).as.string, "rgba(0, 0, 255, 0.5)"));
    CanvasRect_fillRect(ctx, 20, 0, 10, 10);
    Pixel half = pixel(ctx, 25, 5);
    CHECK(half.b == 255 && half.r == 0 && half.a >= 127 && half.a <= 129);

    // clearRect, and strokeRect with a line width.
    CanvasRect_clearRect(ctx, 0, 0, 5, 5);
    CHECK(is(pixel(ctx, 2, 2), 0, 0, 0, 0) && is(pixel(ctx, 7, 7), 255, 0, 0, 255));
    CanvasPathDrawingStyles_set_lineWidth(ctx, 4);
    CHECK(CanvasPathDrawingStyles_get_lineWidth(ctx) == 4);
    CanvasFillStrokeStyles_set_strokeStyle(ctx, color("#00ff00"));
    CanvasRect_strokeRect(ctx, 32, 12, 16, 16);
    CHECK(is(pixel(ctx, 32, 20), 0, 255, 0, 255));
    CHECK(is(pixel(ctx, 40, 20), 0, 0, 0, 0));
    CHECK(is(pixel(ctx, 20, 20), 0, 0, 0, 0));

    // putImageData: bytes made in C land on the canvas.
    ImageData made = CanvasImageData_createImageData(ctx, 2, 2);
    CHECK(ImageData_get_width(made) == 2 && ImageData_get_height(made) == 2);
    CHECK(Uint8ClampedArray_byteLength(ImageData_get_data(made)) == 16);
    uint8_t bytes[16] = { 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255 };
    Uint8ClampedArray_writeBytes(ImageData_get_data(made), 0, bytes, 16);
    CanvasImageData_putImageData(ctx, made, 50, 30);
    CHECK(is(pixel(ctx, 50, 30), 10, 20, 30, 255));
    CHECK(is(pixel(ctx, 51, 30), 40, 50, 60, 255));
    CHECK(is(pixel(ctx, 51, 31), 100, 110, 120, 255));
    ImageData back = CanvasImageData_getImageData(ctx, 50, 30, 2, 2);
    uint8_t got[16];
    Uint8ClampedArray_readBytes(ImageData_get_data(back), 0, got, 16);
    int same = 1;
    for (int i = 0; i < 16; i++) if (got[i] != bytes[i]) same = 0;
    CHECK(same);
    // ImageData built from bytes directly, with values clamped.
    uint8_t loud[4] = { 200, 100, 50, 255 };
    ImageData from_bytes = ImageData_new_with_data(Uint8ClampedArray_from(loud, 4), 1, js_some_double(1));
    CanvasImageData_putImageData(ctx, from_bytes, 59, 39);
    CHECK(is(pixel(ctx, 59, 39), 200, 100, 50, 255));

    // Paths: a triangle and a circle.
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#000000"));
    CanvasDrawPath_beginPath(ctx);
    CanvasPath_moveTo(ctx, 0, 20);
    CanvasPath_lineTo(ctx, 20, 20);
    CanvasPath_lineTo(ctx, 0, 40);
    CanvasPath_closePath(ctx);
    CHECK(CanvasDrawPath_isPointInPath(ctx, 3, 25));
    CHECK(!CanvasDrawPath_isPointInPath(ctx, 18, 38));
    CanvasDrawPath_fill(ctx);
    CHECK(is(pixel(ctx, 3, 25), 0, 0, 0, 255) && is(pixel(ctx, 18, 38), 0, 0, 0, 0));
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#0000ff"));
    CanvasDrawPath_beginPath(ctx);
    CanvasPath_arc(ctx, 40, 32, 6, 0, 6.283185307179586);
    CanvasDrawPath_fill(ctx);
    CHECK(is(pixel(ctx, 40, 32), 0, 0, 255, 255));
    CHECK(is(pixel(ctx, 40, 27), 0, 0, 255, 255));
    CHECK(is(pixel(ctx, 34, 37), 0, 0, 0, 0));
    CHECK(CanvasDrawPath_isPointInPath(ctx, 43, 32) && !CanvasDrawPath_isPointInPath(ctx, 47, 38));
    CanvasDrawPath_beginPath(ctx);
    CanvasPath_moveTo(ctx, 0, 0);
    CanvasPath_lineTo(ctx, 30, 0);
    CHECK(CanvasDrawPath_isPointInStroke(ctx, 15, 0));
    // A Path2D object is drawn with the context's fill.
    // FINDING 4: Path2D_new() with no arguments is a -Wvariadic-macro-arguments-omitted error under -std=c11 -pedantic -Werror; pass the null path.
    Path2D square = Path2D_new((Path2D){ 0 });
    CanvasPath_rect(square, 12, 12, 4, 4);
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#ffff00"));
    CanvasDrawPath_fill_with_path(ctx, square, CanvasFillRule_nonzero);
    CHECK(is(pixel(ctx, 13, 13), 255, 255, 0, 255));
    CHECK(CanvasDrawPath_isPointInPath_with_path(ctx, square, 13, 13, CanvasFillRule_absent));

    // A gradient varies along its axis.
    CanvasGradient gradient = CanvasFillStrokeStyles_createLinearGradient(ctx, 0, 0, 60, 0);
    CanvasGradient_addColorStop(gradient, 0, "#000000");
    CanvasGradient_addColorStop(gradient, 1, "#ffffff");
    CanvasFillStrokeStyles_set_fillStyle(ctx, CanvasFillStrokeStyles_fillStyle_of_CanvasGradient(gradient));
    CHECK(CanvasFillStrokeStyles_get_fillStyle(ctx).tag == CanvasFillStrokeStyles_fillStyle_CanvasGradient);
    CanvasRect_fillRect(ctx, 0, 0, 60, 3);
    Pixel left = pixel(ctx, 6, 1), right = pixel(ctx, 54, 1);
    CHECK(left.r < 40 && right.r > 215 && left.a == 255 && right.a == 255);

    // save and restore keep the drawing state apart.
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#123456"));
    CanvasState_save(ctx);
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#abcdef"));
    CanvasPathDrawingStyles_set_lineWidth(ctx, 9);
    CanvasCompositing_set_globalAlpha(ctx, 0.25);
    CHECK(STR(CanvasFillStrokeStyles_get_fillStyle(ctx).as.string, "#abcdef"));
    CanvasState_restore(ctx);
    CHECK(STR(CanvasFillStrokeStyles_get_fillStyle(ctx).as.string, "#123456"));
    CHECK(CanvasPathDrawingStyles_get_lineWidth(ctx) == 4 && CanvasCompositing_get_globalAlpha(ctx) == 1);

    // Transforms move the origin; getTransform reports them.
    CanvasRect_clearRect(ctx, 0, 0, 60, 40);
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#ff0000"));
    CanvasState_save(ctx);
    CanvasTransform_translate(ctx, 10, 20);
    CanvasRect_fillRect(ctx, 0, 0, 5, 5);
    DOMMatrix m = CanvasTransform_getTransform(ctx);
    CHECK(DOMMatrixReadOnly_get_e(m) == 10 && DOMMatrixReadOnly_get_f(m) == 20 && DOMMatrixReadOnly_get_a(m) == 1);
    CanvasTransform_scale(ctx, 2, 2);
    CanvasRect_fillRect(ctx, 10, 0, 5, 5);
    CHECK(DOMMatrixReadOnly_get_a(CanvasTransform_getTransform(ctx)) == 2);
    CanvasState_restore(ctx);
    CHECK(is(pixel(ctx, 12, 22), 255, 0, 0, 255) && is(pixel(ctx, 2, 2), 0, 0, 0, 0));
    CHECK(is(pixel(ctx, 35, 22), 255, 0, 0, 255) && is(pixel(ctx, 25, 22), 0, 0, 0, 0));
    CHECK(DOMMatrixReadOnly_get_e(CanvasTransform_getTransform(ctx)) == 0);
    CanvasTransform_setTransform(ctx, 1, 0, 0, 1, 40, 5);
    CanvasRect_fillRect(ctx, 0, 0, 2, 2);
    CHECK(is(pixel(ctx, 41, 6), 255, 0, 0, 255));
    CanvasTransform_resetTransform(ctx);
    CHECK(DOMMatrixReadOnly_get_e(CanvasTransform_getTransform(ctx)) == 0);

    // Text: measureText depends on the font; a monospace font gives equal widths.
    CanvasTextDrawingStyles_set_font(ctx, "20px monospace");
    CHECK(STR(CanvasTextDrawingStyles_get_font(ctx), "20px monospace"));
    TextMetrics narrow = CanvasText_measureText(ctx, "iiii");
    TextMetrics wide = CanvasText_measureText(ctx, "WWWW");
    CHECK(TextMetrics_get_width(narrow) > 0 && TextMetrics_get_width(narrow) == TextMetrics_get_width(wide));
    CHECK(TextMetrics_get_width(CanvasText_measureText(ctx, "WWWWWWWW")) > TextMetrics_get_width(wide) * 1.9);
    CHECK(TextMetrics_get_width(CanvasText_measureText(ctx, "")) == 0);
    CHECK(TextMetrics_get_actualBoundingBoxAscent(wide) > 0);
    CanvasRect_clearRect(ctx, 0, 0, 60, 40);
    CanvasFillStrokeStyles_set_fillStyle(ctx, color("#000000"));
    CanvasText_fillText(ctx, "WWWW", 0, 30);
    int inked = 0;
    ImageData text_area = CanvasImageData_getImageData(ctx, 0, 0, 60, 40);
    uint8_t row[60 * 40 * 4];
    Uint8ClampedArray_readBytes(ImageData_get_data(text_area), 0, row, sizeof row);
    for (int i = 3; i < (int)sizeof row; i += 4) if (row[i]) inked++;
    CHECK(inked > 100);

    // Line dashes.
    JsArray_double dash = JsArray_double_new();
    JsArray_double_push(dash, 4);
    JsArray_double_push(dash, 2);
    CanvasPathDrawingStyles_setLineDash(ctx, dash);
    CHECK(JsArray_double_length(CanvasPathDrawingStyles_getLineDash(ctx)) == 2);

    // The canvas encodes itself as a data URL.
    char url[32];
    JsString png = HTMLCanvasElement_toDataURL(canvas);
    JsString_copy(png, url, sizeof url);
    const char* prefix = "data:image/png;base64,";
    int ok = 1;
    for (int i = 0; prefix[i]; i++) if (url[i] != prefix[i]) ok = 0;
    CHECK(ok && JsString_len(png) > 40);
    JsString jpeg = HTMLCanvasElement_toDataURL(canvas, "image/jpeg");
    JsString_copy(jpeg, url, sizeof url);
    CHECK(url[11] == 'j' && url[12] == 'p' && url[13] == 'e' && url[14] == 'g');

    // Resizing the canvas clears it.
    HTMLCanvasElement_set_width(canvas, 30);
    CHECK(HTMLCanvasElement_get_width(canvas) == 30);
    CHECK(is(pixel(ctx, 5, 28), 0, 0, 0, 0));
}
