# Ergonomic targets

Draft, for agreement before the generator changes. For eight demos it shows how
each one reads in C, C++, Rust, Gleam and F# today, and how it should read: the
code a native library author in that language would have written for the same
JavaScript library. The generator will be judged against these targets, not
against what compiles. Zig is left out: it already reads like the TypeScript.

## The principle

Design for the pattern the JavaScript library uses, in the idiom of the target
language, not for each TypeScript construct. Zig shows the goal for option
objects: `.title = .{ .text = "Monthly Output", .left = 16 }` reads like the
TypeScript object literal, because a Zig literal takes its type from where it goes.

## Conventions

1. **Option objects, nested ones too.** C `Pkg_Type_create(.field = ..)`; C++
   `Type({.a = ..})`, nested option fields in double braces with no type name
   (`.lineStyle = {{.color = "#111"}}`), designators in ASCII order of the field
   name, with chainable setters kept for fields set conditionally (`ts::Field<T>`
   takes `T::Fields`, so `{{..}}` needs no type name); Rust
   `Type::default().with_a(..)`; Gleam `type_module.create() |> type_module.with_a(..)`;
   F# `Type.Create(a = ..)`.

2. **A value of one of several kinds, going in.** The user never writes a
   generated union name.
   - C++, Rust and F# convert implicitly. C++ unions need a non-template
     constructor per member, plus one from `Inner::Fields` for an option-object
     member, `std::initializer_list<T>` for `T[]`, pairs for a record, and templates
     for any arithmetic type (so `.min = 0` is not ambiguous with a pointer) and any
     callable (a lambda reaching a `std::function` member); `std::variant` takes no
     braced list. Rust gives a callback member its own `name_fn` taking
     `impl FnMut`, since a closure through `impl Into` needs annotated arguments.
     F# unions are one flat type with an
     `op_Implicit` from every member and every subclass of a class member, since F#
     does not chain an upcast into `op_Implicit`; no nested `U9`, and no overload per
     member.
   - C function arguments take any member (`_Generic`).
   - Gleam functions and builders, and C option fields, give the plain name to the
     *primary* member and `name_<member>` to each other one: `with_left(16.0)` /
     `with_left_string("10%")`, `.fill = "#111"` / `.fill_gradient = g`. Separate C
     fields also avoid C11 zeroing a union's other members, which makes
     `{ .number = 0 }` and `{ .boolean = false }` the same bytes. A member suffix
     comes before `_with` (`add_days_string_with`); a callback member's suffix is
     `_fn` in every language (`.color_fn`, `with_color_fn`).
   - The primary member is given by the first rule that applies:
     1. `T | T[]`: `T`; the array is `name_list` (`.title_list`, `with_series_list`),
        also where `T` is itself a union of objects, flattened (ECharts'
        `SeriesOption | SeriesOption[]`).
     2. `string`, when every other member is an object (`string | CanvasGradient`,
        `HTMLDivElement | string`, `string | NodeType`, `string | symbol`; `symbol`
        is an opaque `JsSymbol`, not `any`).
     3. A record beside other objects (`{ [name]: NodeSpec } | OrderedMap`): the
        record, as a pair list (convention 10).
     4. When every member is an object: the one named by just the words they all
        share (Leaflet's `Icon` beside `DivIcon`, `.icon` / `.icon_div`; `LatLng` in
        `LatLngExpression`, `map.set_view(m, london)`), else a lone array of
        other things (Konva's `points`, `number[]` beside typed arrays; Leaflet's
        `PointExpression` tuple beside `Point`), else none, each named
        (`.series_bar`, `.xAxis_category`, `with_background_texture`).
     5. Otherwise the first member as declared: a library's alias keeps its
        written order (`PositionSizeOption = number | string` gives `number`,
        `OptionName = string | number` gives `string`, date-fns' `DateType | number
        | string` gives `Date`). An inline union's order is the checker's, which
        always puts `string` before `number`, so there the number comes first
        (ECharts' `fontSize`, `min`, `max`), but for a key's `string | number |
        symbol`.

     A member is named by: `string`, `number`, `boolean`, `list`, `fn`, `symbol`;
     an option object by the `type` its `create` fixes (`category`, `value`); a
     type lowering made up by the last part of its name
     (`EditorView_new_place_object`: `object`); else its type's name, less the
     words every such member shares and the package prefix (`BarSeriesOption`:
     `bar`; `CanvasGradient`: `canvas_gradient`), an acronym a word of its own
     (`GLBufferAttribute`: `gl`). A name another parameter's member
     already took is the member's whole name (node's `BufferEncoding` beside a
     path's `Buffer`).

     `null` and `undefined` are never the primary member. A nullable value takes
     null itself: C `NULL`, C++ `nullptr` (a constructor from `nullptr_t`, since
     `std::optional` takes neither `nullptr` nor a braced list), Rust `None`, F#
     `null` (the union allows the null literal, also through a property setter),
     Gleam `name_null` like any other member (`object3d.set_background_null(sc)`). An
     optional nullable argument treats null as absent.
   - In a Gleam function with several union parameters, a call leaving one
     parameter's primary member uses `name_<member>`
     (`schema.node_list_with(sch, "doc", ..)`), unless another parameter has the
     same union (`difference_in_hours(later, earlier)`: which one is the string?);
     then, and when two or more leave theirs, it is `name_of`, taking union values. Never one function per combination.
   - Gleam union modules (`series_option.bar(..)`) remain for lists, `_of` and
     values coming back.

3. **Lists mixing kinds** keep their element type: C `X_list({ .number = 42 }, ..)`
   and `js_numbers(..)`, a list of one class `X_list(a, b)` and of `any`
   `js_values(..)`; C++ `{42, 68}`; Rust `[12.0.into(), item.into()]`; Gleam
   `[data.number(42.0), data.item(i)]`; F# `ResizeArray [ !^42.; !^item ]` (F#
   applies `op_Implicit` to arguments, not list elements).

4. **A value of one of several kinds, coming back**, is matched with short
   variant names: Rust `Token::Heading(h)`, C `switch (t.tag)`, Gleam
   `token.Heading(h)`, F# `Token.Heading h`. A field every member has is read
   without a match, as any getter is: Rust `t.type_()`, C++ `t.type()`, Gleam
   `token.get_type(t)`, C `Marked_Token_get_type(t)`, F# ``t.``type`` ``. A field of string literals widens to
   `string`. Getters only.

5. **Optional arguments.**
   - C leaves out trailing ones; C++ overloads; F# optional arguments.
   - Rust: one function with every argument, an optional being `impl Optional<T>`,
     a bare value or `None` (`Vector3::new(0.0, -9.81, 0.0)`, `v.set(6.0, 10.0, 8.0)`,
     `world.create_collider(&desc, None)`). Trailing options objects, records, lists,
     callbacks and `object` unions are left out of the plain name and given with
     `name_with` (`marked(md)`, `echarts::init(&el)`, `schema.text("bold")`,
     `world.step(&q)`); class instances are not. A union counts as its primary
     member (`Material | Material[]` is a class). A constructor whose arguments are
     all optional also implements `Default`.
   - Gleam: the plain name without the optional tail; `name_with` takes the whole
     tail, bare, except that a callback or an interface of methods ending a tail of
     two or more is `Option` (`world.step_with(w, q, None)`), as is a type nothing
     in the bindings makes or a call gives back (pts' `CanvasLineJoin` without the
     DOM's bindings). More than four optionals are an option bag folded into the
     class's module: `sphere_geometry.new_with(sphere_geometry.args_create() |>
     sphere_geometry.args_with_radius(0.36))`.
   - More than four optionals (three's geometries): the plain name has none and
     `name_with` takes an arguments builder (`CylinderGeometryArgs::default()
     .with_radius_top(0.55)`; Gleam's, as above, `cylinder_geometry.args_create() |> ..`).
   - Overloads differing only in their optional tail (`setOption(opt, notMerge?,
     lazyUpdate?)`, `setOption(opt, opts?)`) merge: the shared part is the plain
     name, the one ending in an options object gives `name_with`, and each other one
     is `name_with_<its first optional>` (`set_option_with_not_merge`).

6. **Rest parameters.** C++ and F# variadic. C one argument per value through
   `_Generic`, plus `name_list` taking a `JsArray` for loops (the 40-subtype
   `_Generic` limit must hold for three's `Object3D`, Konva's `Shape` and the DOM's
   `Node`). Rust and Gleam a list (in Gleam, of one class; mixed classes take
   separate calls): after required arguments the plain name leaves the rest out and
   `name_with` takes the list (`emit("idle")`, `emit_with("metric", [..])`); a rest
   parameter alone is the plain name's list (`layer.add([&r, &c])`, borrowed in Rust).

7. **Type parameters**, by the first rule that applies:
   1. Kept: built-in containers (`Array`, `ReadonlyArray`, `Promise`, `Map`, `Set`,
      `Iterable`, `AsyncIterable`, `Record`), and a parameter without a useful
      constraint that the user fills with their own data (TanStack `Table<TData>`,
      ProseMirror `OrderedMap<T>`).
   2. A useful constraint (not `any`, `unknown`, `object`, `Record<string, unknown>`,
      or an F-bound such as `EventMap<T>`): erased to it, and mapped or conditional types are evaluated
      against it (`Shape<Config extends ShapeConfig>`, `DateType extends Date`,
      `Nodes extends string`).
   3. Otherwise the union of the library's own instantiations, without `never`,
      `any`, `unknown` and members a wider one includes (`LineStyleOption<Color | Color[]>`,
      `ItemStyleOption<CallbackDataParams>`); with none, the default.

   A subclass's `extends Base<X>` gives the inherited members `X`. The user never
   writes `<>`, a turbofish, a phantom default, or `X<obj>` with
   `unbox`.

8. **Callbacks** are closures passed directly.
   - Rust takes `impl FnMut(..) + 'static`, so closure argument types are inferred.
     When a removal method takes the same callback, `name_listener(&Listener)` takes
     a kept handle (`on` / `on_listener`).
   - A borrowing closure only for an explicit per-library list of `Type.method`s
     that call it before returning (Rapier `EventQueue.drainCollisionEvents`); never
     option fields (marked stores `walkTokens`), never wildcards.
   - A closure passed directly is freed by `FinalizationRegistry` once JavaScript
     drops it (`requestAnimationFrame`, `once`, `on` without a handle).
   - C++ stored lambdas capture by value; `[&]` only on the borrowing list.
   - A callback whose result is `void | Promise<void> | ..` returns nothing (so no
     async walker; intended).

9. **Names.** A union the library named keeps that name; otherwise a short
   concept name from where it is used. The same holds for any alias the library
   named (marked's mapped `RendererObject`); `X & {}` is `X`. Never `TypeAOrTypeBOrTypeC`, never doubled.
   Callback types too (`Rapier_DrainCollisionEventsFn`). Gleam enum members live in
   the enum's module (`active_events.CollisionEvents`). One module per class in
   Gleam, one prefix in C.

10. **String-keyed maps whose order matters** (ProseMirror `nodes`, `marks`,
    `attrs`) are ordered pair lists: C `Pm_X_record({ k, v }, ..)`, C++
    `{{"k", v}, ..}`, Rust `[("k", v), ..]`, Gleam `[#("k", v), ..]`, F#
    `[ "k", v; .. ]`.

11. **Smaller.**
    - `T | false`, with literal `false` and no null member, is an option
      (`Option<String>`, `Option(String)`, C++ `std::optional<std::string>`, C a
      null handle; F# keeps `U2<T, bool>`, Fable's option being `undefined`).
    - A callable property is a method (`Marked#parse`).
    - Overloads chosen by a literal: the plain name is the synchronous one and
      passes `{ async: false }` itself; `_async` passes `{ async: true }`; the
      catch-all goes. Not Node's `{ bigint: true }`, whose options carry more
      than the literal (`throwIfNoEntry`): those overloads stay as they are.
    - Flag enums combine with `|`, in Gleam with a list. Nothing in a `.d.ts` marks
      them (a power-of-two test also catches plain `0, 1, 2` enums), so they are
      listed per library, like the borrowing callbacks.
    - F#'s `this`-returning methods keep `|> ignore`.
    - Hex colours stay floats.

---

# Target sketches: echarts/bar_chart, konva/shapes_gallery

### echarts/bar_chart

**Exercises:** conventions 1, 2, 3, 5, 7, 9

The section: the whole option object passed to `setOption` (title, grid, both axes, one bar series).

**TypeScript:**
```ts
chart.setOption({
  animation: false,
  backgroundColor: '#ffffff',
  title: { text: 'Monthly Output', left: 16, top: 12, textStyle: { color: '#111111', fontSize: 13 } },
  grid: { left: 48, right: 24, top: 48, bottom: 36 },
  xAxis: {
    type: 'category',
    data: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun'],
    axisLine: { lineStyle: { color: '#111111' } },
    axisLabel: { color: '#555555' },
  },
  yAxis: {
    type: 'value', min: 0, max: 100,
    splitLine: { lineStyle: { color: '#e5e5e5' } },
    axisLabel: { color: '#555555' },
  },
  series: { type: 'bar', name: 'Units', barWidth: '46%', data: [42, 68, 55, 84, 71, 93],
            itemStyle: { color: '#111111' } },
});
```

#### C
Today
```c
    JsArray_Echarts_CategoryAxisBase_data_or_string_or_number cats = JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_new();
    const char* months[] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun" };
    for (int i = 0; i < 6; i++) JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_push(cats, months[i]);
    // ...
        .title = EChartsOption_title_of(Echarts_TitleComponentOption_create(
            .text = "Monthly Output", .left = Position(16), .top = Position(12),
            .textStyle = Echarts_TitleTextStyleOption_create(.color = js_string("#111111"), .fontSize = Echarts_string_or_number_of(13)))),
        // ...
        .yAxis = EChartsOption_yAxis_of(Echarts_YAXisComponentOption_yAxis_value_create(
            .min = Bound(0), .max = Bound(100),
        // ...
        .series = EChartsOption_series_of(Echarts_BarSeriesOption_create(
            .name = Echarts_OptionName_of("Units"), .barWidth = Position("46%"), .data = vals,
            .itemStyle = Echarts_BarItemStyleOption_create(.color = Echarts_ItemStyleOption_color_of("#111111")))));
    ECharts_setOption(chart, opt);
```
(`Position` and `Bound` are macros the demo defines to hide `Echarts_PositionSizeOption_of` and
`Echarts_AngleAxisComponentOption_category_min_of`; `line_color()` is a helper the demo writes for the same reason.)

Target
```c
EChartsOption opt = EChartsOption_create(
    .animation = false,
    .backgroundColor = "#ffffff",
    .title = Echarts_TitleComponentOption_create(
        .text = "Monthly Output", .left = 16, .top = 12,
        .textStyle = Echarts_TitleTextStyleOption_create(.color = "#111111", .fontSize = 13)),
    .grid = Echarts_GridComponentOption_create(.left = 48, .right = 24, .top = 48, .bottom = 36),
    .xAxis_category = Echarts_CategoryXAxisOption_create(           /* (name TBD) */
        .data = js_strings("Jan", "Feb", "Mar", "Apr", "May", "Jun"),
        .axisLine = Echarts_AxisLineOption_create(
            .lineStyle = Echarts_LineStyleOption_create(.color = "#111111")),
        .axisLabel = Echarts_AxisLabelOption_create(.color = "#555555")),
    .yAxis_value = Echarts_ValueYAxisOption_create(                 /* (name TBD) */
        .min = 0, .max = 100,
        .splitLine = Echarts_SplitLineOption_create(
            .lineStyle = Echarts_LineStyleOption_create(.color = "#e5e5e5")),
        .axisLabel = Echarts_AxisLabelOption_create(.color = "#555555")),
    .series_bar = Echarts_BarSeriesOption_create(
        .name = "Units", .barWidth_string = "46%",
        .data = js_numbers(42, 68, 55, 84, 71, 93),
        .itemStyle = Echarts_BarItemStyleOption_create(.color = "#111111")));
ECharts_setOption(chart, opt);
```
Needs: 1 (nested `Pkg_Inner_create`), 2 (primary member plain: `.backgroundColor`, `.left`, `.fontSize`, `.name`, `.color`, `.min`; the others suffixed: `.barWidth_string`; `T | T[]` plain: `.title`, `.grid`; kinds of an all-object union each named: `.xAxis_category`, `.yAxis_value`, `.series_bar`), 3 (`js_strings`, `js_numbers`), 5 (`setOption` tail left out), 7 (one `LineStyleOption`, `AxisLabelOption`, `BarItemStyleOption`; inherited `color` read through `Base<X>`, a string on the title), 9 (axis kind names).

#### C++
Today
```cpp
        .title = TitleComponentOption({.top = 12, .left = 16, .text = "Monthly Output", .textStyle = TitleTextStyleOption({.color = "#111111", .fontSize = 13})}),
        .grid = GridComponentOption({.top = 48, .right = 24, .bottom = 36, .left = 48}),
        .xAxis = XAXisComponentOption_xAxis({
            .axisLabel = AxisLabelOption<std::string>({.color = "#555555"}),
            .data = ts::Array<std::string>::of("Jan", "Feb", "Mar", "Apr", "May", "Jun"),
            .axisLine = AxisLineOption({.lineStyle = LineStyleOption<Color>({.color = "#111111"})}),
        }),
        .yAxis = YAXisComponentOption_yAxis_value({
            .axisLabel = AxisLabelOption<std::string>({.color = "#555555"}),
            .splitLine = SplitLineOption({.lineStyle = LineStyleOption<SplitLineOption_lineStyle>({.color = "#e5e5e5"})}),
        // ...
        .series = BarSeriesOption({
            .data = ts::Array<double>::of(42, 68, 55, 84, 71, 93),
        // ...
            .itemStyle = BarItemStyleOption<>({.color = "#111111"}),
```
Target
```cpp
EChartsOption opt({
    .animation = false,
    .backgroundColor = "#ffffff",
    .grid = {{.bottom = 36, .left = 48, .right = 24, .top = 48}},
    .series = BarSeriesOption({             // one of 23 series kinds: named, then converts
        .barWidth = "46%",
        .data = {42, 68, 55, 84, 71, 93},
        .itemStyle = {{.color = "#111111"}},
        .name = "Units",
    }),
    .title = {{.left = 16, .text = "Monthly Output",
               .textStyle = {{.color = "#111111", .fontSize = 13}}, .top = 12}},
    .xAxis = CategoryXAxisOption({          // (name TBD)
        .axisLabel = {{.color = "#555555"}},
        .axisLine = {{.lineStyle = {{.color = "#111111"}}}},
        .data = {"Jan", "Feb", "Mar", "Apr", "May", "Jun"},
    }),
    .yAxis = ValueYAxisOption({             // (name TBD)
        .axisLabel = {{.color = "#555555"}},
        .max = 100,
        .min = 0,
        .splitLine = {{.lineStyle = {{.color = "#e5e5e5"}}}},
    }),
});
chart.setOption(opt);
```
Needs: 1 (`{{..}}` through `ts::Field<T>` taking `T::Fields`, designators in ASCII order), 2 (a constructor per member, from `Inner::Fields` for `.title`/`.grid`, an arithmetic template for `.min = 0`, `std::initializer_list` for `.data`), 3 (braced lists), 5 (merged `setOption`), 7 (no `<std::string>`, `<Color>`, `<SplitLineOption_lineStyle>`, `<>`), 9 (axis kind names).

#### Rust
Today
```rust
    let x_axis = XAXisComponentOptionXAxis::default()
        .with_data(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
        .with_axis_line(&AxisLineOption::default().with_line_style(
            &LineStyleOption::<JsValue>::default().with_color(JsValue::from("#111111")),
        ))
        .with_axis_label(&AxisLabelOption::default().with_color("#555555"));
    // ...
    let opt = EChartsOption::default().with_animation(false);
    opt.set_background_color("#ffffff");
    opt.set_title(&title);
    opt.set_grid(&grid);
    opt.set_x_axis(&x_axis);
    opt.set_y_axis(&y_axis);
    opt.set_series(&bars);
    chart.set_option(opt);
```
Target
```rust
let opt = EChartsOption::default()
    .with_animation(false)
    .with_background_color("#ffffff")
    .with_title(
        &TitleComponentOption::default()
            .with_text("Monthly Output")
            .with_left(16.0)
            .with_top(12.0)
            .with_text_style(&TitleTextStyleOption::default().with_color("#111111").with_font_size(13.0)),
    )
    .with_grid(&GridComponentOption::default().with_left(48.0).with_right(24.0).with_top(48.0).with_bottom(36.0))
    .with_x_axis(
        &CategoryXAxisOption::default() // (name TBD)
            .with_data(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
            .with_axis_line(&AxisLineOption::default().with_line_style(&LineStyleOption::default().with_color("#111111")))
            .with_axis_label(&AxisLabelOption::default().with_color("#555555")),
    )
    .with_y_axis(
        &ValueYAxisOption::default() // (name TBD)
            .with_min(0.0)
            .with_max(100.0)
            .with_split_line(&SplitLineOption::default().with_line_style(&LineStyleOption::default().with_color("#e5e5e5")))
            .with_axis_label(&AxisLabelOption::default().with_color("#555555")),
    )
    .with_series(
        &BarSeriesOption::default()
            .with_name("Units")
            .with_bar_width("46%")
            .with_data([42.0, 68.0, 55.0, 84.0, 71.0, 93.0])
            .with_item_style(&BarItemStyleOption::default().with_color("#111111")),
    );
chart.set_option(&opt);
```
Needs: 1 (builders), 2 (`impl Into`, from `&T` as today, through `T | T[]`), 3 (`[..]` of one kind), 5 (merged `set_option`), 7 (no `::<JsValue>`, no `JsValue::from`; `set_option`'s `Opt` erased to `EChartsCoreOption`), 9.

#### Gleam
Today
```gleam
        raw.create()
        |> unit.with_animation(False)
        |> unit.with_at("backgroundColor", dynamic.string("#ffffff"))
        // ...
        |> unit.with_at(
          "series",
          dynamic.array([
            obj([
              #("type", dynamic.string("bar")),
              #("name", dynamic.string("Units")),
              // ...
              #("itemStyle", obj([#("color", dynamic.string("#111111"))])),
            ]),
          ]),
        )
```
Target
```gleam
import echarts/bar_series_data     // element of BarSeriesOption.data (name TBD)
import echarts/category_axis_data  // element of the category axis data (name TBD)
import gleam/list
// ...plus one module per option type, as today

  let title =
    title_component_option.create()
    |> title_component_option.with_text("Monthly Output")
    |> title_component_option.with_left(16.0)
    |> title_component_option.with_top(12.0)
    |> title_component_option.with_text_style(
      title_text_style_option.create()
      |> title_text_style_option.with_color("#111111")
      |> title_text_style_option.with_font_size(13.0),
    )
  let grid =
    grid_component_option.create()
    |> grid_component_option.with_left(48.0)
    |> grid_component_option.with_right(24.0)
    |> grid_component_option.with_top(48.0)
    |> grid_component_option.with_bottom(36.0)
  let x_axis =
    category_x_axis_option.create()
    |> category_x_axis_option.with_data(
      list.map(["Jan", "Feb", "Mar", "Apr", "May", "Jun"], category_axis_data.string),
    )
    |> category_x_axis_option.with_axis_line(
      axis_line_option.create()
      |> axis_line_option.with_line_style(
        line_style_option.create() |> line_style_option.with_color("#111111"),
      ),
    )
    |> category_x_axis_option.with_axis_label(
      axis_label_option.create() |> axis_label_option.with_color("#555555"),
    )
  let y_axis =
    value_y_axis_option.create()
    |> value_y_axis_option.with_min(0.0)
    |> value_y_axis_option.with_max(100.0)
    |> value_y_axis_option.with_split_line(
      split_line_option.create()
      |> split_line_option.with_line_style(
        line_style_option.create() |> line_style_option.with_color("#e5e5e5"),
      ),
    )
    |> value_y_axis_option.with_axis_label(
      axis_label_option.create() |> axis_label_option.with_color("#555555"),
    )
  let bars =
    bar_series_option.create()
    |> bar_series_option.with_name("Units")
    |> bar_series_option.with_bar_width_string("46%")
    |> bar_series_option.with_data(
      list.map([42.0, 68.0, 55.0, 84.0, 71.0, 93.0], bar_series_data.number),
    )
    |> bar_series_option.with_item_style(
      bar_item_style_option.create() |> bar_item_style_option.with_color("#111111"),
    )
  let opt =
    e_charts_option.create()
    |> e_charts_option.with_animation(False)
    |> e_charts_option.with_background_color("#ffffff")
    |> e_charts_option.with_title(title)
    |> e_charts_option.with_grid(grid)
    |> e_charts_option.with_x_axis_category(x_axis)
    |> e_charts_option.with_y_axis_value(y_axis)
    |> e_charts_option.with_series_bar(bars)
  e_charts_type.set_option(chart, opt)
```
Needs: 1 (pipelines), 2 (primary member plain: `with_left(16.0)`, `with_color("#..")`, `with_name("Units")`, `with_min(0.0)`; the others suffixed: `with_bar_width_string`; `T | T[]` plain: `with_title(title)`; kinds of an all-object union each named: `with_x_axis_category`, `with_y_axis_value`, `with_series_bar`; union modules only for the two data lists), 3 (`list.map(.., bar_series_data.number)`), 5 (`set_option` without its tail), 7 (no `clr`/`t_cb_params`/`t_type` parameters, no `Codec`), 9.

#### F#
Today
```fsharp
let private setChartOption (chart: ECharts) (opt: obj) =
    chart.setOption (unbox<EChartsCoreOption> opt, false)
// ...
setChartOption chart {|
    animation = false
    backgroundColor = "#ffffff"
    title = {| text = "Monthly Output"; left = 16; top = 12; textStyle = {| color = "#111111"; fontSize = 13 |} |}
    // ...
    series = [|
        {|
            ``type`` = "bar"
            name = "Units"
            // ...
            itemStyle = {| color = "#111111" |}
        |}
```
Target
```fsharp
open Fable.Core.JsInterop

chart.setOption (
    EChartsOption.Create (
        animation = false,
        backgroundColor = "#ffffff",
        title = TitleComponentOption.Create (
            text = "Monthly Output", left = 16.0, top = 12.0,
            textStyle = TitleTextStyleOption.Create (color = "#111111", fontSize = 13.0)),
        grid = GridComponentOption.Create (left = 48.0, right = 24.0, top = 48.0, bottom = 36.0),
        xAxis = CategoryXAxisOption.Create (            // (name TBD)
            data = ResizeArray [ !^"Jan"; !^"Feb"; !^"Mar"; !^"Apr"; !^"May"; !^"Jun" ],
            axisLine = AxisLineOption.Create (lineStyle = LineStyleOption.Create (color = "#111111")),
            axisLabel = AxisLabelOption.Create (color = "#555555")),
        yAxis = ValueYAxisOption.Create (               // (name TBD)
            min = 0.0, max = 100.0,
            splitLine = SplitLineOption.Create (lineStyle = LineStyleOption.Create (color = "#e5e5e5")),
            axisLabel = AxisLabelOption.Create (color = "#555555")),
        series = BarSeriesOption.Create (
            name = "Units", barWidth = "46%",
            data = ResizeArray [ !^42.0; !^68.0; !^55.0; !^84.0; !^71.0; !^93.0 ],
            itemStyle = BarItemStyleOption.Create (color = "#111111"))))
```
Needs: 1 (`Create` named arguments), 2 (flat unions with an `op_Implicit` per member, at `Create`'s arguments), 3 (`ResizeArray [ !^.. ]`), 5 (merged `setOption`, so no `, false`), 7 (no `LineStyleOption<Color>`, no `unbox`), 9.

#### Notes (echarts/bar_chart)
- **Axis kinds need names (name TBD).** The kind suffixes come from the `type` discriminant (`_category`, `_value`, `_bar`); the type names `CategoryXAxisOption` and `ValueYAxisOption` are TBD.
- **C++ names the kind too.** `{{..}}` picks a union member by its designators: the category x-axis would resolve (only it has `data`), but the y-axis is ambiguous, since all four kinds have `min`/`max` (checked, clang 21). So both name their kind, as rule 2.4 does for C and Gleam.
- **Bare `TextCommonOption` (?).** The category `data` item `{ value, textStyle?: TextCommonOption }` uses it without arguments. Its constraint `Record<string, unknown>` must count as not useful, like `object`; erased to it by rule 7.2, the conditional `color` gives `unknown`.
- **A callback in an option field** is `_fn` in every language (convention 2): Rust `.with_color_fn(|p| ..)`, C `.color_fn` (function pointer plus context), Gleam `with_color_fn(fn(p) { .. })`. In F#, `fun (p: CallbackDataParams) -> ..` must return `Color`, so each string result needs `!^` (?).
- **`setOption` merge** (convention 5): `set_option(&opt)`, `chart.setOption opt`, `set_option(chart, opt)`; `_with` takes `SetOptionOpts`, and the `notMerge, lazyUpdate` overload is `set_option_with_not_merge`.
- **Behaviour:** the same program and chart. Gleam and F# now go through typed `create`, which (as in C/C++/Rust today) also writes `mainType` (`{"type":"category","mainType":"xAxis"}`; the bar series adds `"mainType":"series","progressiveChunkMode":"mod"`), and they pass `series` as the object rather than a one-element array. ECharts normalises both.

---

### konva/shapes_gallery

**Exercises:** conventions 1, 2, 3, 5, 6, 7, 11

The section: stage and layer setup, the caption helper, the two Row-1 rects (union `fill`/`stroke`/`cornerRadius`, `dash` list), and the framed text (a `Rect` and a `Text` in one `add`).

**TypeScript:**
```ts
const stage = new Konva.Stage({ container: 'stage', width: 680, height: 400 });
const layer = new Konva.Layer();
stage.add(layer);

function addCaption(x: number, y: number, text: string) {
  layer.add(new Konva.Text({ x, y, text, fontSize: 11, fontFamily: 'monospace', fill: '#555555' }));
}

layer.add(new Konva.Rect({ x: 40, y: 55, width: 110, height: 76, cornerRadius: 8, fill: '#111111' }));
addCaption(40, 142, 'Rect (filled)');
layer.add(new Konva.Rect({ x: 195, y: 55, width: 110, height: 76, fill: '#ffffff',
  stroke: '#111111', strokeWidth: 2, dash: [6, 4] }));
addCaption(195, 142, 'Rect (dashed)');
// ...Circle, RegularPolygon, Star, Ring, Wedge likewise
layer.add(
  new Konva.Rect({ x: 500, y: 224, width: 150, height: 88, fill: '#f7f7f7', stroke: '#111111', strokeWidth: 1.5 }),
  new Konva.Text({ x: 514, y: 240, text: 'KONVA 2D\nMonochrome\nScene Graph', fontSize: 13, fontFamily: 'monospace', fill: '#111111' }),
);
layer.draw();
```

#### C
Today
```c
    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node children = JsArray_Konva_Node_new();
    // ...
    Konva_RectConfig r2 = Konva_RectConfig_create(
        .x = 195, .y = 55, .width = 110, .height = 76, .fill = Konva_string_or_CanvasGradient_of("#ffffff"),
        .stroke = Konva_string_or_CanvasGradient_of("#111111"), .strokeWidth = 2,
        .dash = JsArray_double_from((double[]){ 6.0, 4.0 }, 2)
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(r2)));
    add_caption(children, 195, 142, "Rect (dashed)");
    // ...
    Konva_Layer_add(layer, children);
    Konva_Layer_draw(layer);
```
Target
```c
static void add_caption(Konva_Layer layer, double x, double y, const char* caption) {
    Konva_Layer_add(layer, Konva_Text_new(Konva_TextConfig_create(
        .x = x, .y = y, .text = caption, .fontSize = 11, .fontFamily = "monospace", .fill = "#555555")));
}

    Konva_Stage stage = Konva_Stage_new(Konva_StageConfig_create(
        .container = "stage", .width = 680, .height = 400));
    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);

    Konva_Layer_add(layer, Konva_Rect_new(Konva_RectConfig_create(
        .x = 40, .y = 55, .width = 110, .height = 76, .cornerRadius = 8, .fill = "#111111")));
    add_caption(layer, 40, 142, "Rect (filled)");

    Konva_Layer_add(layer, Konva_Rect_new(Konva_RectConfig_create(
        .x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff",
        .stroke = "#111111", .strokeWidth = 2, .dash = js_numbers(6, 4))));
    add_caption(layer, 195, 142, "Rect (dashed)");
    // ... Circle, RegularPolygon, Star, Ring, Wedge likewise

    Konva_Layer_add(layer,
        Konva_Rect_new(Konva_RectConfig_create(
            .x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7",
            .stroke = "#111111", .strokeWidth = 1.5)),
        Konva_Text_new(Konva_TextConfig_create(
            .x = 514, .y = 240, .text = "KONVA 2D\nMonochrome\nScene Graph", .fontSize = 13,
            .fontFamily = "monospace", .fill = "#111111")));
    add_caption(layer, 558, 325, "Text");
    Konva_Layer_draw(layer);
```
Needs: 1, 2 (primary member plain: `.fill`, `.stroke`, `.container = "stage"`, `.cornerRadius = 8`), 3 (`js_numbers`), 5 (`Konva_Layer_new()`, `Konva_Text_new(cfg)`), 6 (`Konva_Layer_add` takes one argument per child through `_Generic`; `Konva_Stage_add` with no rest values).

#### C++
Today
```cpp
static void add_caption(const Layer& layer, double x, double y, const char* label) {
    layer.add(Text::create({{.x = x, .y = y, .fill = "#555555", .text = label, .fontFamily = "monospace", .fontSize = 11}}));
}
// ...
    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

    Layer layer = Layer::create();
    stage.add(layer);
    // ...
    layer.add(Rect::create({{.x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2, .dash = ts::Array<double>::of(6.0, 4.0)}}));
    // ...
    layer.add(
        Rect::create({{.x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5}}),
        Text::create({{.x = 514, .y = 240, .fill = "#111111", .text = "KONVA 2D\nMonochrome\nScene Graph", .fontFamily = "monospace", .fontSize = 13}})
    );
```
Target
```cpp
static void add_caption(const Layer& layer, double x, double y, const char* label) {
    layer.add(Text::create({{.fill = "#555555", .fontFamily = "monospace", .fontSize = 11, .text = label, .x = x, .y = y}}));
}

    Stage stage = Stage::create({{.container = "stage", .height = 400, .width = 680}});
    Layer layer = Layer::create();
    stage.add(layer);

    layer.add(Rect::create({{.cornerRadius = 8.0, .fill = "#111111", .height = 76, .width = 110, .x = 40, .y = 55}}));
    add_caption(layer, 40, 142, "Rect (filled)");

    layer.add(Rect::create({{.dash = {6, 4}, .fill = "#ffffff", .height = 76, .stroke = "#111111", .strokeWidth = 2, .width = 110, .x = 195, .y = 55}}));
    add_caption(layer, 195, 142, "Rect (dashed)");
    // ... Circle, RegularPolygon, Star, Ring, Wedge likewise

    layer.add(
        Rect::create({{.fill = "#f7f7f7", .height = 88, .stroke = "#111111", .strokeWidth = 1.5, .width = 150, .x = 500, .y = 224}}),
        Text::create({{.fill = "#111111", .fontFamily = "monospace", .fontSize = 13, .text = "KONVA 2D\nMonochrome\nScene Graph", .x = 514, .y = 240}}));
    add_caption(layer, 558, 325, "Text");
    layer.draw();
```
Needs: 1 (designators in ASCII order), 2 (`fill`, `stroke`, `container`, `cornerRadius` convert), 3 (`.dash = {6, 4}`), 5 (overloads: `Layer::create()`), 6 (variadic `add`; `stage.add(layer)` with no rest values).

#### Rust
Today
```rust
    let stage = Stage::new(&sc);

    let layer = Layer::new();
    stage.add(&layer, JsArray::<JsValue>::new());
    // ...
    let tf = RectConfig::default()
        .with_x(500.0)
        // ...
        .with_stroke_width(1.5);
    layer.add([Rect::new_with(&tf)]);

    let tc = TextConfig::default()
        // ...
        .with_fill("#111111");
    layer.add([Text::new_with(&tc)]);
```
Target
```rust
fn add_caption(layer: &Layer, x: f64, y: f64, label: &str) {
    let tc = TextConfig::default()
        .with_x(x)
        .with_y(y)
        .with_text(label)
        .with_font_size(11.0)
        .with_font_family("monospace")
        .with_fill("#555555");
    layer.add([&Text::new_with(&tc)]);
}

    let stage = Stage::new(&StageConfig::default().with_container("stage").with_width(680.0).with_height(400.0));
    let layer = Layer::new();
    stage.add(&layer);

    let r1 = RectConfig::default()
        .with_x(40.0)
        .with_y(55.0)
        .with_width(110.0)
        .with_height(76.0)
        .with_corner_radius(8.0)
        .with_fill("#111111");
    layer.add([&Rect::new_with(&r1)]);
    add_caption(&layer, 40.0, 142.0, "Rect (filled)");

    let r2 = RectConfig::default()
        .with_x(195.0)
        .with_y(55.0)
        .with_width(110.0)
        .with_height(76.0)
        .with_fill("#ffffff")
        .with_stroke("#111111")
        .with_stroke_width(2.0)
        .with_dash([6.0, 4.0]);
    layer.add([&Rect::new_with(&r2)]);
    add_caption(&layer, 195.0, 142.0, "Rect (dashed)");
    // ... Circle, RegularPolygon, Star, Ring, Wedge likewise

    let frame = RectConfig::default()
        .with_x(500.0)
        .with_y(224.0)
        .with_width(150.0)
        .with_height(88.0)
        .with_fill("#f7f7f7")
        .with_stroke("#111111")
        .with_stroke_width(1.5);
    let label = TextConfig::default()
        .with_x(514.0)
        .with_y(240.0)
        .with_text("KONVA 2D\nMonochrome\nScene Graph")
        .with_font_size(13.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    layer.add([&Rect::new_with(&frame), &Text::new_with(&label)]);
    add_caption(&layer, 558.0, 325.0, "Text");
    layer.draw();
```
Needs: 1, 2 (`impl Into` for `fill`/`stroke`/`cornerRadius`/`container`), 5 (`Layer::new()`, `new_with(&cfg)`), 6 (`layer.add([&a, &b])` borrowed, mixed classes without `.into()`; `stage.add(&layer)` leaves the rest out, no `JsArray::<JsValue>`).

#### Gleam
Today
```gleam
fn add_caption(l: t.Layer(Nil), x: Float, y: Float, label: String) -> Nil {
  // ...
    |> text_config.with_fill(t.StringOrCanvasGradientString("#555555"))
  layer.add(l, [t.GroupOrShapeShape(shape.widen(text.new_with(Some(tc))))])
  Nil
}
// ...
    |> stage_config.with_container(t.HTMLDivElementOrStringString("stage"))
  // ...
  stage.add(stg, l, [])
  // ...
    |> rect_config.with_corner_radius(t.NumberOrNumberArrayNumber(8.0))
  // ...
    t.GroupOrShapeShape(shape.widen(rect.new_with(Some(tf)))),
    t.GroupOrShapeShape(shape.widen(text.new_with(Some(txt)))),
```
Target
```gleam
import konva/layer
import konva/node
import konva/rect
import konva/rect_config
import konva/stage
import konva/stage_config
import konva/text
import konva/text_config
import konva/types as t
// ...plus circle, regular_polygon, star, ring, wedge and their configs

fn add_caption(l: t.Layer, x: Float, y: Float, label: String) -> Nil {
  let tc =
    text_config.create()
    |> text_config.with_x(x)
    |> text_config.with_y(y)
    |> text_config.with_text(label)
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill("#555555")
  layer.add_shape(l, [text.new_with(tc)])
  Nil
}

pub fn main() -> Nil {
  let stg =
    stage.new(
      stage_config.create()
      |> stage_config.with_container("stage")
      |> stage_config.with_width(680.0)
      |> stage_config.with_height(400.0),
    )
  let l = layer.new()
  stage.add(stg, l)

  let r1 =
    rect_config.create()
    |> rect_config.with_x(40.0)
    |> rect_config.with_y(55.0)
    |> rect_config.with_width(110.0)
    |> rect_config.with_height(76.0)
    |> rect_config.with_corner_radius(8.0)
    |> rect_config.with_fill("#111111")
  let r2 =
    rect_config.create()
    |> rect_config.with_x(195.0)
    |> rect_config.with_y(55.0)
    |> rect_config.with_width(110.0)
    |> rect_config.with_height(76.0)
    |> rect_config.with_fill("#ffffff")
    |> rect_config.with_stroke("#111111")
    |> rect_config.with_stroke_width(2.0)
    |> rect_config.with_dash([6.0, 4.0])
  // ... circle, polygon, star, ring, wedge configs likewise
  let tf =
    rect_config.create()
    |> rect_config.with_x(500.0)
    |> rect_config.with_y(224.0)
    |> rect_config.with_width(150.0)
    |> rect_config.with_height(88.0)
    |> rect_config.with_fill("#f7f7f7")
    |> rect_config.with_stroke("#111111")
    |> rect_config.with_stroke_width(1.5)
  let txt =
    text_config.create()
    |> text_config.with_x(514.0)
    |> text_config.with_y(240.0)
    |> text_config.with_text("KONVA 2D\nMonochrome\nScene Graph")
    |> text_config.with_font_size(13.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill("#111111")

  layer.add_shape(l, [rect.new_with(r1), rect.new_with(r2)])
  // ... circle, polygon, star, ring, wedge: one call each
  layer.add_shape(l, [rect.new_with(tf)])
  layer.add_shape(l, [text.new_with(txt)])
  add_caption(l, 40.0, 142.0, "Rect (filled)")
  add_caption(l, 195.0, 142.0, "Rect (dashed)")
  // ...
  add_caption(l, 558.0, 325.0, "Text")
  node.draw(l)
```
Needs: 1, 2 (primary member plain: `with_fill("#..")`, `with_container("stage")`, `with_corner_radius(8.0)`; `Group | Shape` is all objects, so `add_shape` / `add_group`), 5 (`new_with(cfg)` without `Some`), 6 (`layer.add_shape` takes a list of one class, mixed classes in separate calls; `stage.add(stg, l)` leaves the rest out), 7 (`Layer.add` takes `Group | Shape` through `extends Container<Group | Shape>`; `Shape` without its config parameter; `t.Layer` without `(Nil)`).

#### F#
Today
```fsharp
let private addShape (s: Shape<ShapeConfig>) =
    layer.add s |> ignore
// ...
addShape (
    Rect.Create(
        RectConfig.Create (
            x = 40.0,
            y = 55.0,
            width = 110.0,
            height = 76.0,
            cornerRadius = 8.0,
            fill = "#111111"
        )
    ).asShape ()
)
```
Target
```fsharp
let private stage = Stage.Create (StageConfig.Create (container = "stage", width = 680.0, height = 400.0))
let private layer = Layer.Create ()
stage.add layer |> ignore

let private addCaption x y caption =
    layer.add (Text.Create (TextConfig.Create (x = x, y = y, text = caption, fontSize = 11.0, fontFamily = "monospace", fill = "#555555")))
    |> ignore

layer.add (Rect.Create (RectConfig.Create (x = 40.0, y = 55.0, width = 110.0, height = 76.0, cornerRadius = 8.0, fill = "#111111")))
|> ignore
addCaption 40.0 142.0 "Rect (filled)"

layer.add (
    Rect.Create (
        RectConfig.Create (
            x = 195.0, y = 55.0, width = 110.0, height = 76.0, fill = "#ffffff",
            stroke = "#111111", strokeWidth = 2.0, dash = ResizeArray [ 6.0; 4.0 ])))
|> ignore
addCaption 195.0 142.0 "Rect (dashed)"
// ... Circle, RegularPolygon, Star, Ring, Wedge likewise

layer.add (
    Rect.Create (RectConfig.Create (x = 500.0, y = 224.0, width = 150.0, height = 88.0, fill = "#f7f7f7", stroke = "#111111", strokeWidth = 1.5)),
    Text.Create (TextConfig.Create (x = 514.0, y = 240.0, text = "KONVA 2D\nMonochrome\nScene Graph", fontSize = 13.0, fontFamily = "monospace", fill = "#111111")))
|> ignore
addCaption 558.0 325.0 "Text"

layer.draw () |> ignore
```
Needs: 1, 2 (one flat `Group | Shape` with an `op_Implicit` from every `Shape` subclass, applied at each `ParamArray` element as today), 5 (`Layer.Create ()`), 6 (`ParamArray`), 7 (`Shape<ShapeConfig>` erased to `Shape`: no `.asShape ()`, no `addShape` wrapper), 11 (`|> ignore`).

#### Notes (konva/shapes_gallery)
- **Rust:** `layer.add([&r, &t])` mixing `&Rect` and `&Text` compiles only if `add` takes `[&dyn LayerChild; N]`, so that each element coerces; an `impl IntoIterator` parameter does not coerce them (both checked, rustc 1.99). Adding one child still needs `[&x]`.
- **Gleam `t.Layer`:** today's `(Nil)` is the subclass tag of the Gleam encoding, not a TypeScript parameter. Dropping it from annotations needs a generated alias for the exact class (`Layer = LayerOf(Nil)`, name TBD) (?).
- **Gleam `add_shape`:** rule 2.4 names both kinds of `Group | Shape`, so the common call reads `layer.add_shape(l, [..])`, and a list of one class splits the framed rect and its text into two calls.
- **Behaviour:** each language keeps today's order of children (Gleam still adds all shapes, then the captions, now one call per run of one class). C now adds shapes one call at a time instead of pushing them to one array added at the end, and Rust and F# add the framed rect and its text in one call instead of two: same children, same order.

---

### three/hierarchical_orrery
**Exercises:** conventions 1, 2, 5, 6, 7

**TypeScript:**
```ts
function stdMat(hex: number, roughness: number, metalness: number, flatShading: boolean) {
  return new THREE.MeshStandardMaterial({ color: hex, roughness, metalness, flatShading });
}
let moon: THREE.Mesh;                          // kept in app state for updateOrrery

const scene = new THREE.Scene();
scene.background = new THREE.Color(0xf7f7f7);
const key = new THREE.DirectionalLight(0xffffff, 2.2);
key.position.set(6, 10, 8);
scene.add(key);

const moonPivot = new THREE.Group();
innerPlanet.add(moonPivot);
moon = new THREE.Mesh(new THREE.OctahedronGeometry(0.18, 0), stdMat(0xd4d4d4, 0.4, 0.1, true));
moon.position.set(0.75, 0, 0);
moonPivot.add(moon);

scene.updateMatrixWorld(true);                 // in updateOrrery()
const mpos = moon.getWorldPosition(new THREE.Vector3());
```

#### C
Today
```c
#define add_obj(parent, child) \
    Three_Object3D_add(AS_Three_Object3D(parent), JsArray_Three_Object3D_from((Three_Object3D[]){ AS_Three_Object3D(child) }, 1))
static Three_MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    return Three_MeshStandardMaterial_new(Three_MeshStandardMaterialParameters_create(
        .color = Three_ColorRepresentation_of(hex), .roughness = rough, .metalness = metal, .flatShading = flat));
}
// ...
    Three_DirectionalLight key = Three_DirectionalLight_new(0xffffff, 2.2);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    add_obj(g_scene, key);
// ...
    g_moon = Three_Mesh_new(Three_OctahedronGeometry_new(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
    js_retain(g_moon);
    Three_Vector3_set(Three_Mesh_get_position(g_moon), 0.75, 0.0, 0.0);
    add_obj(g_moon_pivot, g_moon);
```
Target
```c
static Three_MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    return Three_MeshStandardMaterial_new(Three_MeshStandardMaterialParameters_create(
        .color_number = hex, .roughness = rough, .metalness = metal, .flatShading = flat));
}

static void update_orrery(void) {
    js_scope();
    /* ... rotations ... */
    Three_Scene_updateMatrixWorld(g_scene, true);
    Three_Vector3 mpos = Three_Mesh_getWorldPosition(g_moon, Three_Vector3_new());
    /* ... */
}

/* in start(): */
    g_scene = Three_Scene_new();
    js_retain(g_scene);
    Three_Scene_set_background(g_scene, Three_Color_new(0xf7f7f7));
    Three_DirectionalLight key = Three_DirectionalLight_new(0xffffff, 2.2);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    Three_Scene_add(g_scene, key);
    /* ... */
    Three_Mesh planet_mesh = Three_Mesh_new(Three_SphereGeometry_new(0.36, 20.0, 16.0), std_mat(0x555555, 0.55, 0.1, false));
    Three_Group_add(inner_planet, planet_mesh);
    g_moon_pivot = Three_Group_new();
    js_retain(g_moon_pivot);
    Three_Group_add(inner_planet, g_moon_pivot);
    g_moon = Three_Mesh_new(Three_OctahedronGeometry_new(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
    js_retain(g_moon);
    Three_Vector3_set(Three_Mesh_get_position(g_moon), 0.75, 0.0, 0.0);
    Three_Group_add(g_moon_pivot, g_moon);
```
Needs: 2 (`.color_number`: a C option field names each non-primary member; `Three_Mesh_new` takes a material or a `JsArray` through `_Generic`, as today), 6 (`Three_Scene_add(g_scene, key)`: one argument per value, so the `add_obj` macro goes). Optional arguments (5) are already left out as today.

#### C++
Today
```cpp
static MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    return MeshStandardMaterial::create({{.color = hex, .roughness = rough, .metalness = metal, .flatShading = flat}});
}
// ...
    Scene<> scene;
// ...
    Mesh<> moon;
// ...
    st->scene.updateMatrixWorld(true);
// ...
    Scene scene = Scene<>::create();
// ...
    Group moon_pivot = Group<>::create();
    Mesh moon = Mesh(OctahedronGeometry(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
```
Target
```cpp
static MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    return MeshStandardMaterial({.color = hex, .flatShading = flat, .metalness = metal, .roughness = rough});
}

struct AppState {
    WebGLRenderer renderer;
    Scene scene;
    PerspectiveCamera camera;
    Group inner_pivot;
    // ...
    Mesh moon;
    Mesh station_body;
    // ...
};

static void update_orrery(const std::shared_ptr<AppState>& st) {
    // ... rotations ...
    st->scene.updateMatrixWorld(true);
    Vector3 mpos = st->moon.getWorldPosition(Vector3::create());
    // ...
}

// in start():
    Scene scene = Scene::create();
    scene.background(Color(0xf7f7f7));
    DirectionalLight key = DirectionalLight(0xffffff, 2.2);
    key.position().set(6.0, 10.0, 8.0);
    // ...
    Group moon_pivot = Group::create();
    Mesh moon = Mesh(OctahedronGeometry(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
    moon.position().set(0.75, 0.0, 0.0);
    moon_pivot.add(moon);
    inner_planet.add(Mesh(SphereGeometry(0.36, 20.0, 16.0), std_mat(0x555555, 0.55, 0.1, false)), moon_pivot);
    // ...
    scene.add(AmbientLight(0xffffff, 1.2), key, rim, grid, pedestal, core, inner_pivot, outer_pivot);
```
Needs: 1 (single braces for a top-level option object, designators in ASCII order), 7 (`Scene`, `Group`, `Mesh` with no `<>`). Overloads (5), the variadic `add` (6) and the implicit `Material | Material[]` (2) are as today.

#### Rust
Today
```rust
    moon: Mesh<OctahedronGeometry, MeshStandardMaterial>,
// ...
    st.scene.update_matrix_world_with(true);
    let mpos = st.moon.get_world_position(&Vector3::new());
// ...
    let scene = Scene::new();
    scene.set_background(Color::new_with(0xf7f7f7 as f64));
// ...
    let key = DirectionalLight::new_with(ColorRepresentation::from(0xffffff as f64), 2.2);
    key.position().set_with(6.0, 10.0, 8.0);
// ...
    let moon = Mesh::new_with(
        Some(OctahedronGeometry::new_with(0.18, 0.0)),
        Some(std_mat(0xd4d4d4, 0.4, 0.1, true)),
    );
```
Target
```rust
fn std_mat(hex: u32, rough: f64, metal: f64, flat: bool) -> MeshStandardMaterial {
    let mp = MeshStandardMaterialParameters::default()
        .with_color(hex as f64)
        .with_roughness(rough)
        .with_metalness(metal)
        .with_flat_shading(flat);
    MeshStandardMaterial::new_with(&mp)
}

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene,
    // ...
    moon: Mesh,
    station_body: Mesh,
    // ...
}

fn update_orrery(st: &AppState) {
    // ... rotations ...
    st.scene.update_matrix_world(true);
    let mpos = st.moon.get_world_position(&Vector3::default());
    // ...
}

// in start():
    let scene = Scene::new();
    scene.set_background(Color::new(0xf7f7f7 as f64));
    let key = DirectionalLight::new(0xffffff as f64, 2.2);
    key.position().set(6.0, 10.0, 8.0);
    scene.add([&key]);
    // ...
    let planet = Mesh::new(
        &SphereGeometry::new_with(
            SphereGeometryArgs::default()
                .with_radius(0.36)
                .with_width_segments(20.0)
                .with_height_segments(16.0),
        ),
        &std_mat(0x555555, 0.55, 0.1, false),
    );
    inner_planet.add([&planet]);
    let moon_pivot = Group::new();
    inner_planet.add([&moon_pivot]);
    let moon = Mesh::new(&OctahedronGeometry::new(0.18, 0.0), &std_mat(0xd4d4d4, 0.4, 0.1, true));
    moon.position().set(0.75, 0.0, 0.0);
    moon_pivot.add([&moon]);
```
Needs: 5 (one function per call with `impl Optional<T>`: `DirectionalLight::new`, `set(6.0, 10.0, 8.0)`, `update_matrix_world(true)`, `Mesh::new`; `Vector3::default()` for `new Vector3()`; an arguments builder for `SphereGeometry`'s seven optionals), 2 (no `ColorRepresentation::from`; a `MeshStandardMaterial` converts to `Material | Material[]`, which counts as a class and so stays in `Mesh::new`), 6 (a borrowed list, no `.clone()`), 7 (`Mesh` with no type arguments).

#### Gleam
Today
```gleam
) -> t.MeshStandardMaterial(Nil) {
  // ...
  |> Some
  |> mesh_standard_material.new_with
  // ...
      t.ColorOrTextureColor(
        color.new_with(Some(t.ColorRepresentationNumber(hex(0xf7f7f7)))),
  // ...
      Some(octahedron_geometry.new_with(Some(0.18), Some(0.0))),
      Some(std_mat(0xd4d4d4, 0.4, 0.1, True)),
      t.mesh_standard_material_codec(),
  // ...
    object3d.update_matrix_world_with(sc, Some(True))
  // ...
        vector3.new_with(Some(0.0), Some(0.0), Some(0.0)),
```
Target
```gleam
import three/color
import three/directional_light
import three/group
import three/mesh
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/octahedron_geometry
import three/scene
import three/sphere_geometry
import three/sphere_geometry_args
import three/types as t
import three/vector3

fn std_mat(color_hex: Int, rough: Float, metal: Float, flat: Bool) -> t.MeshStandardMaterial {
  mesh_standard_material_parameters.create()
  |> mesh_standard_material_parameters.with_color_number(hex(color_hex))
  |> mesh_standard_material_parameters.with_roughness(rough)
  |> mesh_standard_material_parameters.with_metalness(metal)
  |> mesh_standard_material_parameters.with_flat_shading(flat)
  |> mesh_standard_material.new_with
}

pub fn main() {
  // ...
  let sc = scene.new()
  scene.set_background_color(sc, color.new_number_with(hex(0xf7f7f7)))
  let key = directional_light.new_number_with(hex(0xffffff), 2.2)
  object3d.get_position(key) |> vector3.set_with(6.0, 10.0, 8.0)
  object3d.add(sc, [key])
  // ...
  let planet =
    mesh.new_with(
      sphere_geometry_args.new()
        |> sphere_geometry_args.with_radius(0.36)
        |> sphere_geometry_args.with_width_segments(20.0)
        |> sphere_geometry_args.with_height_segments(16.0)
        |> sphere_geometry.new_with,
      std_mat(0x555555, 0.55, 0.1, False),
    )
  object3d.add(inner_planet, [planet])
  let moon_pivot = group.new()
  object3d.add(inner_planet, [moon_pivot])
  let moon =
    mesh.new_with(
      octahedron_geometry.new_with(0.18, 0.0),
      std_mat(0xd4d4d4, 0.4, 0.1, True),
    )
  object3d.get_position(moon) |> vector3.set_with(0.75, 0.0, 0.0)
  object3d.add(moon_pivot, [moon])
  // ...
  let update_orrery = fn() {
    // ... rotations ...
    object3d.update_matrix_world_with(sc, True)
    let mpos = object3d.get_world_position(moon, vector3.new())
    // ...
  }
  // ...
}
```
Needs: 2 (`Color` is `ColorRepresentation`'s primary member, so a hex number takes the suffix: `with_color_number`, `color.new_number_with`, `directional_light.new_number_with`; `Color | Texture` has none, so `set_background_color`, and null would be `set_background_null(sc)`, with no `Option`; `mesh.new_with` takes one `Material`, `mesh.new_list_with` several), 5 (`_with` takes the tail bare; an arguments builder for `SphereGeometry`), 6 (a list of one class), 7 (no `(Nil)`, no material codec argument).

#### F#
Today
```fsharp
    let scene = Scene.Create()
    scene.background <- Some (ColorOrTexture.Color (Color.Create (float 0xf7f7f7)))
    // ...
    let moon =
        Mesh.Create(
            OctahedronGeometry.Create(0.18, 0.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = float 0xd4d4d4,
                    roughness = 0.4,
                    flatShading = true
    // ...
        scene.updateMatrixWorld true
        let moonPos = moon.getWorldPosition (Vector3.Create ())
```
Target
```fsharp
    let scene = Scene.Create()
    scene.background <- Color.Create(float 0xf7f7f7)
    // ...
    let keyLight = DirectionalLight.Create(float 0xffffff, 2.2)
    keyLight.position.set (6.0, 10.0, 8.0) |> ignore
    scene.add (keyLight) |> ignore
    // ...
    let planetMesh =
        Mesh.Create(
            SphereGeometry.Create(0.36, 20.0, 16.0),
            MeshStandardMaterial.Create(MeshStandardMaterialParameters.Create(color = float 0x555555, roughness = 0.55))
        )
    innerPlanet.add (planetMesh) |> ignore
    let moonPivot = Group.Create()
    innerPlanet.add (moonPivot) |> ignore
    let moon =
        Mesh.Create(
            OctahedronGeometry.Create(0.18, 0.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(color = float 0xd4d4d4, roughness = 0.4, flatShading = true)
            )
        )
    moon.position.set (0.75, 0.0, 0.0) |> ignore
    moonPivot.add (moon) |> ignore
    // ...
    let updateOrrery () =
        // ... rotations ...
        scene.updateMatrixWorld true
        let moonPos = moon.getWorldPosition (Vector3.Create ())
        // ...
```
Needs: 2 (a nullable union property takes a member, or `null`, with no `Some` and no `ColorOrTexture.Color`; one flat `Material | Material[]` with an `op_Implicit` from every `Material` subclass, so `MeshStandardMaterial` passes as is), 7 (`Mesh` erased; F# infers its arguments today, so the text does not change). Optional arguments (5), the variadic `add` (6) and `|> ignore` (11) are as today.

**Notes**
- The targets add the inner planet (`new THREE.SphereGeometry(0.36, 20, 16)`, 3 of 7 optionals), which the TypeScript excerpt leaves out, to show the arguments builder in Rust and Gleam. C, C++ and F# write it as today.
- Gleam inherited methods stay in the declaring module (`object3d.add(sc, ..)`, `object3d.get_position(key)`), while C reaches them through the class's own prefix (`Three_Scene_add`). I read "one module per class" as "a class is not split across modules" (?).
- Gleam `t.MeshStandardMaterial` without `(Nil)` needs a non-generic alias over the phantom subtype tag, and types stay in `three/types` (?).
- The arguments builder is passed by value in Rust (`SphereGeometry::new_with(SphereGeometryArgs::default()..)`), unlike an options object (`new_with(&mp)`). In Gleam it starts with `new()`, as HEADER's example does, unlike an option object's `create()` (?).
- The F# demo already behaves differently from the others: it has no `std_mat`, and its moon and planet omit `metalness`. The F# sketch keeps F#'s own behaviour.

### rapier/collision_events_sensor
**Exercises:** conventions 5, 7, 8, 9, 11

**TypeScript:**
```ts
let world = new RAPIER.World({ x: 0.0, y: -9.81, z: 0.0 });
let queue = new RAPIER.EventQueue(true);
const sensorBody = world.createRigidBody(RAPIER.RigidBodyDesc.fixed().setTranslation(0, 1.35, 0));
const sensorDesc = RAPIER.ColliderDesc.cuboid(1.85, 0.3, 1.2)
  .setSensor(true).setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
sensorHandle = world.createCollider(sensorDesc, sensorBody).handle;

function stepSim() {
  world.step(queue);
  stepCount++;
  queue.drainCollisionEvents((h1, h2, started) => {
    if (h1 === sensorHandle || h2 === sensorHandle) {
      if (started) { sensorCrossings++; activeInSensor++; }
      else if (activeInSensor > 0) activeInSensor--;
    } else if (started) solidContacts++;
  });
}
const loop = () => { if (!paused) { stepSim(); render(); } requestAnimationFrame(loop); };
requestAnimationFrame(loop);
```

#### C
Today
```c
static Rapier_EventQueue_drainCollisionEvents_f g_col_cb;
// ...
    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
// ...
    Rapier_ColliderDesc scd = Rapier_ColliderDesc_setActiveEvents(Rapier_ColliderDesc_setSensor(Rapier_ColliderDesc_cuboid(1.85, 0.30, 1.2), true), Rapier_ActiveEvents_COLLISION_EVENTS);
    Rapier_Collider sc = Rapier_World_createCollider(g_world, scd, sbody);
// ...
static void step_sim(void) {
    Rapier_World_step(g_world, g_queue);
    g_step_count++;
    Rapier_EventQueue_drainCollisionEvents(g_queue, g_col_cb);
}
// ...
    g_col_cb = Rapier_EventQueue_drainCollisionEvents_f_from(on_collision, NULL);
    js_retain(g_col_cb);
```
Target
```c
static Rapier_DrainCollisionEventsFn g_col_cb;

static void on_collision(void* ctx, double h1, double h2, bool started) {
    (void)ctx;
    if (h1 == g_sensor_handle || h2 == g_sensor_handle) {
        if (started) {
            g_sensor_crossings++;
            g_active_in_sensor++;
        } else if (g_active_in_sensor > 0) {
            g_active_in_sensor--;
        }
    } else if (started) {
        g_solid_contacts++;
    }
}

/* in build_scene(): */
    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);
    g_queue = Rapier_EventQueue_new(true);
    js_retain(g_queue);
    /* ... */
    Rapier_RigidBody sbody = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, 1.35, 0.0));
    Rapier_ColliderDesc scd = Rapier_ColliderDesc_setActiveEvents(Rapier_ColliderDesc_setSensor(Rapier_ColliderDesc_cuboid(1.85, 0.30, 1.2), true), Rapier_ActiveEvents_COLLISION_EVENTS);
    Rapier_Collider sc = Rapier_World_createCollider(g_world, scd, sbody);
    g_sensor_handle = Rapier_Collider_get_handle(sc);

static void step_sim(void) {
    Rapier_World_step(g_world, g_queue);
    g_step_count++;
    Rapier_EventQueue_drainCollisionEvents(g_queue, g_col_cb);
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    js_scope();
    if (!g_paused) {
        step_sim();
        render();
    }
    requestAnimationFrame(g_raf_cb);
}

/* in on_init(): */
    g_col_cb = Rapier_DrainCollisionEventsFn_from(on_collision, NULL);
    js_retain(g_col_cb);
    /* ... */
    g_raf_cb = FrameRequestCallback_from(on_raf, NULL);
    js_retain(g_raf_cb);
    requestAnimationFrame(g_raf_cb);
```
Needs: 9 only (`Rapier_DrainCollisionEventsFn`, the callback type's concept name). Everything else is already the target: trailing optionals are left out (5), and a callback is a function pointer plus context.

#### C++
Today
```cpp
    EventQueue_drainCollisionEvents_f col_cb;
    dom::FrameRequestCallback raf_cb;
// ...
    st->world = World(Vector3(0.0, -9.81, 0.0));
    st->queue = EventQueue(true);
// ...
    st->world->step(*st->queue);
    st->step_count++;
    st->queue->drainCollisionEvents(st->col_cb);
// ...
    st->col_cb = [st](double h1, double h2, bool started) {
// ...
    st->raf_cb = [st](double) {
// ...
        dom::requestAnimationFrame(st->raf_cb);
```
Target
```cpp
struct AppState {
    // ...
    bool paused = false;
    // no col_cb or raf_cb: both callbacks are passed directly
};

static void build_scene(const std::shared_ptr<AppState>& st) {
    // ...
    st->world = World(Vector3(0.0, -9.81, 0.0));
    st->queue = EventQueue(true);
    // ...
    RigidBody sbody = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(0.0, 1.35, 0.0));
    Collider sc = st->world->createCollider(
        ColliderDesc::cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents(ActiveEvents::COLLISION_EVENTS),
        sbody
    );
    st->sensor_handle = sc.handle();
    // ...
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step(*st->queue);
    st->step_count++;
    st->queue->drainCollisionEvents([&](double h1, double h2, bool started) {
        if (h1 == st->sensor_handle || h2 == st->sensor_handle) {
            if (started) {
                st->sensor_crossings++;
                st->active_in_sensor++;
            } else if (st->active_in_sensor > 0) {
                st->active_in_sensor--;
            }
        } else if (started) {
            st->solid_contacts++;
        }
    });
}

static void run_frames(std::shared_ptr<AppState> st) {
    dom::requestAnimationFrame([st](double) {
        if (!st->paused) {
            step_sim(st);
            render(st);
        }
        run_frames(st);
    });
}

// in run_app(), replacing raf_cb:
    run_frames(st);
```
Needs: 8 (lambdas passed straight to the method: `[&]` for the drain, on Rapier's borrowing list; `[st]` by value for `requestAnimationFrame`, freed by `FinalizationRegistry`; no stored `col_cb` or `raf_cb`). Overloads (5) are as today.

#### Rust
Today
```rust
    let world = World::new(&Vector3::new(0.0, -9.81, 0.0));
    let queue = EventQueue::new(true);
// ...
        w.step_with(q, None);
        EventQueueDrainCollisionEventsF::scoped(
            |h1: f64, h2: f64, started: bool| {
                if h1 == sh || h2 == sh {
// ...
            |cb| q.drain_collision_events(cb),
        );
// ...
        let raf_holder: Rc<RefCell<Option<FrameRequestCallback>>> = Rc::new(RefCell::new(None));
        let raf_c = raf_holder.clone();
        let st_c = st.clone();
        *raf_holder.borrow_mut() = Some(FrameRequestCallback::new(move |_: f64| {
```
Target
```rust
fn build_scene(st: &mut AppState) {
    // ...
    let world = World::new(&Vector3::new(0.0, -9.81, 0.0));
    let queue = EventQueue::new(true, None);
    // ...
    let sbody = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation(0.0, 1.35, 0.0));
    let sc = world.create_collider(
        &ColliderDesc::cuboid(1.85, 0.30, 1.2)
            .set_sensor(true)
            .set_active_events(ActiveEvents::CollisionEvents),
        &sbody,
    );
    st.sensor_handle = sc.handle();
    // ...
}

fn step_sim(st: &mut AppState) {
    let sh = st.sensor_handle;
    if let (Some(w), Some(q)) = (st.world.as_ref(), st.queue.as_ref()) {
        w.step(q);
        q.drain_collision_events(|h1, h2, started| {
            if h1 == sh || h2 == sh {
                if started {
                    st.sensor_crossings += 1;
                    st.active_in_sensor += 1;
                } else if st.active_in_sensor > 0 {
                    st.active_in_sensor -= 1;
                }
            } else if started {
                st.solid_contacts += 1;
            }
        });
    }
    st.step_count += 1;
}

fn run_frames(st: Rc<RefCell<AppState>>) {
    dom::request_animation_frame(move |_| {
        {
            let mut s = st.borrow_mut();
            if !s.paused {
                step_sim(&mut s);
                render(&s);
            }
        }
        run_frames(st.clone());
    });
}

// in start(), replacing raf_holder:
        run_frames(st.clone());
```
Needs: 8 (closures passed directly: a borrowing `FnMut` for `EventQueue.drainCollisionEvents`, which is on Rapier's list; a `'static` one for `requestAnimationFrame`, freed by `FinalizationRegistry`), 5 (`create_collider(&desc, &sbody)` and `step(q)`: one function each, with the trailing `hooks` options object left to `step_with`; `EventQueue::new(true, None)`, see Notes).

#### Gleam
Today
```gleam
fn vec(x: Float, y: Float, z: Float) -> rt.Vector(Nil) {
// ...
fn start_loop(tick: fn() -> Nil) -> Nil {
  core.request_animation_frame(fn(_) {
// ...
    let w = world.new(vec(0.0, -9.81, 0.0))
    let q = event_queue.new(True)
// ...
      |> collider_desc.set_active_events(rt.ActiveEventsCollisionEvents)
    let sc = world.create_collider_with(w, scd, Some(sbody))
// ...
        world.step_with(w, Some(q), None)
        rp_rt.ref_update(step_ref, fn(s) { s + 1 })
        let sh = rp_rt.ref_get(sensor_handle_ref)
        event_queue.drain_collision_events(q, fn(h1, h2, started) {
```
Target
```gleam
import dimforge_rapier3d_compat/active_events

fn vec(x: Float, y: Float, z: Float) -> rt.Vector {
  vector.create()
  |> vector.with_x(x)
  |> vector.with_y(y)
  |> vector.with_z(z)
}

fn start_loop(tick: fn() -> Nil) -> Nil {
  core.request_animation_frame(fn(_) {
    tick()
    start_loop(tick)
  })
  Nil
}

pub fn main() {
  // ...
  let build_scene = fn() {
    // ...
    let w = world.new(vec(0.0, -9.81, 0.0))
    let q = event_queue.new(True)
    // ...
    let sbody =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed() |> rigid_body_desc.set_translation(0.0, 1.35, 0.0),
      )
    let scd =
      collider_desc.cuboid(1.85, 0.3, 1.2)
      |> collider_desc.set_sensor(True)
      |> collider_desc.set_active_events([active_events.CollisionEvents])
    let sc = world.create_collider_with(w, scd, sbody)
    rp_rt.ref_set(sensor_handle_ref, collider.get_handle(sc))
    // ...
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref), rp_rt.ref_get(queue_ref) {
      Some(w), Some(q) -> {
        world.step_with(w, q, None)
        rp_rt.ref_update(step_ref, fn(s) { s + 1 })
        let sh = rp_rt.ref_get(sensor_handle_ref)
        event_queue.drain_collision_events(q, fn(h1, h2, started) {
          case h1 == sh || h2 == sh, started {
            True, True -> {
              rp_rt.ref_update(crossings_ref, fn(c) { c + 1 })
              rp_rt.ref_update(active_ref, fn(a) { a + 1 })
              Nil
            }
            True, False -> {
              rp_rt.ref_update(active_ref, fn(a) { int.max(0, a - 1) })
              Nil
            }
            False, True -> {
              rp_rt.ref_update(contacts_ref, fn(c) { c + 1 })
              Nil
            }
            False, False -> Nil
          }
        })
      }
      _, _ -> Nil
    }
  }
  // ...
  start_loop(fn() {
    case rp_rt.ref_get(paused_ref) {
      False -> {
        step_sim()
        render()
      }
      True -> Nil
    }
  })
}
```
Needs: 5 (`create_collider_with` takes `parent` bare; in `step_with(w, q, None)` the queue is bare and `hooks`, an interface of methods ending a tail of two, is `Option`), 7 (`rt.Vector` with no `(Nil)`), 9 (`active_events.CollisionEvents`: an enum member in the enum's module), 11 (a flag enum is a list). Callbacks (8) are already plain functions.

#### F#
Today
```fsharp
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))
            let q = EventQueue.Create true
            // ...
            let sensorCollider = w.createCollider (sensorDesc, sensorBody)
            // ...
        let stepSim () =
            world.step queue
            stepCount <- stepCount + 1
            queue.drainCollisionEvents (fun h1 h2 started ->
                if h1 = sensorHandle || h2 = sensorHandle then
            // ...
        let rec loop (_ts: float) =
            if not paused then
            // ...
            requestAnimationFrame loop |> ignore
```
Target (unchanged; this already reads as the target)
```fsharp
        let createScene () =
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))
            let q = EventQueue.Create true
            // ...
            let sensorBody = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, 1.35, 0.0))
            let sensorDesc =
                ColliderDesc.cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents(ActiveEvents.COLLISION_EVENTS)
            let sensorCollider = w.createCollider (sensorDesc, sensorBody)
            let sHandle = sensorCollider.handle
            // ...

        let stepSim () =
            world.step queue
            stepCount <- stepCount + 1
            queue.drainCollisionEvents (fun h1 h2 started ->
                if h1 = sensorHandle || h2 = sensorHandle then
                    if started then
                        sensorCrossings <- sensorCrossings + 1
                        activeInSensor <- activeInSensor + 1
                    else
                        if activeInSensor > 0 then activeInSensor <- activeInSensor - 1
                elif started then
                    solidContacts <- solidContacts + 1
            )

        let rec loop (_ts: float) =
            if not paused then
                stepSim ()
                render ()
            requestAnimationFrame loop |> ignore

        requestAnimationFrame loop |> ignore
```
Needs: nothing new. Optional arguments (5), plain-function callbacks (8) and `World.Create(gravity)` already work.

**Notes**
- Rust `EventQueue::new(true, None)` is a known residue: Rapier's constructors end in `raw?: RawEventQueue`, a wasm-bindgen class that users never pass, and convention 5 keeps a class optional in the plain name. `World::new(&gravity)` avoids it only because its 13 `raw*` optionals take the arguments-builder rule.
- C keeps the drain and frame callbacks that today's demo creates once and retains (`g_col_cb`, `g_raf_cb`). Building `Rapier_DrainCollisionEventsFn_from(..)` inside `step_sim` would make a JavaScript function every frame. I read convention 8's "passed directly" as not changing C's function pointer plus context (?).
- Gleam `world.new(gravity: Vector)` takes the `Vector` interface. A `Vector3`, a class that implements it, is a different phantom type, so the demo's `vec` option-object builder remains (?).
- `ActiveEvents` is a flag enum only because Rapier's per-library list says so (convention 11).

---

### date_fns/date_arithmetic
**Exercises:** conventions 1, 2, 5, 7 (Duration options object; `DateArg = DateType | number | string` going in, primary member `Date` as the first declared; `options?` left out; `DateType`/`ResultDate` erased to `Date`)

Section: parse the two dates, build the duration, fill the operations table, and compute each row's cells.

**TypeScript:**
```ts
const base = parseISO("2024-03-15");
const jan31 = parseISO("2024-01-31");
const dur: Duration = { months: 1, days: 2 };

const ops: [string, Date, string][] = [
  ["addDays(base, 3)", addDays(base, 3), "yyyy-MM-dd"],
  ["subDays(base, 15)", subDays(base, 15), "yyyy-MM-dd"],
  // addWeeks(base, 2), addMonths(jan31, 1), subMonths(base, 3), addYears(base, 1): same shape
  ["startOfWeek(base)", startOfWeek(base), "yyyy-MM-dd"],
  ["endOfMonth(base)", endOfMonth(base), "yyyy-MM-dd HH:mm:ss"],
  ["add(base, { months: 1, days: 2 })", add(base, dur), "yyyy-MM-dd"],
  ["base (immutable check)", base, "yyyy-MM-dd"],
];
for (const [name, res, fmt] of ops) {
  const dd = differenceInCalendarDays(res, base);
  const dh = differenceInHours(res, base);
  addRow(tbody, name, formatDate(res, fmt), signed(dd), signed(dh));
}
kv("formatDuration(dur)", formatDuration(dur));
```

#### C
Today
```c
    int dd = (int)df_differenceInCalendarDays(res, base_d);
    int dh = (int)df_differenceInHours(res, base_d);
// ...
    HTMLElement_set_textContent(td_out, df_formatDate(res, fmt));
// ...
    Date base_d = df_parseISO("2024-03-15");
    Date jan31 = df_parseISO("2024-01-31");

    Df_Duration dur = Df_Duration_create(.months = 1, .days = 2);
// ...
    add_op_row(doc, tbody, "addDays(base, 3)", df_addDays(base_d, 3.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "addMonths(jan31, 1)", df_addMonths(jan31, 1.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "startOfWeek(base)", df_startOfWeek(base_d), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "add(base, { months: 1, days: 2 })", df_add(base_d, dur), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd");
```
Target (unchanged: C already reads like `addDays(date, 3)`)
```c
    int dd = (int)df_differenceInCalendarDays(res, base_d);
    int dh = (int)df_differenceInHours(res, base_d);
// ...
    HTMLElement_set_textContent(td_out, df_formatDate(res, fmt));
// ...
    Date base_d = df_parseISO("2024-03-15");
    Date jan31 = df_parseISO("2024-01-31");
    Df_Duration dur = Df_Duration_create(.months = 1, .days = 2);

    add_op_row(doc, tbody, "addDays(base, 3)", df_addDays(base_d, 3.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "subDays(base, 15)", df_subDays(base_d, 15.0), base_d, "yyyy-MM-dd");
    /* addWeeks, addMonths(jan31, 1.0), subMonths, addYears: same shape */
    add_op_row(doc, tbody, "startOfWeek(base)", df_startOfWeek(base_d), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "endOfMonth(base)", df_endOfMonth(base_d), base_d, "yyyy-MM-dd HH:mm:ss");
    add_op_row(doc, tbody, "add(base, { months: 1, days: 2 })", df_add(base_d, dur), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd");
```
Needs: 1 (`Df_Duration_create`), 2 (`_Generic` takes the `Date` member of `DateArg`), 5 (trailing `options` left out), 7 (`df_addDays` returns `Date`); all present today.

#### C++
Today
```cpp
    auto base_d = date_fns::parseISO("2024-03-15");
    auto jan31 = date_fns::parseISO("2024-01-31");
    auto dur = date_fns::Duration({.months = 1.0, .days = 2.0});

    struct OpRow { const char* name; date_fns::Date res; const char* fmt; };
    OpRow ops[] = {
        { "addDays(base, 3)", date_fns::addDays(base_d, 3.0), "yyyy-MM-dd" },
        // ...
        { "add(base, { months: 1, days: 2 })", date_fns::add(base_d, dur), "yyyy-MM-dd" },
        { "base (immutable check)", base_d, "yyyy-MM-dd" },
    };
    // ...
        tr.appendChild(make_el(doc, "td", "", date_fns::formatDate(op.res, op.fmt)));
        int dd = static_cast<int>(date_fns::differenceInCalendarDays(op.res, base_d));
        int dh = static_cast<int>(date_fns::differenceInHours(op.res, base_d));
```
Target (only the designator order changes)
```cpp
    auto base_d = date_fns::parseISO("2024-03-15");
    auto jan31 = date_fns::parseISO("2024-01-31");
    auto dur = date_fns::Duration({.days = 2.0, .months = 1.0});

    struct OpRow { const char* name; date_fns::Date res; const char* fmt; };
    OpRow ops[] = {
        { "addDays(base, 3)", date_fns::addDays(base_d, 3.0), "yyyy-MM-dd" },
        { "subDays(base, 15)", date_fns::subDays(base_d, 15.0), "yyyy-MM-dd" },
        // addWeeks, addMonths(jan31, 1.0), subMonths, addYears: same shape
        { "startOfWeek(base)", date_fns::startOfWeek(base_d), "yyyy-MM-dd" },
        { "endOfMonth(base)", date_fns::endOfMonth(base_d), "yyyy-MM-dd HH:mm:ss" },
        { "add(base, { months: 1, days: 2 })", date_fns::add(base_d, dur), "yyyy-MM-dd" },
        { "base (immutable check)", base_d, "yyyy-MM-dd" },
    };
    for (const auto& op : ops) {
        // ... <tr> with op.name, then:
        tr.appendChild(make_el(doc, "td", "", date_fns::formatDate(op.res, op.fmt)));
        int dd = static_cast<int>(date_fns::differenceInCalendarDays(op.res, base_d));
        int dh = static_cast<int>(date_fns::differenceInHours(op.res, base_d));
        // ...
    }
```
Needs: 1 (`Duration({..})`, designators in ASCII order, so `.days` before `.months`), 2 (`DateArg` is a class with a non-template constructor from `Date`, `double` and `std::string`, not `std::variant<DateType, ..>`), 5 (overloads), 7 (`DateType`/`ResultDate` erased to `Date`: no defaulted template parameters, `addDays` returns `Date`).

#### Rust
Today
```rust
use date_fns::{Date, DateArg, Duration};
// ...
fn day(d: &Date) -> DateArg<Date> {
    DateArg::DateType(d.clone())
}
// ...
    let base_d: Date = date_fns::parse_iso("2024-03-15");
    let dur = Duration::default().with_months(1.0).with_days(2.0);
// ...
            date_fns::add_days(day(&base_d), 3.0),
// ...
            date_fns::add(day(&base_d), &dur),
// ...
        let dd = date_fns::difference_in_calendar_days(res, &base_d) as i32;
        let dh = date_fns::difference_in_hours(res, &base_d) as i32;
```
Target
```rust
use date_fns::Duration;

    let base_d = date_fns::parse_iso("2024-03-15");
    let jan31 = date_fns::parse_iso("2024-01-31");
    let dur = Duration::default().with_months(1.0).with_days(2.0);

    let ops = [
        ("addDays(base, 3)", date_fns::add_days(&base_d, 3.0), "yyyy-MM-dd"),
        ("subDays(base, 15)", date_fns::sub_days(&base_d, 15.0), "yyyy-MM-dd"),
        // add_weeks, add_months(&jan31, 1.0), sub_months, add_years: same shape
        ("startOfWeek(base)", date_fns::start_of_week(&base_d), "yyyy-MM-dd"),
        ("endOfMonth(base)", date_fns::end_of_month(&base_d), "yyyy-MM-dd HH:mm:ss"),
        ("add(base, { months: 1, days: 2 })", date_fns::add(&base_d, &dur), "yyyy-MM-dd"),
        ("base (immutable check)", base_d.clone(), "yyyy-MM-dd"),
    ];
    for (op_name, res, fmt) in &ops {
        // ... <tr> with op_name, then:
        append(&tr, make_el(&doc, "td", "", Some(&date_fns::format_date(res, fmt))));
        let dd = date_fns::difference_in_calendar_days(res, &base_d) as i32;
        let dh = date_fns::difference_in_hours(res, &base_d) as i32;
        // ...
    }
```
Needs: 2 (`impl Into<DateArg>` takes `&Date`), 5 (plain names leave `options` out; `add_days_with` takes it), 7 (`DateType`/`ResultDate` erased to `Date`: `DateArg` is not generic and has `From<&Date>`, `add_days` returns `Date`, so the `day()` helper and the `: Date` annotations go).

#### Gleam
Today
```gleam
      let day = dt.DateOrStringOrNumberDate
      let arg = dt.DateArgDateType
// ...
      let ops = [
        #("addDays(base, 3)", df.add_days(arg(base_d), 3.0), "yyyy-MM-dd"),
// ...
        #(
          "add(base, { months: 1, days: 2 })",
          df.add(arg(base_d), dur),
          "yyyy-MM-dd",
        ),
// ...
        node.set_text_content(td_res, Some(df.format_date(day(res), fmt)))
// ...
            fmt_signed(df.difference_in_calendar_days(day(res), day(base_d))),
```
Target
```gleam
import date_fns as df
import date_fns/duration

      let base_d = df.parse_iso("2024-03-15")
      let jan31 = df.parse_iso("2024-01-31")
      let dur =
        duration.create()
        |> duration.with_months(1.0)
        |> duration.with_days(2.0)

      let ops = [
        #("addDays(base, 3)", df.add_days(base_d, 3.0), "yyyy-MM-dd"),
        #("subDays(base, 15)", df.sub_days(base_d, 15.0), "yyyy-MM-dd"),
        // add_weeks, add_months(jan31, 1.0), sub_months, add_years: same shape
        #("startOfWeek(base)", df.start_of_week(base_d), "yyyy-MM-dd"),
        #("endOfMonth(base)", df.end_of_month(base_d), "yyyy-MM-dd HH:mm:ss"),
        #("add(base, { months: 1, days: 2 })", df.add(base_d, dur), "yyyy-MM-dd"),
        #("base (immutable check)", base_d, "yyyy-MM-dd"),
      ]

      list.each(ops, fn(item) {
        let #(name, res, fmt) = item
        // ... <tr> and the name cell, then:
        node.set_text_content(td_res, df.format_date(res, fmt))
        node.set_text_content(td_dd, fmt_signed(df.difference_in_calendar_days(res, base_d)))
        node.set_text_content(td_dh, fmt_signed(df.difference_in_hours(res, base_d)))
        // ...
        Nil
      })
```
Needs: 2 (`Date`, the first declared member of `DateArg`, takes the plain name: `add_days(Date, Float)`, `format_date(Date, String)`, and `difference_in_hours(Date, Date)` for both parameters; `add_days_string`, `add_days_string_with` and `difference_in_hours_of` (for any call where either parameter is not a `Date`, since both share the union) exist but are not needed here), 5 (plain names leave `options` out), 7 (`DateType` erased, so `parse_iso` returns `Date` with no phantom), 9 (`DateArg<Date> & {}` is `DateArg`, so one `date_arg` module, not also `DateOrStringOrNumber`). The local aliases `day`/`arg` and `import date_fns/types` go.

#### F#
Today
```fsharp
let baseDate: JS.Date = parseISO "2024-03-15"
let jan31: JS.Date = parseISO "2024-01-31"
let dur = Duration.Create (months = 1.0, days = 2.0)

let ops: (string * JS.Date * string) list =
    [ "addDays(base, 3)", addDays (baseDate, 3.0), "yyyy-MM-dd"
      // ...
      "add(base, { months: 1, days: 2 })", add (baseDate, dur), "yyyy-MM-dd"
      "base (immutable check)", baseDate, "yyyy-MM-dd" ]

let bodyRows =
    ops
    |> List.map (fun (opName, res, fmt) ->
        let dd = int (differenceInCalendarDays (res, baseDate))
        let dh = int (differenceInHours (res, baseDate))
```
Target
```fsharp
let baseDate = parseISO "2024-03-15"
let jan31 = parseISO "2024-01-31"
let dur = Duration.Create (months = 1.0, days = 2.0)

let ops =
    [ "addDays(base, 3)", addDays (baseDate, 3.0), "yyyy-MM-dd"
      "subDays(base, 15)", subDays (baseDate, 15.0), "yyyy-MM-dd"
      // addWeeks, addMonths (jan31, 1.0), subMonths, addYears: same shape
      "startOfWeek(base)", startOfWeek baseDate, "yyyy-MM-dd"
      "endOfMonth(base)", endOfMonth baseDate, "yyyy-MM-dd HH:mm:ss"
      "add(base, { months: 1, days: 2 })", add (baseDate, dur), "yyyy-MM-dd"
      "base (immutable check)", baseDate, "yyyy-MM-dd" ]

let bodyRows =
    ops
    |> List.map (fun (opName, res, fmt) ->
        let dd = int (differenceInCalendarDays (res, baseDate))
        let dh = int (differenceInHours (res, baseDate))
        wrap "tr" "" [ wrap "td" "" [ el "code" "" opName ]
                       el "td" "" (formatDate (res, fmt)); el "td" "" (signed dd); el "td" "" (signed dh) ])
```
Needs: 2 (one flat `DateArg` with an `op_Implicit` from `JS.Date`, `float` and `string`, so one method per function in place of today's per-member overloads: 3 for `addDays`, 9 for `differenceInHours`), 5 (F# optional `options`), 7 (`DateType`/`ResultDate` erased to `Date`, so `parseISO` returns `JS.Date` and the `: JS.Date` and `(string * JS.Date * string) list` annotations go).

**Notes**
- None: HEADER covers everything in this demo.

### marked/custom_renderer
**Exercises:** conventions 1, 4, 5, 6, 7, 8, 9, 11 (extension with a nested renderer object; `walkTokens` reads `token.type`, a field every `Token` has; `marked(md)`/`parse(md)` without options; `new Marked(...extensions)`; `ParserOutput`/`RendererOutput` resolved to `string`; closures in place, `walkTokens` stored; library name `RendererObject`; `string | false` from `heading`, the callable property `parse`, the `async` overloads)

Section: build the extension (heading renderer + token walker), create the `Marked` instance, render with the default and the custom instance.

**TypeScript:**
```ts
const md = "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n";
const visited: string[] = [];

const custom = new Marked({
  renderer: {
    heading(h) {
      return `<h${h.depth} class="wire-h">§ ${h.text}</h${h.depth}>\n`;
    },
  },
  walkTokens(token) {
    visited.push(token.type);
  },
});

const defHtml = marked(md, { async: false });
const custHtml = custom.parse(md, { async: false });
// badge `${visited.length} tokens visited`; defOut.innerHTML = defHtml;
// custOut.innerHTML = custHtml; one .tok-pill <span> per entry of visited
```

#### C
Today
```c
static JsValue walk_token_cb(void* ctx, JsValue token) {
// ...
        JsString t = Marked_Tokens_Generic_get_type((Marked_Tokens_Generic){ token.h });
// ...
    return (JsValue){ 0 };
// ...
static JsValue render_heading(void* ctx, Marked_Tokens_Heading h) {
// ...
    MarkedExtension ext = MarkedExtension_create(
        .renderer = MarkedExtension_renderer_create(.heading = MarkedExtension_renderer_heading_from(render_heading, 0)),
        .walkTokens = MarkedExtension_walkTokens_from(walk_token_cb, 0)
    );
    Marked custom_m = Marked_new(JsArray_MarkedExtension_from(&ext, 1));
    JsString def_html = marked(md, (marked_options_no_async){ 0 });
    JsString custom_html = Marked_parse_invoke(Marked_get_parse(custom_m), md, (marked_options_no_async){ 0 });
```
Target
```c
static void walk_token(void* ctx, Marked_Token t) {
    (void)ctx;
    if (walked_count < 64) {
        JsString s = Marked_Token_get_type(t);
        JsString_copy(s, walked_types[walked_count], sizeof walked_types[0]);
        js_release(s);
        walked_count++;
    }
}

static const char* render_heading(void* ctx, Marked_Tokens_Heading h) {
    (void)ctx;
    js_scope();
    uint32_t depth = (uint32_t)Marked_Tokens_Heading_get_depth(h);
    if (depth < 1 || depth > 6) depth = 1;
    char text_buf[128];
    JsString_copy(Marked_Tokens_Heading_get_text(h), text_buf, sizeof text_buf);
    static char out[256];   /* copied into JS when the callback returns; NULL gives `false` */
    snprintf(out, sizeof out, "<h%u class=\"wire-h\">\xC2\xA7 %s</h%u>\n", depth, text_buf, depth);
    return out;
}

    MarkedExtension ext = MarkedExtension_create(
        .renderer = Marked_RendererObject_create(.heading = Marked_RendererHeadingFn_from(render_heading, 0)),
        .walkTokens = Marked_WalkTokensFn_from(walk_token, 0)
    );
    Marked custom_m = Marked_new(ext);
    JsString def_html = marked(md);
    JsString custom_html = Marked_parse(custom_m, md);
```
Needs: 1 (nested `Marked_RendererObject_create`), 4 (the walker gets a typed `Marked_Token` and reads `Marked_Token_get_type(t)`, no cast to `Tokens_Generic`), 5 (`marked(md)`, `Marked_parse(m, md)` leave options out), 6 (`Marked_new(ext)`, one argument per value; `Marked_new_list` for a `JsArray`), 8 (fn pointer + ctx; the walker returns `void`), 9 (`RendererObject`; callback types `Marked_RendererHeadingFn`, `Marked_WalkTokensFn` (name TBD)), 11 (`string | false` is a `const char*`, `NULL` for `false`, so no `js_retain`; `parse` is a method; plain `marked` is the synchronous overload).

#### C++
Today
```cpp
    using Mk = marked::Marked<std::string, std::string>;
    auto rend = marked::MarkedExtension_renderer<std::string, std::string>({.heading = [](ts::Tuple<marked::Tokens_Heading> args) {
        auto h = args.get<0>();
        auto d = std::to_string(int(h.depth()));
        return std::string("<h") + d + " class=\"wire-h\">\xC2\xA7 " + h.text() + "</h" + d + ">\n";
    }});
    ts::Scoped<marked::MarkedExtension_walkTokens> on_token = [&](const marked::Token& token) {
        std::visit([&](const auto& x) {
            // ...
        }, token);
    };
    auto ext = marked::MarkedExtension<std::string, std::string>({.renderer = rend, .walkTokens = on_token});
    Mk custom_m = Mk(ext);
    std::string def_html = marked::marked(md, marked::marked_options_no_async::create());
    std::string cust_html = std::get<std::string>(custom_m.parse()(md));
```
Target
```cpp
    auto visited = std::make_shared<std::vector<std::string>>();

    auto ext = marked::MarkedExtension({
        .renderer = {{.heading = [](const marked::Tokens_Heading& h) {
            auto d = std::to_string(int(h.depth()));
            return "<h" + d + " class=\"wire-h\">\xC2\xA7 " + h.text() + "</h" + d + ">\n";
        }}},
        .walkTokens = [visited](const marked::Token& token) { visited->push_back(token.type()); },
    });
    marked::Marked custom_m(ext);
    std::string def_html = marked::marked(md);
    std::string cust_html = custom_m.parse(md);
    // ... badge from visited->size(), one pill per entry of *visited
```
Needs: 1 (nested `.renderer = {{..}}` through `ts::Field<T>` taking `T::Fields`; `.renderer` before `.walkTokens` in ASCII order), 4 (`Token` is a union class and `token.type()` reads the shared field, no `std::visit`), 5 (`marked(md)`, `parse(md)` overloads), 6 (variadic `Marked(ext)`), 7 (`ParserOutput`/`RendererOutput` resolved to `std::string`: no `<std::string, std::string>`, no `Mk` alias; `Parameters<..>` evaluated, so the heading lambda takes `const Tokens_Heading&`, not a `ts::Tuple`), 8 (lambdas in place, no `ts::Scoped`; the stored walker captures by value, so `visited` becomes a `shared_ptr`, as Rust's `seen` is an `Rc`), 9 (`RendererObject`), 11 (the heading result is `std::optional<std::string>` and the lambda's `std::string` converts; `parse` is a method; `marked`/`parse` are the synchronous overloads and return `std::string`, so no `std::get`).

#### Rust
Today
```rust
    let heading = MarkedExtensionRendererHeading::new(|h: tokens::Heading| {
        BooleanOrRendererOutput::RendererOutput(format!(
// ...
    let renderer = MarkedExtensionRenderer::<String, String>::default().with_heading(&heading);
    let walk = MarkedExtensionWalkTokens::new({
        let seen = seen.clone();
        move |t: Token| {
            seen.borrow_mut().push(token_type(&t));
            None
// ...
    let custom_m = Marked::new(vec![ext]);
    let def_html = match marked::marked_with_opt_marked_options(md) {
        StringOrPromise::String(s) => s,
// ...
    let cust_html = match custom_m.parse().invoke_2(md) {
```
Target
```rust
use marked::{Marked, MarkedExtension, RendererObject};
use std::cell::RefCell;
use std::rc::Rc;

    let seen = Rc::new(RefCell::new(Vec::<String>::new()));

    let ext = MarkedExtension::default()
        .with_renderer(&RendererObject::default().with_heading(|h| {
            Some(format!("<h{0} class=\"wire-h\">\u{00A7} {1}</h{0}>\n", h.depth() as u32, h.text()))
        }))
        .with_walk_tokens({
            let seen = seen.clone();
            move |t| seen.borrow_mut().push(t.type_())
        });

    let custom_m = Marked::new([&ext]);
    let def_html = marked::marked(md);
    let cust_html = custom_m.parse(md);
    let visited = seen.borrow();
```
Needs: 1 (builders), 4 (`t.type_()` in place of the 23-arm `token_type`), 5 (`marked(md)`, `parse(md)` leave options out), 6 (`Marked::new([&ext])`, a borrowed list), 7 (no `::<String, String>`), 8 (`impl FnMut(..) + 'static` straight to `with_heading`/`with_walk_tokens`, argument types inferred, no `...::new` wrappers; `walkTokens` is stored, so `seen` stays `Rc<RefCell<..>>`; the walker returns `()`), 9 (`RendererObject`, not `MarkedExtensionRenderer`), 11 (`Option<String>` heading result; `parse` is a method; `marked`/`parse` are synchronous and return `String`, no `match` on `StringOrPromise`).

#### Gleam
Today
```gleam
    mt.TokenTokensHeading(_) -> "heading"
// ...
        |> marked_extension_renderer.with_heading(
          fn(heading) {
            let tok = mr.dynamic(heading)
            let d = int.to_string(float.round(field_float(tok, "depth")))
            mt.BooleanOrRendererOutputRendererOutput(
// ...
          mr.codec_string(),
        )
// ...
      let call = marked_class.parse_call_options_no_async_create()
      let def_html = marked.marked(md, marked_options_no_async.create())
      let cust_html =
        marked_class.parse_invoke(
```
Target
```gleam
import gleam/option.{None, Some}
import marked
import marked/marked as marked_class
import marked/marked_extension
import marked/namespaces/tokens/heading
import marked/renderer_object
import marked/token

      let renderer =
        renderer_object.create()
        |> renderer_object.with_heading(fn(h) {
          let d = int.to_string(float.round(heading.get_depth(h)))
          Some("<h" <> d <> " class=\"wire-h\">§ " <> heading.get_text(h) <> "</h" <> d <> ">\n")
        })

      let ext =
        marked_extension.create()
        |> marked_extension.with_renderer(renderer)
        |> marked_extension.with_walk_tokens(fn(t) {
          let pill = document.create_element(doc, tag.span)
          element.set_class_name(pill, "tok-pill")
          node.set_text_content(pill, token.get_type(t))
          node.append_child(strip, pill)
          Nil
        })

      let custom_m = marked_class.new([ext])
      let def_html = marked.marked(md)
      let cust_html = marked_class.parse(custom_m, md)
```
Needs: 1 (pipelines), 4 (`token.get_type(t)` in place of the 23-arm `token_type`), 5 (`marked(md)`, `parse(m, md)` leave options out), 6 (`marked_class.new([ext])`, a list), 7 (no `codec_string()` arguments, no phantom parameters; `Parameters<..>` evaluated, so the heading callback gets a typed `TokensHeading` and the `mr.dynamic` + `decode` bypass goes), 8 (plain functions; the walker returns `Nil`), 9 (`renderer_object`, `token`), 11 (`Option(String)` heading result; `parse` is a method, no `get_parse`/`parse_invoke`; plain `marked`/`parse` are synchronous and return `String`).

#### F#
Today
```fsharp
let ext =
    MarkedExtension.Create (
        renderer =
            MarkedExtensionRenderer.Create (
                heading =
                    (fun (h: Tokens.Heading) ->
                        !^ $"<h{int h.depth} class=\"wire-h\">§ {h.text}</h{int h.depth}>\n")
// ...
let customInstance = Marked.Create ext
let customTokens = customInstance.lexer md
// ...
customInstance.walkTokens (
    !^customTokens,
// ...
let customHtml = customInstance.parser (ResizeArray customTokens)
```
Target (sets `walkTokens` on the extension, like the other languages, in place of today's hand-run `lexer`/`walkTokens`/`parser`; the same tokens are visited in the same order and the HTML is the same)
```fsharp
let mutable seen: string list = []

let customInstance =
    Marked.Create (
        MarkedExtension.Create (
            renderer =
                RendererObject.Create (
                    heading = fun h -> Some $"<h{int h.depth} class=\"wire-h\">§ {h.text}</h{int h.depth}>\n"),
            walkTokens = fun t -> seen <- t.``type`` :: seen))

let defaultHtml = marked md
let customHtml = customInstance.parse md
let visited = List.rev seen
```
Needs: 1 (`Create` with named arguments, nested), 4 (``t.``type`` `` in place of the 23-case `tokenType`), 5 (F# optional arguments), 6 (`Marked.Create` takes the extensions as a `ParamArray`), 7 (`Parameters<..>` evaluated, so `h` is inferred as `Tokens.Heading` with no annotation), 8 (`walkTokens` returns `unit`, no `!^()`), 9 (`RendererObject`), 11 (`Some ..` for the `string | false` result, no `!^`; `parse` is a method; `marked md` and `parse md` are synchronous and return `string`, so `unwrapHtml` goes).

**Notes**
- In Gleam, `import marked` (the `marked()` function) and `import marked/marked` (the `Marked` class) collide, so an alias is still needed (`marked_class`).
- C++ `Token` as a union class keeps `std::visit` and `std::holds_alternative` if it derives from `std::variant` (checked with `clang++ -std=c++20`); convention 4 gives C++ no matching form, and this demo needs none.

---

# Target sketches: node/event_bus, prosemirror/rich_schema_marks

### node/event_bus

**Exercises:** conventions 2, 3, 5, 6, 7, 8, 9, 11

**TypeScript:**
```ts
const emitter = new EventEmitter();
emitter.setMaxListeners(12);
let metricCalls = 0, metricSum = 0, onceCalls = 0, shutdownCalls = 0;
const onMetric = (...args: any[]) => {
  metricCalls++;
  if (args.length > 0) metricSum += args[0];
};
emitter.on("metric", onMetric);
emitter.once("metric", () => { onceCalls++; });
emitter.on("shutdown", () => { shutdownCalls++; });
check(emitter.listenerCount("metric") === 2 && emitter.eventNames().length === 2);
check(emitter.emit("metric", 101, "ok") && metricCalls === 1 && onceCalls === 1);
check(emitter.emit("metric", 202, "ok") && metricSum === 303 && onceCalls === 1);
check(!emitter.emit("idle"));
emitter.removeListener("metric", onMetric);
check(emitter.listenerCount("metric") === 0 && !emitter.emit("metric", 1));
emitter.removeAllListeners("shutdown");
check(emitter.eventNames().length === 0 && shutdownCalls === 0);
```

#### C

Today
```c
    Node_EventEmitter emitter = Node_EventEmitter_new();
    Node_any_array_fn metric_cb = Node_any_array_fn_from(on_metric, 0);
    Node_any_array_fn once_cb = Node_any_array_fn_from(on_once, 0);
    Node_any_array_fn shutdown_cb = Node_any_array_fn_from(on_shutdown, 0);
    NodeJS_EventEmitter_on(emitter, "metric", metric_cb);
    NodeJS_EventEmitter_once(emitter, "metric", once_cb);
    NodeJS_EventEmitter_on(emitter, "shutdown", shutdown_cb);
    JsArray args1 = JsArray_new();
    js_push_num(args1, 101.0);
    js_push_str(args1, "ok");
    check(Node_EventEmitter_emit(emitter, "metric", args1) && g_metric_calls == 1 && g_once_calls == 1);
    JsArray empty_args = JsArray_new();
    check(!Node_EventEmitter_emit(emitter, "idle", empty_args));
    Node_EventEmitter_removeListener(emitter, "metric", metric_cb);
    Node_EventEmitter_removeAllListeners(emitter, "shutdown");
```

Target
```c
/* on_metric, on_once, on_shutdown: unchanged `static void f(void* ctx, JsArray args)` listeners */

    Node_EventEmitter emitter = Node_EventEmitter_new();
    Node_EventEmitter_setMaxListeners(emitter, 12.0);
    check(Node_EventEmitter_getMaxListeners(emitter) == 12.0);

    /* kept in a variable: removeListener needs the same function */
    Node_Listener metric_cb = Node_Listener_from(on_metric, NULL);
    Node_EventEmitter_on(emitter, "metric", metric_cb);
    Node_EventEmitter_once(emitter, "metric", Node_Listener_from(on_once, NULL));
    Node_EventEmitter_on(emitter, "shutdown", Node_Listener_from(on_shutdown, NULL));

    check(Node_EventEmitter_listenerCount(emitter, "metric") == 2.0);
    check(JsArray_length(Node_EventEmitter_eventNames(emitter)) == 2);

    check(Node_EventEmitter_emit(emitter, "metric", 101.0, "ok") && g_metric_calls == 1 && g_once_calls == 1);
    check(Node_EventEmitter_listenerCount(emitter, "metric") == 1.0);
    check(Node_EventEmitter_emit(emitter, "metric", 202.0, "ok") && g_metric_calls == 2 && g_metric_sum == 303.0 && g_once_calls == 1);
    check(!Node_EventEmitter_emit(emitter, "idle"));

    Node_EventEmitter_removeListener(emitter, "metric", metric_cb);
    check(Node_EventEmitter_listenerCount(emitter, "metric") == 0.0 && !Node_EventEmitter_emit(emitter, "metric"));
    check(g_metric_calls == 2);

    Node_EventEmitter_removeAllListeners(emitter, "shutdown");
    check(JsArray_length(Node_EventEmitter_eventNames(emitter)) == 0 && g_shutdown_calls == 0);
```
Needs: 2 (`"metric"` into `string | symbol` through `_Generic`), 5 (`listenerCount` leaves out its trailing listener), 6 (`emit(e, "metric", 101.0, "ok")`, one `_Generic` value per argument; `emit(e, "idle")`), 7 (rule 3 makes `T` the union of the library's event maps and rule 2 makes `E` `string | symbol`, so event names are `string | symbol` and arguments `any[]`), 8 (fn + ctx), 9 (`Node_Listener` from the library's `Listener`; one `Node_EventEmitter_` prefix, not `NodeJS_` and `Node_`).

#### C++

Today
```cpp
    auto emitter = EventEmitter<ts::Value>::create();
    auto ev_metric = ts::Value::string("metric");
    auto ev_shutdown = ts::Value::string("shutdown");
    auto ev_idle = ts::Value::string("idle");
    ts::Scoped<any_array_fn> on_metric = [&](ts::Array<> args) {
        metric_calls++;
        if (args.size() > 0) {
            metric_sum += args[0].to_number();
        }
    };
    ts::Scoped<any_array_fn> on_once = [&](ts::Array<>) { once_calls++; };
    ts::Scoped<any_array_fn> on_shutdown = [&](ts::Array<>) { shutdown_calls++; };
    emitter.on(ev_metric, on_metric);
    emitter.once(ev_metric, on_once);
    bool emitted1 = emitter.emit(ev_metric, 101, "ok");
```

Target
```cpp
    auto emitter = EventEmitter::create();
    emitter.setMaxListeners(12);
    check(emitter.getMaxListeners() == 12);

    // stored lambdas capture by value, so the counters are shared
    auto metric_calls = std::make_shared<int>(0);
    auto metric_sum = std::make_shared<double>(0.0);
    auto once_calls = std::make_shared<int>(0);
    auto shutdown_calls = std::make_shared<int>(0);

    // kept in a variable: removeListener needs the same function
    ts::Scoped<Listener> on_metric = [metric_calls, metric_sum](ts::Array<> args) {
        ++*metric_calls;
        if (args.size() > 0) {
            *metric_sum += args[0].to_number();
        }
    };
    emitter.on("metric", on_metric);
    emitter.once("metric", [once_calls](ts::Array<>) { ++*once_calls; });
    emitter.on("shutdown", [shutdown_calls](ts::Array<>) { ++*shutdown_calls; });

    check(emitter.listenerCount("metric") == 2);
    check(emitter.eventNames().size() == 2);

    bool emitted1 = emitter.emit("metric", 101, "ok");
    check(emitted1 && *metric_calls == 1 && *once_calls == 1);
    check(emitter.listenerCount("metric") == 1);
    bool emitted2 = emitter.emit("metric", 202, "ok");
    check(emitted2 && *metric_calls == 2 && *metric_sum == 303.0 && *once_calls == 1);
    check(!emitter.emit("idle"));

    emitter.removeListener("metric", on_metric);
    check(emitter.listenerCount("metric") == 0 && !emitter.emit("metric", 1));
    check(*metric_calls == 2);
    emitter.removeAllListeners("shutdown");
    check(emitter.eventNames().size() == 0 && *shutdown_calls == 0);
```
Needs: 2 (`"metric"` converts to `string | symbol`), 5 (overloads), 6 (variadic `emit`), 7 (rule 3: `EventEmitter`, not `EventEmitter<ts::Value>`), 8 (lambdas passed directly and freed by `FinalizationRegistry`; stored, so they capture by value; `ts::Scoped<Listener>` only for the one removed), 9 (`Listener`, not `any_array_fn`).

#### Rust

Today
```rust
    let emitter = EventEmitter::new();
    let on_metric = AnyArrayFn::new({
        let metric_calls = metric_calls.clone();
        let metric_sum = metric_sum.clone();
        move |args: JsArray| {
    // ...
    let on_once = AnyArrayFn::new({
        let once_calls = once_calls.clone();
        move |_: JsArray| once_calls.set(once_calls.get() + 1)
    });
    emitter.on("metric", &on_metric);
    emitter.once("metric", &on_once);
    let emitted1 = emitter.emit("metric", [JsValue::from(101.0), JsValue::from("ok")]);
    c.check(!emitter.emit("idle", JsArray::new()));
    emitter.remove_all_listeners_with("shutdown");
```

Target
```rust
    let emitter = EventEmitter::new();
    emitter.set_max_listeners(12.0);
    c.check(emitter.get_max_listeners() == 12.0);

    let metric_calls = Rc::new(Cell::new(0u32));
    let metric_sum = Rc::new(RefCell::new(0.0f64));
    let once_calls = Rc::new(Cell::new(0u32));
    let shutdown_calls = Rc::new(Cell::new(0u32));

    // kept in a variable: remove_listener needs the same function
    let on_metric = Listener::new({
        let metric_calls = metric_calls.clone();
        let metric_sum = metric_sum.clone();
        move |args| {
            metric_calls.set(metric_calls.get() + 1);
            if !args.is_empty() {
                *metric_sum.borrow_mut() += args.get(0).as_f64();
            }
        }
    });
    emitter.on_listener("metric", &on_metric);
    emitter.once("metric", {
        let once_calls = once_calls.clone();
        move |_| once_calls.set(once_calls.get() + 1)
    });
    emitter.on("shutdown", {
        let shutdown_calls = shutdown_calls.clone();
        move |_| shutdown_calls.set(shutdown_calls.get() + 1)
    });

    c.check(emitter.listener_count("metric") == 2.0);
    c.check(emitter.event_names().len() == 2);

    let emitted1 = emitter.emit_with("metric", [101.0.into(), "ok".into()]);
    c.check(emitted1 && metric_calls.get() == 1 && once_calls.get() == 1);
    c.check(emitter.listener_count("metric") == 1.0);
    let emitted2 = emitter.emit_with("metric", [202.0.into(), "ok".into()]);
    c.check(emitted2 && metric_calls.get() == 2 && *metric_sum.borrow() == 303.0 && once_calls.get() == 1);
    c.check(!emitter.emit("idle"));

    emitter.remove_listener("metric", &on_metric);
    c.check(emitter.listener_count("metric") == 0.0 && !emitter.emit_with("metric", [1.0.into()]));
    c.check(metric_calls.get() == 2);

    emitter.remove_all_listeners("shutdown");
    c.check(emitter.event_names().is_empty() && shutdown_calls.get() == 0);
```
Needs: 2 (`impl Into<..>` for the event name), 3 (`[101.0.into(), "ok".into()]`; the list's element type is `JsValue`, so `.into()` infers, and a one-kind list is `[1.0.into()]`), 5 (`remove_all_listeners("shutdown")`, a bare `impl Optional`; `listener_count` leaves out its trailing callback), 6 (`emit("idle")` / `emit_with("metric", [..])`), 7 (rule 3: `EventEmitter`, no `<JsValue>`), 8 (`on` / `once` take closures with inferred arguments, freed by `FinalizationRegistry`; `on_listener` and `remove_listener` take the kept `&Listener`), 9 (`Listener`, not `AnyArrayFn`).

#### Gleam

Today
```gleam
import node/areas/events/event_emitter as events
import node/namespaces/node_js/event_emitter as emitter
  let bus =
    events.new()
    |> emitter.set_max_listeners(12.0)
  let ev_metric = runtime.dynamic("metric")
  let ev_shutdown = runtime.dynamic("shutdown")
  emitter.on(bus, ev_metric, on_metric)
  emitter.once(bus, ev_metric, on_once)
  let emitted1 =
    emitter.emit(bus, ev_metric, [
      runtime.dynamic(101.0),
      runtime.dynamic("ok"),
    ])
  emitter.remove_all_listeners_with(bus, Some(ev_shutdown))
```

Target
```gleam
import gleam/dynamic
import gleam/dynamic/decode
import gleam/list
import node/events/event_emitter as emitter
import node/runtime

  let bus =
    emitter.new()
    |> emitter.set_max_listeners(12.0)
  let passed = check(emitter.get_max_listeners(bus) == 12.0, "maxListeners")

  let metric_calls = runtime.ref_new(0)
  let metric_sum = runtime.ref_new(0.0)
  let once_calls = runtime.ref_new(0)
  let shutdown_calls = runtime.ref_new(0)

  // kept in a variable: remove_listener needs the same function
  let on_metric = fn(args) {
    runtime.ref_update(metric_calls, fn(n) { n + 1 })
    case args {
      [first, ..] ->
        case decode.run(first, decode.float) {
          Ok(n) -> {
            runtime.ref_update(metric_sum, fn(s) { s +. n })
            Nil
          }
          Error(_) -> Nil
        }
      [] -> Nil
    }
  }
  emitter.on(bus, "metric", on_metric)
  emitter.once(bus, "metric", fn(_) {
    runtime.ref_update(once_calls, fn(n) { n + 1 })
    Nil
  })
  emitter.on(bus, "shutdown", fn(_) {
    runtime.ref_update(shutdown_calls, fn(n) { n + 1 })
    Nil
  })

  let passed =
    passed
    + check(emitter.listener_count(bus, "metric") == 2.0, "initial listenerCount")
    + check(list.length(emitter.event_names(bus)) == 2, "eventNames")

  let emitted1 =
    emitter.emit_with(bus, "metric", [dynamic.float(101.0), dynamic.string("ok")])
  // ... checks as today
  let emitted2 =
    emitter.emit_with(bus, "metric", [dynamic.float(202.0), dynamic.string("ok")])
  // ... checks as today, then:
  let passed = passed + check(!emitter.emit(bus, "idle"), "idle emit false")

  emitter.remove_listener(bus, "metric", on_metric)
  let silent = !emitter.emit_with(bus, "metric", [dynamic.float(1.0)])
  // ... checks as today

  emitter.remove_all_listeners_with(bus, "shutdown")
  let gone =
    emitter.event_names(bus) == [] && runtime.ref_get(shutdown_calls) == 0
```
Needs: 2 (`string | symbol` takes the string plainly, `emit(bus, "idle")`), 3 (`[dynamic.float(..), dynamic.string(..)]`: `any` is `Dynamic`, built with gleam/dynamic's own constructors, no `runtime.dynamic`), 5 (`remove_all_listeners_with(bus, "shutdown")`, bare), 6 (`emit` / `emit_with`), 7 (rule 3: no `EventEmitter(t, Nil)` phantoms), 8 (plain functions), 9 (one `event_emitter` module instead of `areas/events` + `namespaces/node_js`).

#### F#

Today
```fsharp
let emitter = Events.EventEmitter.Create ()
emitter.setMaxListeners 12.0 |> ignore
let onMetric =
    NodeJS.AddListenerListener.Create(fun args ->
        metricCalls <- metricCalls + 1
        if args.Count > 0 then
            metricSum <- metricSum + unbox<float> args.[0])
let onOnce =
    NodeJS.AddListenerListener.Create(fun _ -> onceCalls <- onceCalls + 1)
emitter.on ("metric", onMetric) |> ignore
emitter.once ("metric", onOnce) |> ignore
let emitted1 = emitter.emit ("metric", box 101.0, box "ok")
check (not (emitter.emit "idle")) "idle emit false"
emitter.removeListener ("metric", onMetric) |> ignore
emitter.removeAllListeners "shutdown" |> ignore
```

Target
```fsharp
let emitter = Events.EventEmitter.Create ()
emitter.setMaxListeners 12.0 |> ignore
check (emitter.getMaxListeners () = 12.0) "maxListeners"

let mutable metricCalls = 0
let mutable metricSum = 0.0
let mutable onceCalls = 0
let mutable shutdownCalls = 0

// kept in a variable: removeListener needs the same function
let onMetric =
    Listener (fun args ->
        metricCalls <- metricCalls + 1
        if args.Length > 0 then
            metricSum <- metricSum + unbox<float> args.[0])

emitter.on ("metric", onMetric) |> ignore
emitter.once ("metric", fun _ -> onceCalls <- onceCalls + 1) |> ignore
emitter.on ("shutdown", fun _ -> shutdownCalls <- shutdownCalls + 1) |> ignore

check (emitter.listenerCount "metric" = 2.0) "initial listenerCount"
check (emitter.eventNames().Count = 2) "eventNames"

let emitted1 = emitter.emit ("metric", 101.0, "ok")
check (emitted1 && metricCalls = 1 && onceCalls = 1) "first metric emit"
check (emitter.listenerCount "metric" = 1.0) "once removes itself"
let emitted2 = emitter.emit ("metric", 202.0, "ok")
check (emitted2 && metricCalls = 2 && metricSum = 303.0 && onceCalls = 1) "second metric emit"
check (not (emitter.emit "idle")) "idle emit false"

emitter.removeListener ("metric", onMetric) |> ignore
check (emitter.listenerCount "metric" = 0.0 && not (emitter.emit ("metric", 1.0))) "removeListener"
check (metricCalls = 2) "metric count unchanged after remove"
emitter.removeAllListeners "shutdown" |> ignore
check (emitter.eventNames().Count = 0 && shutdownCalls = 0) "removeAllListeners"
```
Needs: 2 (`"metric"` converts at the method argument), 5 (optional arguments), 6 (`ParamArray`: `emit ("metric", 101.0, "ok")` boxes its arguments itself), 7 (rule 3: no `EventEmitter<obj>`), 8 (lambdas passed directly; a `Listener` kept for removal), 9 (`Listener`, not `NodeJS.AddListenerListener`), 11 (`|> ignore` on `this`-returning calls).

#### Notes
- Rule 3 now leaves `any` out, so `T` is the union of the event maps the library instantiates (`WorkerEventMap`, `SocketEventMap`, ..). `IfEventMap` then takes its true branch, but with `E` erased to `string | symbol` the result is the same as with `any`: event names `string | symbol`, `Listener` `(...args: any[]) => void`, arguments `any[]`. Only the evaluation order matters: `E` must be erased before the conditional types are evaluated.
- (?) F#: `Listener` is `delegate of [<ParamArray>] args: obj[] -> unit`. `dotnet fsi` accepts a lambda for it at a method argument, along with `"metric"` through `op_Implicit` and unboxed `ParamArray` arguments. I have not checked whether Fable compiles a `ParamArray` delegate to a JS rest-parameter function, which `args` needs in order to see `101, "ok"`.
- (?) The Gleam path `node/events/event_emitter`, and `prosemirror/model/..` below, drop `areas/`, `namespaces/` and the doubled `prosemirror_`. HEADER does not say how module paths are formed.

---

### prosemirror/rich_schema_marks

**Exercises:** conventions 1, 2, 3, 5, 7, 8, 9, 10

**TypeScript:**
```ts
const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    heading: { attrs: { level: { default: 2 } }, content: "inline*", group: "block", defining: true,
               toDOM: (n) => [n.attrs.level === 1 ? "h1" : "h2", 0] },
    paragraph: { content: "inline*", group: "block", toDOM: () => ["p", 0] },
    code_block: { content: "text*", marks: "", group: "block", code: true, defining: true,
                  toDOM: () => ["pre", ["code", 0]] },
    // blockquote, horizontal_rule: same shape
    text: { group: "inline" },
  },
  marks: { strong: { toDOM: () => ["strong", 0] }, em: { toDOM: () => ["em", 0] }, code: { toDOM: () => ["code", 0] } },
});
const h = schema.node("heading", { level: 2 }, schema.text("ProseMirror Document Schema"));
const p = schema.node("paragraph", null, [
  schema.text("Inline marks compose cleanly: "), schema.text("bold", [schema.mark("strong")]), /* ... */
]);
const doc = schema.node("doc", null, [h, p, quote, codeBlock, outro]);
const view = new EditorView(document.getElementById("editor"),
  { state: EditorState.create({ schema, doc }), dispatchTransaction: onDispatch });
```

#### C

Today
```c
    Pm_MarkSpec_attrs_set_at(hattrs, "level", lvl);
    Pm_NodeSpec heading = Pm_NodeSpec_create(.attrs = hattrs, .content = "inline*", .group = "block", .defining = true, .toDOM = Pm_Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));
    // ...
    Pm_Attrs nodes = Pm_Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    // ... one js_set_val per node type, then the marks the same way
    return Pm_Schema_new(Pm_SchemaSpec_create(.nodes = js_value(nodes), .marks = js_value(marks)));
    // ...
        Pm_Schema_text(g_schema, "bold", JsArray_Pm_Mark_from(&m_strong, 1)),
    // ...
    Pm_Node blocks[5] = { h, p_marks, quote, code_block, outro };
    return Pm_Schema_node(g_schema, "doc", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(blocks, 5));
    // ...
    g_view = Pm_EditorView_new(AS_Node(mount), props);
```

Target
```c
/* heading_dom, tag_dom, mark_dom, on_dispatch: unchanged fn + ctx callbacks */
static JsValue dom_hole(const char* tag) { return js_value(js_values(tag, 0)); }
static JsValue code_block_dom(void* ctx, Pm_Node n) {
    (void)ctx; (void)n;
    return js_value(js_values("pre", js_values("code", 0)));
}
/* hr_dom: js_value(js_values("hr")) */

static Pm_Schema make_schema(void) {
    Pm_NodeSpec doc = Pm_NodeSpec_create(.content = "block+");
    Pm_NodeSpec heading = Pm_NodeSpec_create(
        .attrs = Pm_AttributeSpec_record({ "level", Pm_AttributeSpec_create(.default_ = js_number(2)) }),
        .content = "inline*", .group = "block", .defining = true,
        .toDOM = Pm_NodeSpecToDOMFn_from(heading_dom, NULL));                 /* (name TBD) */
    Pm_NodeSpec cblock = Pm_NodeSpec_create(.content = "text*", .marks = "", .group = "block", .code = true,
        .defining = true, .toDOM = Pm_NodeSpecToDOMFn_from(code_block_dom, NULL));
    /* para, quote, hr, text and strong/em/code: as today, with Pm_NodeSpecToDOMFn_from / Pm_MarkSpecToDOMFn_from */

    return Pm_Schema_new(Pm_SchemaSpec_create(
        .nodes = Pm_NodeSpec_record({ "doc", doc }, { "heading", heading }, { "paragraph", para },
                     { "blockquote", quote }, { "code_block", cblock }, { "horizontal_rule", hr }, { "text", text }),
        .marks = Pm_MarkSpec_record({ "strong", strong }, { "em", em }, { "code", code })));
}

static Pm_Node make_initial_doc(void) {
    Pm_Node h = Pm_Schema_node(g_schema, "heading", Pm_Attrs_record({ "level", js_number(2) }),
        t_node("ProseMirror Document Schema"));
    Pm_Mark m_strong = Pm_Schema_mark(g_schema, "strong");
    /* m_em, m_code likewise */
    Pm_Node p_marks = Pm_Schema_node(g_schema, "paragraph", NULL, Pm_Node_list(
        t_node("Inline marks compose cleanly: "),
        Pm_Schema_text(g_schema, "bold", Pm_Mark_list(m_strong)),
        /* ... */));
    /* q_inner, quote, code_block, outro: as today, NULL for attrs */
    return Pm_Schema_node(g_schema, "doc", NULL, Pm_Node_list(h, p_marks, quote, code_block, outro));
}

    /* start() */
    Pm_DirectEditorProps props = Pm_DirectEditorProps_create(.state = make_state(),
        .dispatchTransaction = Pm_DispatchTransactionFn_from(on_dispatch, NULL));
    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(mount, props);
```
Needs: 1 (`_create` designators, a nested `Pm_AttributeSpec_create`), 2 (`.nodes` / `.marks` take the record, the primary member by rule 3; `"heading"`, `NULL` and `mount` go straight to `_Generic` arguments, `mount` within the 40-subtype limit for DOM `Node`), 3 (`Pm_Node_list(..)`, `Pm_Mark_list(..)`, `js_values(..)`), 5 (trailing `marks` / `attrs` left out), 7 (rule 2: `Nodes extends string` erases to `string`, so `nodes` is a string-keyed record), 8 (fn + ctx), 9 (`Pm_NodeSpecToDOMFn`, `Pm_DispatchTransactionFn`; no `Pm_MarkSpec_attrs`, `Pm_Node_to_DOMOutputSpec_fn`, `AS_Node`), 10 (`Pm_NodeSpec_record`, `Pm_MarkSpec_record`, `Pm_AttributeSpec_record`, `Pm_Attrs_record` (name TBD)).

#### C++

Today
```cpp
using Sch = Schema<ts::Value, ts::Value>;
    MarkSpec_attrs hattrs = MarkSpec_attrs::create().set_at("level", AttributeSpec::create().default_(ts::Value::number(2)));
    // ...
        .toDOM = [](Node) { return DOMOutputSpec(ts::Array<>::of("pre", ts::Array<>::of("code", 0))); },
    // ...
    auto nodes = NodeSpec::create()
        .set_at("doc", doc)
    // ...
    return Sch::create({{.nodes = nodes, .marks = marks}});
    // ...
        schema.text("bold", ts::Array<Mark>{ schema.mark("strong") }),
    // ...
    Node p1 = schema.node("paragraph", std::nullopt, p1_runs);
    // ...
    app->view = EditorView(app->doc.getElementById("editor")->unsafe_as<Node_2>(), props);
```

Target
```cpp
static DOMOutputSpec hole_spec(std::string_view tag) { return {tag, 0}; } // (?) see Notes

static Schema make_schema() {
    NodeSpec doc({.content = "block+"});
    NodeSpec heading({
        .attrs = {{"level", {{.default_ = 2}}}},
        .content = "inline*",
        .defining = true,
        .group = "block",
        .toDOM = [](Node n) {
            int l = static_cast<int>(n.attrs().at("level").to_number());
            return hole_spec(l == 1 ? "h1" : "h2");
        },
    });
    NodeSpec cblock({
        .code = true,
        .content = "text*",
        .defining = true,
        .group = "block",
        .marks = "",
        .toDOM = [](Node) -> DOMOutputSpec { return {"pre", ts::Array<>{"code", 0}}; },
    });
    // para, quote, hr, text, make_mark: as today, designators in ASCII order

    return Schema::create({{
        .marks = {{"strong", make_mark("strong")}, {"em", make_mark("em")}, {"code", make_mark("code")}},
        .nodes = {{"doc", doc}, {"heading", heading}, {"paragraph", para}, {"blockquote", quote},
                  {"code_block", cblock}, {"horizontal_rule", hr}, {"text", text}},
    }});
}

static Node make_initial_doc(const Schema& schema) {
    Node h = schema.node("heading", {{"level", 2}}, schema.text("ProseMirror Document Schema"));
    Node p1 = schema.node("paragraph", nullptr, {
        schema.text("Inline marks compose cleanly: "),
        schema.text("bold", {schema.mark("strong")}),
        // ...
    });
    // q_inner, quote, cblock, outro: as today, nullptr for attrs
    return schema.node("doc", nullptr, {h, p1, quote, cblock, outro});
}

    // start()
    DirectEditorProps props({
        .dispatchTransaction = [app](Transaction tr) { /* as today */ },
        .state = make_state(app->schema),
    });
    app->view = EditorView(*app->doc.getElementById("editor"), props);
```
Needs: 1 (`Type({..})`, nested `{{.default_ = 2}}`, designators in ASCII order), 2 (`"heading"`, a braced list and an `HTMLElement` convert into `string | NodeType`, `Fragment | Node | readonly Node[]` and the place union; `nodes` / `marks` take pairs), 3 (braced `{h, p1, ..}`, `{schema.mark("strong")}`), 5 (overloads; `nullptr` for `Attrs | null` through a constructor from `nullptr_t`), 7 (rule 2: `Schema`, not `Schema<ts::Value, ts::Value>`), 8 (lambdas, as today; the stored ones capture by value, `[app]`, `[tag]`), 9 (no `MarkSpec_attrs`, `Node_2`, `unsafe_as`), 10 (`{{"k", v}, ..}`).

#### Rust

Today
```rust
    let hattrs = MarkSpecAttrs::default();
    hattrs.set_at("level", &lvl);
    // ...
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            let inner = JsArray::<JsValue>::from([JsValue::from("code"), JsValue::from(0.0)]);
            DOMOutputSpec::Array(JsArray::from([JsValue::from("pre"), JsValue::from(inner)]))
    // ...
    let nodes = JsValue::object()
        .with("doc", &doc)
    // ...
    let spec = SchemaSpec::<JsValue, JsValue>::default()
    // ...
    let quote = schema.node_with("blockquote", None, q_inner, None);
    // ...
    let place = EditorViewNewPlaceOrDistDOMNode::from(AsRef::<dom::Node>::as_ref(&mount));
```

Target
```rust
fn hole_spec(tag: &str) -> DOMOutputSpec {
    DOMOutputSpec::Array(vec![tag.into(), 0.0.into()]) // (?) see Notes
}

fn build_schema() -> Schema {
    let doc = NodeSpec::default().with_content("block+");
    let heading = NodeSpec::default()
        .with_attrs([("level", AttributeSpec::default().with_default(2.0))])
        .with_content("inline*")
        .with_group("block")
        .with_defining(true)
        .with_to_dom(|n| {
            let l = n.attrs().at("level").as_f64() as i32;
            hole_spec(if l == 1 { "h1" } else { "h2" })
        });
    let cblock = NodeSpec::default()
        .with_content("text*")
        .with_marks("")
        .with_group("block")
        .with_code(true)
        .with_defining(true)
        .with_to_dom(|_| {
            DOMOutputSpec::Array(vec![
                "pre".into(),
                DOMOutputSpec::Array(vec!["code".into(), 0.0.into()]).into(),
            ])
        });
    // para, quote, hr, text: as today, with `.with_to_dom(|_| ..)`
    let make_mark = |tag: &'static str| MarkSpec::default().with_to_dom(move |_, _| hole_spec(tag));

    Schema::new(
        &SchemaSpec::default()
            .with_nodes([
                ("doc", doc),
                ("heading", heading),
                ("paragraph", para),
                ("blockquote", quote),
                ("code_block", cblock),
                ("horizontal_rule", hr),
                ("text", text),
            ])
            .with_marks([
                ("strong", make_mark("strong")),
                ("em", make_mark("em")),
                ("code", make_mark("code")),
            ]),
    )
}

fn make_initial_doc(schema: &Schema) -> Node {
    let h = schema.node("heading", [("level", 2.0)], schema.text("ProseMirror Document Schema"));
    let m_strong = schema.mark("strong");
    // m_em, m_code likewise
    let p1 = schema.node(
        "paragraph",
        None,
        [
            schema.text("Inline marks compose cleanly: "),
            schema.text_with("bold", [m_strong]),
            // ...
        ],
    );
    // q_inner, quote, cb, outro: schema.node("paragraph", None, child)
    schema.node("doc", None, [h, p1, quote, cb, outro])
}

    // start()
    let props = DirectEditorProps::default()
        .with_state(&make_state(&app.borrow().schema))
        .with_dispatch_transaction({
            let app = app.clone();
            move |tr| { /* as today */ }
        });
    let mount = dom_doc.get_element_by_id("editor").expect("missing #editor");
    let view = EditorView::new(&mount, &props);
```
Needs: 1 (builders), 2 (`impl Into<..>` for `string | NodeType`, the content union, `nodes` / `marks` and `place`), 3 (`[h, p1, ..]`; `vec![x.into(), ..]` for the `any` tuple), 5 (`content` counts as its primary member `Node`, a class, so the plain `node` keeps `attrs` and `content` and only `marks` goes to `node_with`; `None` for `null`; `text_with`), 7 (rule 2: `Schema`, and `SchemaSpec::default()` with no `<JsValue, JsValue>`), 8 (closures passed directly, arguments inferred), 9 (no `MarkSpecAttrs`, `NodeToDOMOutputSpecFn`, `MarkBooleanToDOMOutputSpecFn`, `TransactionFn`, `EditorViewNewPlaceOrDistDOMNode`), 10 (`[("k", v), ..]`).

#### Gleam

Today
```gleam
      t.DOMOutputSpecArray([
        dynamic.string("pre"),
        dynamic.array([dynamic.string("code"), dynamic.int(0)]),
  // ...
    |> node_spec.with_at("doc", pm_rt.dynamic(doc_spec))
  // ...
    |> schema_spec.with_nodes(pm_rt.dynamic(nodes))
  // ...
  let ms = schema.mark(sch, t.StringOrMarkTypeString("strong"))
  // ...
      t.StringOrNodeTypeString("blockquote"),
      None,
      Some(t.FragmentOrNodeOrNodeArrayNode(q_inner)),
  // ...
      Some(t.EditorViewNewPlaceOrDistDOMNodeNode2(dom_node.upcast(mount))),
```

Target
```gleam
import dom/areas/core/document
import gleam/dynamic
import gleam/dynamic/decode
import gleam/option.{Some}
import prosemirror/model/attribute_spec
import prosemirror/model/attrs
import prosemirror/model/dom_output_spec.{type DOMOutputSpec}
import prosemirror/model/node.{type Node}
import prosemirror/model/node_spec
import prosemirror/model/schema.{type Schema}
import prosemirror/model/schema_spec
import prosemirror/view/editor_view

fn hole_spec(tag: String) -> DOMOutputSpec {
  dom_output_spec.array([dynamic.string(tag), dynamic.int(0)])
}

fn build_schema() -> Schema {
  let heading_spec =
    node_spec.create()
    |> node_spec.with_attrs([
      #("level", attribute_spec.create() |> attribute_spec.with_default(dynamic.int(2))),
    ])
    |> node_spec.with_content("inline*")
    |> node_spec.with_group("block")
    |> node_spec.with_defining(True)
    |> node_spec.with_to_dom(fn(n) {
      let lvl = case
        decode.run(attrs.get_at(node.get_attrs(n), "level"), decode.int)
      {
        Ok(1) -> "h1"
        _ -> "h2"
      }
      hole_spec(lvl)
    })
  let cb_spec =
    node_spec.create()
    |> node_spec.with_content("text*")
    |> node_spec.with_marks("")
    |> node_spec.with_group("block")
    |> node_spec.with_code(True)
    |> node_spec.with_defining(True)
    |> node_spec.with_to_dom(fn(_) {
      dom_output_spec.array([
        dynamic.string("pre"),
        dynamic.array([dynamic.string("code"), dynamic.int(0)]),
      ])
    })
  // doc_spec, para_spec, quote_spec, hr_spec, text_spec, make_mark: as today

  schema_spec.create()
  |> schema_spec.with_nodes([
    #("doc", doc_spec),
    #("heading", heading_spec),
    #("paragraph", para_spec),
    #("blockquote", quote_spec),
    #("code_block", cb_spec),
    #("horizontal_rule", hr_spec),
    #("text", text_spec),
  ])
  |> schema_spec.with_marks([
    #("strong", make_mark("strong")),
    #("em", make_mark("em")),
    #("code", make_mark("code")),
  ])
  |> schema.new
}

fn make_initial_doc(sch: Schema) -> Node {
  let h =
    schema.node_with(
      sch,
      "heading",
      [#("level", dynamic.int(2))],
      schema.text(sch, "ProseMirror Document Schema"),
      [],
    )
  let ms = schema.mark(sch, "strong")
  // me, mc likewise
  let p1 =
    schema.node_list_with(
      sch,
      "paragraph",
      [],
      [
        schema.text(sch, "Inline marks compose cleanly: "),
        schema.text_with(sch, "bold", [ms]),
        // ...
      ],
      [],
    )
  // q_inner, quote, cb, outro: schema.node_with(sch, "paragraph", [], child, [])
  schema.node_list_with(sch, "doc", [], [h, p1, quote, cb, outro], [])
}

  // main()
  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new_dom_node(mount, props)
```
Needs: 1 (pipelines), 2 (`"heading"` and `"strong"` taken plainly; list content leaves one parameter's primary member, so `node_list_with`; `with_nodes` takes the record (rule 3); `place` has no primary member (rule 4), so `new_dom_node`; its meaningful `null` is `new_null`), 3 (`[dynamic.string(tag), dynamic.int(0)]` for the `any` tuple), 5 (`node_with` / `node_list_with` / `text_with` take their tail bare; `[]` stands for TS's `null` attrs and the absent `marks` (?)), 7 (rule 2: `Schema`, not `t.Schema(Dynamic, Dynamic, Nil)`), 8 (plain functions, as today), 9 (a `dom_output_spec` module for the library's `DOMOutputSpec`; no `t.FragmentOrNodeOrNodeArrayNode`, `t.StringOrNodeTypeString`, `mark_spec_attrs`, `pm_rt.dynamic`, `dom_node.upcast`), 10 (`[#("k", v), ..]`).

#### F#

Today
```fsharp
let private buildSchema () : Schema<obj, obj> =
    // ...
            attrs = JsRecord.Create [ "level", levelAttr ],
    // ...
            toDOM = fun (n: ProsemirrorModel.Node) ->
    // ...
            toDOM = fun _ -> !^(ResizeArray<obj> [ box "pre"; box [| box "code"; box 0 |] ])
    // ...
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
    // ...
    schema.node ("paragraph", content = !^(ResizeArray children))
    // ...
    let v = EditorView.Create ((mountEl :> Dom.Node), props)
```

Target
```fsharp
let private buildSchema () : Schema =
    let docSpec = NodeSpec.Create (content = "block+")
    let headingSpec =
        NodeSpec.Create (
            attrs = [ "level", AttributeSpec.Create (``default`` = 2) ],
            content = "inline*",
            group = "block",
            defining = true,
            toDOM = fun n ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )
    // paraSpec, quoteSpec, codeBlockSpec, hrSpec, textSpec, strongSpec, emSpec, codeSpec: as today

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^[
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
                "blockquote", quoteSpec
                "code_block", codeBlockSpec
                "horizontal_rule", hrSpec
                "text", textSpec
            ],
            marks = !^[ "strong", strongSpec; "em", emSpec; "code", codeSpec ]
        )
    )

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            "heading",
            attrs = [ "level", 2 ],
            content = schema.text "ProseMirror Document Schema"
        )
    // mStrong, mEm, mCode, pMarks (via p), quote, codeBlock, pOutro: as today
    schema.node ("doc", content = !^(ResizeArray [ h; pMarks; quote; codeBlock; pOutro ]))

    // main ()
    let v = EditorView.Create (mountEl, props)
```
Needs: 1 (`Type.Create(a = ..)`), 2 (`"heading"`, `schema.text ..` and `mountEl` convert at method arguments; a list keeps `!^`), 3 (`!^(ResizeArray [ .. ])`, as today), 5 (optional named arguments; `attrs` left out where TS passes `null`), 7 (rule 2: `Schema`, not `Schema<obj, obj>`), 8 (lambdas; `n` inferred), 9 (no `AnyOrOrderedMap`, `JsRecord.Create`, `:> Dom.Node`), 10 (`[ "k", v; .. ]`).

#### Notes
- **Did not fit, Rust's plain `node`.** HEADER's example `schema.node("doc")` contradicts its own rule that a union counts as its primary member. The primary member of `content: Fragment | Node | readonly Node[]` is `Node`, a class, so `content` is not left out of the plain name, and neither is `attrs` before it. The target follows the rule: `schema.node("doc", None, [..])`, and `node_with` adds only `marks`. This reads better than the example, since every call here passes content.
- (?) **Gleam `[]` for null and absent.** `node_with` takes its whole tail bare, and Gleam has no bare "absent", so `[]` stands for TS's `null` `attrs` (null is absent here) and for the left-out `marks`. ProseMirror treats them the same (`computeAttrs` gives equal attrs for `{}` and `null`; `Mark.setFrom([])` is `Mark.none`), but in general the generator cannot know that an empty pair list or list means the same as no argument.
- (?) F#: `!^` is erased, so the union's conversion from the pair list must itself build the JS object (`Object.fromEntries`), or `nodes` reaches JS as an F# list. `attrs` is typed as the pair list itself and needs no `!^`. A one-pair `[ "level", 2 ]` draws F#'s info FS3886 ("did you mean `;`").
- `DOMOutputSpec`'s tuple member `readonly [string, ...any[]]` has no name: it is `array` in Gleam and `Array` holding a `Vec<JsValue>` in Rust (name TBD). In C++, `return {tag, 0};` needs an `std::initializer_list<ts::Value>` constructor for that member, and the nested tuple is written `ts::Array<>{"code", 0}` (checked with clang against a mock).
