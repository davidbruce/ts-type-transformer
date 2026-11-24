// PixiJS v8 from C, in a page: points, rectangles and matrices, a scene
// graph of containers, graphics and a sprite, and their bounds. Every CHECK
// must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool contains(JsString s, const char* needle) {
    char buf[256];
    uint32_t n = JsString_copy(s, buf, sizeof buf), m = ts_strlen(needle);
    for (uint32_t i = 0; i + m <= n; i++) {
        uint32_t j = 0;
        while (j < m && buf[i + j] == needle[j]) j++;
        if (j == m) return true;
    }
    return false;
}
static bool near(double a, double b) { return (a > b ? a - b : b - a) < 1e-6; }
static JsOptionalDouble D(double x) { return js_some_double(x); }
static Rectangle Rect(double x, double y, double w, double h) {
    return Rectangle_new(x, y, w, h);
}

// An application on the page's canvas: its init is a promise.
static int pending;
static Application app;
static void on_app(void* ctx, bool ok, JsValue v) {
    CHECK(ok);
    if (ok) {
        CHECK(js_same((JsValue){ Application_get_canvas(app).h }, (JsValue){ canvas().h }));
        Rectangle screen = Application_get_screen(app);
        CHECK(Rectangle_get_width(screen) == 400 && Rectangle_get_height(screen) == 300);
        Container stage = Application_get_stage(app);
        Graphics dot = Graphics_new((GraphicsOptions){ 0 });
        Graphics_circle(dot, 50, 50, 20);
        Graphics_fill(dot, js_number(0x00ff00));
        JsArray_Container kids = JsArray_Container_new();
        JsArray_Container_push(kids, Graphics_as_Container(dot));
        Container_addChild(stage, kids);
        Application_render(app);
        CHECK(JsArray_Container_length(Container_get_children(stage)) == 1);
        Bounds b = MeasureMixin_getBounds(stage, js_none_bool(), (Bounds){ 0 });
        CHECK(Bounds_get_minX(b) == 30 && Bounds_get_maxX(b) == 70);
        Application_destroy(app, (JsValue){ 0 }, (JsValue){ 0 });
    }
    pending = 0;
}
__attribute__((export_name("busy"))) int busy(void) { return pending; }

__attribute__((export_name("run"))) void run(void) {
    // ---- Point ----
    Point p = Point_new(D(3), D(4));
    CHECK(Point_get_x(p) == 3 && Point_get_y(p) == 4);
    Point q = Point_clone(p);
    Point_set(q, D(7), D(8));
    CHECK(Point_get_x(p) == 3 && Point_get_x(q) == 7);
    CHECK(Point_equals(p, Point_clone(p)) && !Point_equals(p, q));
    Point origin = Point_new(js_none_double(), js_none_double());
    CHECK(Point_get_x(origin) == 0 && Point_get_y(origin) == 0);
    Point_copyFrom(origin, p);
    CHECK(Point_get_x(origin) == 3 && Point_get_y(origin) == 4);
    JsString text = Point_toString(p);
    CHECK(contains(text, "x=3 y=4"));

    // ---- Rectangle ----
    Rectangle r = Rect(10, 20, 100, 50);
    CHECK(Rectangle_get_x(r) == 10 && Rectangle_get_y(r) == 20);
    CHECK(Rectangle_get_width(r) == 100 && Rectangle_get_height(r) == 50);
    CHECK(Rectangle_get_left(r) == 10 && Rectangle_get_right(r) == 110);
    CHECK(Rectangle_get_top(r) == 20 && Rectangle_get_bottom(r) == 70);
    CHECK(ShapePrimitive_contains(r, 50, 40) && !ShapePrimitive_contains(r, 5, 40));
    CHECK(!Rectangle_isEmpty(r) && Rectangle_isEmpty(Rect(0, 0, 0, 0)));
    Rectangle other = Rect(100, 40, 50, 50);
    CHECK(Rectangle_intersects(r, other, (Matrix){ 0 }));
    CHECK(!Rectangle_intersects(r, Rect(200, 200, 5, 5), (Matrix){ 0 }));
    Rectangle grown = Rectangle_pad(Rectangle_clone(r), D(5), D(10));
    CHECK(Rectangle_get_x(grown) == 5 && Rectangle_get_y(grown) == 10);
    CHECK(Rectangle_get_width(grown) == 110 && Rectangle_get_height(grown) == 70);
    CHECK(Rectangle_get_x(r) == 10);
    Rectangle scaled = Rectangle_scale(Rectangle_clone(r), 2, D(3));
    CHECK(Rectangle_get_width(scaled) == 200 && Rectangle_get_height(scaled) == 150);
    CHECK(Rectangle_containsRect(grown, r));
    Rectangle_set_width(r, 60);
    CHECK(Rectangle_get_right(r) == 70);

    // ---- Matrix ----
    Matrix id = Matrix_new(js_none_double());
    CHECK(Matrix_isIdentity(id));
    Matrix m = Matrix_new(js_none_double());
    Matrix_scale(m, 2, 3);
    Matrix_translate(m, 10, 20);
    CHECK(Matrix_get_a(m) == 2 && Matrix_get_d(m) == 3 && Matrix_get_tx(m) == 10 && Matrix_get_ty(m) == 20);
    Point at = Point_new(D(1), D(1));
    PointData moved = Matrix_apply(m, at, (PointData){ 0 });
    CHECK(PointData_get_x(moved) == 12 && PointData_get_y(moved) == 23);
    PointData unmoved = Matrix_applyInverse(m, moved, (PointData){ 0 });
    CHECK(near(PointData_get_x(unmoved), 1) && near(PointData_get_y(unmoved), 1));
    Matrix inv = Matrix_invert(Matrix_clone(m));
    CHECK(near(Matrix_get_a(inv), 0.5) && !Matrix_isIdentity(inv));
    Matrix both = Matrix_append(Matrix_clone(m), inv);
    CHECK(Matrix_isIdentity(both));
    Matrix rot = Matrix_rotate(Matrix_new(js_none_double()), 3.14159265358979323846 / 2);
    PointData turned = Matrix_apply(rot, Point_new(D(1), D(0)), (PointData){ 0 });
    CHECK(near(PointData_get_x(turned), 0) && near(PointData_get_y(turned), 1));
    CHECK(Matrix_equals(Matrix_clone(m), m));

    // ---- Containers ----
    Container root = Container_new((ContainerOptions){ 0 });
    Container_set_x(root, 10);
    Container_set_y(root, 20);
    CHECK(Container_get_x(root) == 10 && Container_get_y(root) == 20);
    CHECK(ObservablePoint_get__x(Container_get_position(root)) == 10);
    CHECK(Container_get_alpha(root) == 1 && Container_get_visible(root));
    Container_set_alpha(root, 0.5);
    CHECK(Container_get_alpha(root) == 0.5);

    // Graphics: a filled rectangle, with bounds.
    Graphics box = Graphics_new((GraphicsOptions){ 0 });
    Graphics_rect(box, 0, 0, 100, 50);
    Graphics_fill(box, js_number(0xff0000));
    Bounds local = Graphics_get_bounds(box);
    CHECK(Bounds_get_minX(local) == 0 && Bounds_get_maxX(local) == 100);
    CHECK(Bounds_get_minY(local) == 0 && Bounds_get_maxY(local) == 50);
    CHECK(Bounds_get_width(local) == 100 && Bounds_get_height(local) == 50);
    Container_set_x(box, 5);
    Container_set_y(box, 5);

    // A Graphics is a Container: it goes where a container is wanted.
    JsArray_Container kids = JsArray_Container_new();
    JsArray_Container_push(kids, Graphics_as_Container(box));
    Container_addChild(root, kids);
    JsArray_Container children = Container_get_children(root);
    CHECK(JsArray_Container_length(children) == 1);
    CHECK(js_same(Container_get_parent(box), root));
    CHECK(js_same((JsValue){ JsArray_Container_get(children, 0).h }, (JsValue){ box.h }));
    CHECK(js_same((JsValue){ ChildrenHelperMixin_getChildAt(root, 0).h }, (JsValue){ box.h }));

    // The bounds in the parent's space: (5, 5) to (105, 55), with the root at (10, 20).
    Bounds world = MeasureMixin_getBounds(root, js_none_bool(), (Bounds){ 0 });
    CHECK(Bounds_get_minX(world) == 15 && Bounds_get_minY(world) == 25);
    CHECK(Bounds_get_maxX(world) == 115 && Bounds_get_maxY(world) == 75);
    Bounds inside = MeasureMixin_getLocalBounds(root);
    CHECK(Bounds_get_minX(inside) == 5 && Bounds_get_maxX(inside) == 105);
    CHECK(Container_get_width(root) == 100 && Container_get_height(root) == 50);

    // Scaling the root scales what it measures.
    PointData two = PointData_create();
    PointData_set_x(two, 2);
    PointData_set_y(two, 2);
    Container_set_scale(root, two);
    CHECK(Container_get_width(root) == 200 && Container_get_height(root) == 100);
    Bounds scaled_world = MeasureMixin_getBounds(root, js_none_bool(), (Bounds){ 0 });
    CHECK(Bounds_get_minX(scaled_world) == 20 && Bounds_get_maxX(scaled_world) == 220);
    PointData_set_x(two, 1);
    PointData_set_y(two, 1);
    Container_set_scale(root, two);

    // A sprite of the white texture, sized.
    Texture white = Texture_get_WHITE();
    CHECK(Texture_get_width(white) > 0 && Texture_get_height(white) > 0);
    Sprite sprite = Sprite_new(white);
    Sprite_set_width(sprite, 30);
    Container_set_height(sprite, 20);
    CHECK(Container_get_width(sprite) == 30 && Container_get_height(sprite) == 20);
    CHECK(js_same((JsValue){ Sprite_get_texture(sprite).h }, (JsValue){ white.h }));
    Container_set_x(sprite, 200);
    JsArray_Container more = JsArray_Container_new();
    JsArray_Container_push(more, Sprite_as_Container(sprite));
    Container_addChild(root, more);
    CHECK(JsArray_Container_length(Container_get_children(root)) == 2);
    Bounds all = MeasureMixin_getBounds(root, js_none_bool(), (Bounds){ 0 });
    CHECK(Bounds_get_minX(all) == 15 && Bounds_get_maxX(all) == 240);

    // Labels and finding children.
    FindMixin_set_label(sprite, "the-sprite");
    CHECK(js_same((JsValue){ FindMixin_getChildByLabel(root, "the-sprite", js_none_bool()).h }, (JsValue){ sprite.h }));
    CHECK(FindMixin_getChildByLabel(root, "nothing", js_none_bool()).h == 0);

    // Removing a child.
    JsArray_Container gone = JsArray_Container_new();
    JsArray_Container_push(gone, Sprite_as_Container(sprite));
    Container_removeChild(root, gone);
    CHECK(JsArray_Container_length(Container_get_children(root)) == 1);
    CHECK(Container_get_parent(sprite).h == 0);
    Container_destroy(root, (JsValue){ 0 });
    CHECK(Container_get_destroyed(root));

    // ---- An application on the page's canvas ----
    app = Application_new();
    Application_init_options init_options = Application_init_options_create();
    Application_init_options_set_canvas(init_options, (ICanvas){ canvas().h });
    Application_init_options_set_width(init_options, D(400));
    Application_init_options_set_height(init_options, D(300));
    pending = 1;
    Promise started = Application_init(app, init_options);
    Promise_then(started, on_app, 0);
}
