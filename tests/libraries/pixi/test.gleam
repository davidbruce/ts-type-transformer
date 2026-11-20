// pixi.js v8 from Gleam, in a page: points, rectangles and matrices, a scene
// graph of containers, graphics and sprites, and their bounds. Every check
// must hold.
//
// FINDING 12: Texture.WHITE is Texture(BufferImageSource(Nil), Nil), which no
// function taking a Texture(TextureSource(Dynamic, Nil), Nil) accepts; a sprite is
// made with Sprite.from through a Dynamic instead.
import bindings/areas/main/application_init_options
import bindings/areas/pixi_js/application
import bindings/areas/pixi_js/bounds
import bindings/areas/pixi_js/container
import bindings/areas/pixi_js/graphics
import bindings/areas/pixi_js/matrix
import bindings/areas/pixi_js/observable_point
import bindings/areas/pixi_js/point
import bindings/areas/pixi_js/point_data
import bindings/areas/pixi_js/rectangle
import bindings/areas/pixi_js/sprite
import bindings/areas/pixi_js/texture
import bindings/runtime
import bindings/types as t
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float) -> Bool {
  float.absolute_value(a -. b) <. 0.00001
}

fn rect(x: Float, y: Float, w: Float, h: Float) {
  let n = fn(v) { Some(t.NumberOrStringNumber(v)) }
  rectangle.new(n(x), n(y), n(w), n(h))
}

pub fn main() {
  let half_pi = 3.141592653589793 /. 2.0

  // ---- Point ----
  // (add, dot, magnitude, rotate... are pixi.js/math-extras, not loaded: not tested.)
  let p = point.new(Some(3.0), Some(4.0))
  let q = point.new(Some(1.0), Some(2.0))
  let _ = point.set(q, Some(7.0), Some(8.0))
  let c = point.new(None, None)
  let _ = point.copy_from(c, p)
  let passed =
    check(point.get_x(p) == 3.0 && point.get_y(p) == 4.0, "components")
    + check(point.equals(p, point.clone(p)), "clone equals")
    + check(!point.equals(p, q), "not equal")
    + check(point.get_x(q) == 7.0 && point.get_y(q) == 8.0, "set")
    + check(point.get_x(c) == 3.0 && point.get_y(c) == 4.0, "copy_from")
    + check(point.get_x(point.new(None, None)) == 0.0, "the origin by default")
    + check(point.to_string(p) == "[pixi.js/math:Point x=3 y=4]", "to_string")

  // ---- Rectangle ----
  let r = rect(10.0, 20.0, 100.0, 50.0)
  let other = rect(60.0, 40.0, 100.0, 100.0)
  let passed =
    passed
    + check(rectangle.get_right(r) == 110.0 && rectangle.get_bottom(r) == 70.0, "right and bottom")
    + check(rectangle.contains(r, 50.0, 30.0) && !rectangle.contains(r, 5.0, 30.0), "contains")
    + check(rectangle.intersects(r, other, None), "intersects")
    + check(!rectangle.intersects(r, rect(500.0, 500.0, 1.0, 1.0), None), "disjoint")
  // (intersection and union are pixi.js/math-extras: enlarge is the core one.)
  let all = rectangle.clone(r)
  let _ = rectangle.enlarge(all, other)
  let passed =
    passed
    + check(rectangle.get_x(all) == 10.0 && rectangle.get_right(all) == 160.0 && rectangle.get_bottom(all) == 140.0, "enlarge")
    + check(rectangle.contains_rect(all, r) && rectangle.contains_rect(all, other), "enlarged contains both")
    + check(!rectangle.is_empty(r) && rectangle.is_empty(rectangle.get_empty()), "is_empty")
  let padded = rectangle.clone(r)
  let _ = rectangle.pad(padded, Some(5.0), Some(5.0))
  let _ = rectangle.set_width(r, 200.0)
  let passed =
    passed
    + check(rectangle.get_x(padded) == 5.0 && rectangle.get_width(padded) == 110.0, "pad")
    + check(rectangle.get_width(r) == 200.0, "set_width")

  // ---- Matrix ----
  let m = matrix.new(None, None, None, None, None, None)
  let _ = matrix.translate(m, 10.0, 20.0)
  let _ = matrix.scale(m, 2.0, 3.0)
  let moved = matrix.apply(m, point.new(Some(1.0), Some(1.0)), None)
  let back = matrix.apply_inverse(m, moved, None)
  let spin = matrix.rotate(matrix.new(None, None, None, None, None, None), half_pi)
  let spun = matrix.apply(spin, point.new(Some(1.0), Some(0.0)), None)
  let passed =
    passed
    + check(matrix.is_identity(matrix.new(None, None, None, None, None, None)), "a new matrix is the identity")
    + check(!matrix.is_identity(m), "not the identity")
    + check(point_data.get_x(moved) == 22.0 && point_data.get_y(moved) == 63.0, "translate then scale applies to a point")
    + check(near(point_data.get_x(back), 1.0) && near(point_data.get_y(back), 1.0), "apply_inverse")
    + check(near(point_data.get_x(spun), 0.0) && near(point_data.get_y(spun), 1.0), "rotate")
    + check(matrix.get_a(m) == 2.0 && matrix.get_d(m) == 3.0 && matrix.get_tx(m) == 20.0 && matrix.get_ty(m) == 60.0, "components")
  let inverse = matrix.clone(m)
  let _ = matrix.invert(inverse)
  let product = matrix.clone(m)
  let _ = matrix.append(product, inverse)
  let passed =
    passed
    + check(matrix.is_identity(product), "m times its inverse")
   

  // ---- Scene graph: containers, graphics, sprites ----
  let root = container.new(None)
  let box = graphics.new(None)
  let _ = graphics.rect(box, 0.0, 0.0, 100.0, 50.0)
  let _ = graphics.fill(box, Some(runtime.dynamic(16_711_680.0)))
  let bb = graphics.get_bounds(box)
  let passed =
    passed
    + check(bounds.get_min_x(bb) == 0.0 && bounds.get_max_x(bb) == 100.0, "graphics bounds in x")
    + check(bounds.get_min_y(bb) == 0.0 && bounds.get_max_y(bb) == 50.0, "graphics bounds in y")
    + check(container.get_width(box) == 100.0 && container.get_height(box) == 50.0, "graphics width and height")
    + check(graphics.contains_point(box, point.new(Some(50.0), Some(25.0))), "graphics contains a point")
    + check(!graphics.contains_point(box, point.new(Some(150.0), Some(25.0))), "and not a point outside")
  // FINDING 10 (awkward): `addChild<U extends ContainerChild[]>(...children: U): U[0]` is
  // add_child(self, children: u, codec) with u the whole rest array, so a list and
  // a list codec; and what comes back is Container(Dynamic, Nil), not the child.
  let _ = container.add_child(root, [box])
  container.set_x(box, 30.0)
  container.set_y(box, 40.0)
  container.set_x(root, 100.0)
  let root_width = container.get_width(root)
  let passed =
    passed
    + check(list.length(container.get_children(root, t.graphics_codec())) == 1, "one child")
    + check(container.get_x(box) == 30.0, "child's own position")
    + check(matrix.get_tx(container.get_local_transform(box)) == 30.0 && matrix.get_ty(container.get_local_transform(box)) == 40.0, "the local transform holds the position")
    // FINDING 11: Container.getBounds, getLocalBounds, toGlobal, toLocal, getGlobalPosition,
    // getChildAt... are on mixin types (MeasureMixin, ToLocalGlobalMixin) with no upcast
    // from a Container: unreachable. The parent's width is checked instead.
    + check(root_width == 100.0, "the parent is as wide as its child")
    + check(option.is_some(container.get_parent(box)), "the child has a parent")
    + check(option.is_none(container.get_parent(root)), "the root has none")

  // Scale of the parent.
  observable_point.set(container.get_scale(root), Some(2.0), Some(2.0)) |> fn(_) { Nil }
  let passed =
    passed
    + check(container.get_width(root) == 200.0, "scaling the parent scales its width")
    + check(container.get_height(root) == 100.0, "and its height")
    + check(container.get_width(box) == 100.0, "the child's own width is unchanged")
  container.set_alpha(box, 0.5)
  container.set_visible(box, False)
  container.set_angle(box, 90.0)
  let passed =
    passed
    + check(container.get_alpha(box) == 0.5, "alpha")
    + check(!container.get_visible(box), "visible")
    + check(near(container.get_rotation(box), half_pi), "angle sets rotation")

  // Removing the child empties the parent.
  let _ = container.remove_child(root, [box])
  let passed =
    passed
    + check(container.get_children(root, t.graphics_codec()) == [], "the child is removed")
    + check(option.is_none(container.get_parent(box)), "and has no parent")

  // Circles and polygons.
  let dot = graphics.new(None)
  let _ = graphics.circle(dot, 50.0, 50.0, 25.0)
  let _ = graphics.fill(dot, Some(runtime.dynamic(255.0)))
  let db = graphics.get_bounds(dot)
  let passed =
    passed
    + check(bounds.get_min_x(db) == 25.0 && bounds.get_max_x(db) == 75.0, "circle bounds")
    + check(graphics.contains_point(dot, point.new(Some(50.0), Some(50.0))), "circle contains its centre")
    + check(!graphics.contains_point(dot, point.new(Some(26.0), Some(26.0))), "circle does not contain its corner")

  // Sprites: Texture.WHITE is one pixel.
  let white = texture.get_white()
  let s = sprite.from(runtime.dynamic(white), None)
  let passed =
    passed
    + check(texture.get_width(white) == 1.0 && texture.get_height(white) == 1.0, "the white texture is 1 by 1")
    + check(container.get_width(s) == 1.0, "a sprite is its texture's size")
  container.set_width(s, 20.0)
  let _ = observable_point.set(sprite.get_anchor(s), Some(0.5), Some(0.5))
  let passed =
    passed
    + check(container.get_width(s) == 20.0, "sprite width set")
    + check(observable_point.get_x(sprite.get_anchor(s)) == 0.5, "anchor")

  // ---- An application with its own canvas ----
  // FINDING 20: ApplicationInitOptions.canvas takes an ICanvas, which the page's
  // HTMLCanvasElement (test_page.canvas()) is not as far as the types go: the page
  // canvas cannot be handed over. The application makes its own.
  let app = application.new()
  let opts = application_init_options.create()
  application_init_options.set_width(opts, Some(200.0))
  application_init_options.set_height(opts, Some(100.0))
  application_init_options.set_background_color(opts, Some(runtime.dynamic(255.0)))
  runtime.promise_then(application.init(app, Some(opts)), fn(ready) {
    let assert Ok(_) = ready
    let screen = application.get_screen(app)
    let stage = application.get_stage(app)
    let dot = graphics.new(None)
    let _ = graphics.circle(dot, 100.0, 50.0, 20.0)
    let _ = graphics.fill(dot, Some(runtime.dynamic(16_777_215.0)))
    let _ = container.add_child(stage, [dot])
    application.render(app)
    let passed =
      passed
      + check(rectangle.get_width(screen) == 200.0 && rectangle.get_height(screen) == 100.0, "the application's screen")
      + check(list.length(container.get_children(stage, runtime.codec_dynamic())) == 1, "the stage holds the dot")
    application.destroy(app, None, None)
    io.println(int.to_string(passed) <> " checks passed")
  })
}

