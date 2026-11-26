// pixi.js v8 from C++, in a page: points, rectangles and matrices, a scene
// graph of containers, graphics and a sprite, with their bounds.
// Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 1e-6) { return std::fabs(a - b) < eps; }

using Cont = Container;

// An Application on the page's canvas; init is asynchronous.
static bool working = true;
static ts::Task app_on_canvas() {
    Application app = Application::create();
    Application_init_options options = Application_init_options::create();
    // FINDING 17: ICanvas is an interface that HTMLCanvasElement satisfies, with
    // no conversion in the bindings (same as 11): rewrapped.
    ICanvas canvas;
    static_cast<ts::Handle&>(canvas) = test_page::canvas();
    options.canvas(canvas);
    options.width(400);
    options.height(300);
    options.backgroundColor(ColorSource(double(0x1099bb)));
    options.autoStart(false);
    ts::Result<void> r = co_await app.init(options);
    CHECK(bool(r));
    if (r) {
        CHECK(app.screen().width() == 400 && app.screen().height() == 300);
        CHECK(app.canvas() == test_page::canvas());
        Graphics box = Graphics::create();
        box.rect(10, 10, 100, 100);
        box.fill(ColorSource(double(0xff0000)));
        app.stage().addChild(ts::Array<Container>{box});
        CHECK(app.stage().children().size() == 1);
        app.render();
        CHECK(near(box.getBounds().maxX(), 110));
        app.destroy();
    }
    working = false;
}

extern "C" __attribute__((export_name("busy"))) int busy() { return working; }

extern "C" __attribute__((export_name("run"))) void run() {
    app_on_canvas();
    // ---- math ----
    Point p = pixi_js::Point::create(3, 4);
    CHECK(p.x() == 3 && p.y() == 4);
    p.set(5, 6);
    CHECK(p.x() == 5 && p.y() == 6);
    Point q = p.clone();
    q.x(100);
    CHECK(p.x() == 5 && q.x() == 100);
    CHECK(p.equals(pixi_js::Point::create(5, 6)));
    CHECK(Point::create(2, 3).toString().find("2") != std::string::npos);

    Rectangle rect = Rectangle::create(StringNumber(10.0), StringNumber(20.0), StringNumber(30.0), StringNumber(40.0));
    CHECK(rect.x() == 10 && rect.y() == 20 && rect.width() == 30 && rect.height() == 40);
    CHECK(rect.left() == 10 && rect.right() == 40 && rect.top() == 20 && rect.bottom() == 60);
    CHECK(rect.contains(25, 30));
    CHECK(!rect.contains(5, 30));
    CHECK(!rect.isEmpty());
    CHECK(rect.intersects(Rectangle::create(StringNumber(35.0), StringNumber(30.0), StringNumber(30.0), StringNumber(30.0))));
    CHECK(!rect.intersects(Rectangle::create(StringNumber(100.0), StringNumber(100.0), StringNumber(5.0), StringNumber(5.0))));
    Rectangle padded = rect.clone().pad(5);
    CHECK(padded.x() == 5 && padded.width() == 40 && padded.height() == 50);
    CHECK(rect.width() == 30);

    // A matrix: translate, scale, rotate, apply to a point, invert.
    Matrix m = Matrix::create();
    CHECK(m.isIdentity());
    m.translate(10, 20).scale(2, 3);
    CHECK(m.tx() == 20 && m.ty() == 60 && m.a() == 2 && m.d() == 3);
    Point moved = m.apply(PointData(p));
    CHECK(moved.x() == 2 * 5 + 20 && moved.y() == 3 * 6 + 60);
    Point restored = m.clone().invert().apply(PointData(moved));
    CHECK(near(restored.x(), 5) && near(restored.y(), 6));
    Matrix turn = Matrix::create().rotate(M_PI / 2);
    Point turned = turn.apply(PointData(pixi_js::Point::create(1, 0)));
    CHECK(near(turned.x(), 0) && near(turned.y(), 1));
    CHECK(!m.isIdentity());
    CHECK(m.clone().identity().isIdentity());

    // ---- scene graph ----
    Cont root = Cont::create();
    Cont group = Cont::create();
    group.x(100);
    group.y(50);
    root.addChild(ts::Array<Container>{group});
    CHECK(root.children().size() == 1);
    CHECK(group.parent().has_value() && *group.parent() == root);
    CHECK(group.x() == 100 && group.y() == 50);
    CHECK(group.position().x() == 100);
    group.position().set(120, 60);
    CHECK(group.x() == 120 && group.y() == 60);
    group.scale().set(2, 2);
    CHECK(group.scale().x() == 2);
    group.rotation(M_PI / 2);
    CHECK(near(group.angle(), 90, 1e-6));
    group.rotation(0);
    group.pivot().set(0, 0);
    group.alpha(0.5);
    CHECK(group.alpha() == 0.5);
    group.visible(false);
    CHECK(!group.visible());
    group.visible(true);
    CHECK(!group.destroyed());

    // Graphics: a filled rectangle, in the group at (120, 60), scaled by 2.
    Graphics box = Graphics::create();
    box.rect(0, 0, 100, 50);
    box.fill(ColorSource(double(0xff0000)));
    CHECK(box.bounds().minX() == 0 && box.bounds().maxX() == 100);
    CHECK(box.bounds().maxY() == 50);
    group.addChild(ts::Array<Container>{box});
    CHECK(group.children().size() == 1);
    CHECK(group.getChildIndex(box) == 0);

    // Local bounds are the shape's own; global bounds include the transforms.
    Bounds local = box.getLocalBounds();
    CHECK(local.minX() == 0 && local.maxX() == 100 && local.minY() == 0 && local.maxY() == 50);
    Bounds global = box.getBounds();
    CHECK(near(global.minX(), 120) && near(global.maxX(), 320));
    CHECK(near(global.minY(), 60) && near(global.maxY(), 160));
    Rectangle rg = global.rectangle();
    CHECK(near(rg.width(), 200) && near(rg.height(), 100));
    // The group's own bounds hold its child.
    Bounds gb = group.getBounds();
    CHECK(near(gb.maxX(), 320));

    // Where the box's local point lands in the world, and back.
    PointData world = box.toGlobal(PointData(pixi_js::Point::create(10, 10)));
    CHECK(near(world.x(), 140) && near(world.y(), 80));
    PointData local_again = box.toLocal(PointData(world));
    CHECK(near(local_again.x(), 10) && near(local_again.y(), 10));
    Point origin = box.getGlobalPosition();
    CHECK(near(origin.x(), 120) && near(origin.y(), 60));

    // More shapes: a circle and a polygon add to the bounds.
    Graphics dot = Graphics::create();
    dot.circle(0, 0, 10);
    dot.fill(ColorSource(std::string("#00ff00")));
    CHECK(near(dot.bounds().minX(), -10) && near(dot.bounds().maxX(), 10));
    dot.x(500);
    root.addChild(ts::Array<Container>{dot});
    CHECK(root.children().size() == 2);
    CHECK(near(root.getBounds().maxX(), 510));
    CHECK(near(root.getBounds().minX(), 120));

    // A sprite on the white texture: its size, anchor and place.
    Texture white = Texture::WHITE();
    Sprite sprite = Sprite::from(Sprite_from_source(white));
    CHECK(sprite.width() == Texture::WHITE().width());
    sprite.width(40);
    sprite.height(30);
    CHECK(sprite.width() == 40 && sprite.height() == 30);
    sprite.anchor().set(0.5, 0.5);
    CHECK(sprite.anchor().x() == 0.5);
    sprite.position().set(200, 200);
    root.addChild(ts::Array<Container>{sprite});
    CHECK(root.children().size() == 3);
    Bounds sb = sprite.getBounds();
    CHECK(near(sb.minX(), 180) && near(sb.maxX(), 220));
    CHECK(near(sb.minY(), 185) && near(sb.maxY(), 215));

    // Reordering and removing children.
    root.setChildIndex(sprite, 0);
    CHECK(root.getChildIndex(sprite) == 0);
    root.removeChild(ts::Array<Container>{dot});
    CHECK(root.children().size() == 2);
    CHECK(!dot.parent().has_value());
    group.removeFromParent();
    CHECK(root.children().size() == 1);

    // FINDING 16: Container extends EventEmitter<ContainerEvents, any> (from
    // eventemitter3, not bound): the generated `EventEmitter` (ts_types.hpp:2328)
    // is an empty struct, so on/off/emit exist on no display object.

    // Destroying a display object.
    dot.destroy();
    CHECK(dot.destroyed());
}
