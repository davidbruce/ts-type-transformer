// SVG from C: elements made in the SVG namespace, attributes set, geometry and
// lengths read back through the SVG interfaces. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))
#define NEAR(a, b) ((a) - (b) < 0.001 && (b) - (a) < 0.001)

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    // An <svg> in the body: 200x100 pixels showing a 400x200 world.
    SVGSVGElement svg = Document_createElementNS_svg_svg(doc);
    Element_setAttribute(svg, "width", "200");
    Element_setAttribute(svg, "height", "100");
    Element_setAttribute(svg, "viewBox", "0 0 400 200");
    Node_appendChild(body, svg);
    CHECK(STR(Element_get_namespaceURI(svg), "http://www.w3.org/2000/svg"));
    CHECK(STR(Element_get_localName(svg), "svg") && STR(Element_get_tagName(svg), "svg"));
    CHECK(Node_get_isConnected(svg));
    CHECK(DOMRect_get_width(Element_getBoundingClientRect(svg)) == 200 && DOMRect_get_height(Element_getBoundingClientRect(svg)) == 100);

    // SVGAnimatedLength: the base value of width and height.
    SVGLength width = SVGAnimatedLength_get_baseVal(SVGSVGElement_get_width(svg));
    CHECK(SVGLength_get_value(width) == 200 && SVGLength_get_unitType(width) == SVGLength_SVG_LENGTHTYPE_NUMBER);
    CHECK(STR(SVGLength_get_valueAsString(width), "200"));
    CHECK(SVGLength_get_value(SVGAnimatedLength_get_animVal(SVGSVGElement_get_height(svg))) == 100);

    // The viewBox as a rectangle; it follows the attribute.
    SVGAnimatedRect view_box = SVGFitToViewBox_get_viewBox(svg);
    DOMRect view = SVGAnimatedRect_get_baseVal(view_box);
    CHECK(DOMRectReadOnly_get_x(view) == 0 && DOMRectReadOnly_get_width(view) == 400 && DOMRectReadOnly_get_height(view) == 200);
    Element_setAttribute(svg, "viewBox", "10 20 300 150");
    view = SVGAnimatedRect_get_baseVal(SVGFitToViewBox_get_viewBox(svg));
    CHECK(DOMRectReadOnly_get_x(view) == 10 && DOMRectReadOnly_get_y(view) == 20 && DOMRectReadOnly_get_width(view) == 300);
    Element_setAttribute(svg, "viewBox", "0 0 400 200");
    CHECK(DOMRectReadOnly_get_width(SVGAnimatedRect_get_baseVal(SVGFitToViewBox_get_viewBox(svg))) == 400);
    SVGPreserveAspectRatio aspect = SVGAnimatedPreserveAspectRatio_get_baseVal(SVGFitToViewBox_get_preserveAspectRatio(svg));
    CHECK(SVGPreserveAspectRatio_get_align(aspect) == 6 && SVGPreserveAspectRatio_get_meetOrSlice(aspect) == 1);  // xMidYMid meet

    // Shapes: a rect, a circle, a line and a path, each made by tag.
    SVGRectElement rect = Document_createElementNS_svg_rect(doc);
    Element_setAttribute(rect, "x", "10");
    Element_setAttribute(rect, "y", "20");
    Element_setAttribute(rect, "width", "100");
    Element_setAttribute(rect, "height", "50");
    Element_setAttribute(rect, "fill", "tomato");
    Node_appendChild(svg, rect);
    SVGCircleElement circle = Document_createElementNS_svg_circle(doc);
    Element_setAttribute(circle, "cx", "150");
    Element_setAttribute(circle, "cy", "60");
    Element_setAttribute(circle, "r", "20");
    Node_appendChild(svg, circle);
    CHECK(ParentNode_get_childElementCount(svg) == 2);
    CHECK(STR(Element_get_tagName(rect), "rect") && STR(Element_getAttribute(rect, "fill"), "tomato"));
    CHECK(js_same(SVGElement_get_ownerSVGElement(rect), svg));
    CHECK(js_same(SVGElement_get_viewportElement(circle), svg));

    // Lengths read through SVGAnimatedLength; setting one writes the attribute.
    CHECK(SVGLength_get_value(SVGAnimatedLength_get_baseVal(SVGRectElement_get_width(rect))) == 100);
    CHECK(SVGLength_get_value(SVGAnimatedLength_get_baseVal(SVGRectElement_get_x(rect))) == 10);
    CHECK(SVGLength_get_value(SVGAnimatedLength_get_baseVal(SVGCircleElement_get_r(circle))) == 20);
    SVGLength rect_width = SVGAnimatedLength_get_baseVal(SVGRectElement_get_width(rect));
    SVGLength_set_value(rect_width, 120);
    CHECK(STR(Element_getAttribute(rect, "width"), "120"));
    SVGLength_set_valueAsString(rect_width, "2in");
    CHECK(SVGLength_get_unitType(rect_width) == SVGLength_SVG_LENGTHTYPE_IN);
    CHECK(SVGLength_get_valueInSpecifiedUnits(rect_width) == 2 && SVGLength_get_value(rect_width) == 192);
    SVGLength_convertToSpecifiedUnits(rect_width, SVGLength_SVG_LENGTHTYPE_PX);
    CHECK(SVGLength_get_valueInSpecifiedUnits(rect_width) == 192 && STR(Element_getAttribute(rect, "width"), "192px"));
    SVGLength_newValueSpecifiedUnits(rect_width, SVGLength_SVG_LENGTHTYPE_NUMBER, 100);
    CHECK(STR(Element_getAttribute(rect, "width"), "100"));
    SVGLength_set_valueAsString(rect_width, "25%");
    CHECK(SVGLength_get_unitType(rect_width) == SVGLength_SVG_LENGTHTYPE_PERCENTAGE && SVGLength_get_value(rect_width) == 100);  // 25% of 400
    SVGLength_newValueSpecifiedUnits(rect_width, SVGLength_SVG_LENGTHTYPE_NUMBER, 100);

    // Bounding boxes: of one shape, and of a group of two.
    DOMRect rect_box = SVGGraphicsElement_getBBox(rect);
    CHECK(DOMRectReadOnly_get_x(rect_box) == 10 && DOMRectReadOnly_get_y(rect_box) == 20);
    CHECK(DOMRectReadOnly_get_width(rect_box) == 100 && DOMRectReadOnly_get_height(rect_box) == 50);
    SVGGElement group = Document_createElementNS_svg_g(doc);
    Node_appendChild(svg, group);
    Node_appendChild(group, rect);
    Node_appendChild(group, circle);
    DOMRect group_box = SVGGraphicsElement_getBBox(group);
    CHECK(NEAR(DOMRectReadOnly_get_x(group_box), 10) && NEAR(DOMRectReadOnly_get_y(group_box), 20));
    CHECK(NEAR(DOMRectReadOnly_get_width(group_box), 160) && NEAR(DOMRectReadOnly_get_height(group_box), 60));
    CHECK(ParentNode_get_childElementCount(svg) == 1 && ParentNode_get_childElementCount(group) == 2);

    // Transforms: the viewBox halves the world; a translate moves the group.
    CHECK(NEAR(DOMMatrixReadOnly_get_a(SVGGraphicsElement_getCTM(rect)), 0.5));
    Element_setAttribute(group, "transform", "translate(30, 40)");
    DOMMatrix ctm = SVGGraphicsElement_getCTM(rect);
    CHECK(NEAR(DOMMatrixReadOnly_get_e(ctm), 15) && NEAR(DOMMatrixReadOnly_get_f(ctm), 20));
    CHECK(NEAR(DOMRectReadOnly_get_x(SVGGraphicsElement_getBBox(group)), 10));  // a box ignores the element's own transform
    CHECK(NEAR(DOMRectReadOnly_get_x(Element_getBoundingClientRect(rect)) - DOMRectReadOnly_get_x(Element_getBoundingClientRect(svg)), 20));
    CHECK(NEAR(DOMRectReadOnly_get_width(Element_getBoundingClientRect(rect)), 50));
    CHECK(NEAR(DOMMatrixReadOnly_get_a(SVGGraphicsElement_getScreenCTM(svg)), 0.5));  // the viewBox scale is part of the screen matrix

    // A path: its length, points along it, and hit tests.
    SVGPathElement path = Document_createElementNS_svg_path(doc);
    Element_setAttribute(path, "d", "M 0 0 L 100 0 L 100 100");
    Element_setAttribute(path, "stroke", "black");
    Element_setAttribute(path, "fill", "none");
    Node_appendChild(svg, path);
    CHECK(NEAR(SVGGeometryElement_getTotalLength(path), 200));
    DOMPoint mid = SVGGeometryElement_getPointAtLength(path, 100);
    CHECK(NEAR(DOMPoint_get_x(mid), 100) && NEAR(DOMPoint_get_y(mid), 0));
    DOMPoint later = SVGGeometryElement_getPointAtLength(path, 150);
    CHECK(NEAR(DOMPoint_get_x(later), 100) && NEAR(DOMPoint_get_y(later), 50));
    Element_setAttribute(path, "d", "M 0 0 H 30 V 40 Z");
    CHECK(NEAR(SVGGeometryElement_getTotalLength(path), 30 + 40 + 50));
    DOMPointInit on_edge = DOMPointInit_create();
    DOMPointInit_set_x(on_edge, js_some_double(15));
    DOMPointInit_set_y(on_edge, js_some_double(0));
    CHECK(SVGGeometryElement_isPointInStroke(path, on_edge));
    Element_setAttribute(path, "fill", "red");
    DOMPointInit inside = DOMPointInit_create();
    DOMPointInit_set_x(inside, js_some_double(25));
    DOMPointInit_set_y(inside, js_some_double(25));
    CHECK(SVGGeometryElement_isPointInFill(path, inside));
    DOMPointInit outside = DOMPointInit_create();
    DOMPointInit_set_x(outside, js_some_double(5));
    DOMPointInit_set_y(outside, js_some_double(35));
    CHECK(!SVGGeometryElement_isPointInFill(path, outside));
    // pathLength rescales distances.
    SVGAnimatedNumber declared = SVGGeometryElement_get_pathLength(path);
    CHECK(SVGAnimatedNumber_get_baseVal(declared) == 0);
    SVGAnimatedNumber_set_baseVal(declared, 10);
    CHECK(STR(Element_getAttribute(path, "pathLength"), "10"));

    // Other tags in the namespace: text measures itself; polygon points are a list.
    SVGTextElement label = Document_createElementNS_svg_text(doc);
    Element_setAttribute(label, "x", "5");
    Element_setAttribute(label, "y", "90");
    Node_set_textContent(label, "Hello");
    Node_appendChild(svg, label);
    CHECK(SVGTextContentElement_getNumberOfChars(label) == 5);
    CHECK(SVGTextContentElement_getComputedTextLength(label) > 10);
    CHECK(DOMRectReadOnly_get_width(SVGGraphicsElement_getBBox(label)) > 10);
    SVGPolygonElement triangle = Document_createElementNS_svg_polygon(doc);
    Element_setAttribute(triangle, "points", "0,0 10,0 10,10");
    Node_appendChild(svg, triangle);
    CHECK(SVGPointList_get_numberOfItems(SVGAnimatedPoints_get_points(triangle)) == 3);
    CHECK(DOMPoint_get_x(SVGPointList_getItem(SVGAnimatedPoints_get_points(triangle), 1)) == 10);
    // The generic constructor makes the same kind of element.
    Element generic = Document_createElementNS(doc, "http://www.w3.org/2000/svg", "ellipse");
    CHECK(STR(Element_get_localName(generic), "ellipse") && STR(Element_get_namespaceURI(generic), "http://www.w3.org/2000/svg"));
    SVGElement named = Document_createElementNS_svg(doc, "line");
    CHECK(STR(Element_get_tagName(named), "line"));
    // An HTML element made the usual way is not in the namespace.
    CHECK(STR(Element_get_namespaceURI(Document_createElement_div(doc)), "http://www.w3.org/1999/xhtml"));

    // Styling and classes work on SVG elements as on HTML ones.
    DOMTokenList_toggle(Element_get_classList(circle), "hot", js_none_bool());
    CHECK(STR(Element_getAttribute(circle, "class"), "hot"));
    CSSStyleDeclarationBase_setProperty(ElementCSSInlineStyle_get_style(circle), "fill", "rgb(0, 0, 255)");
    CHECK(STR(CSSStyleDeclarationBase_getPropertyValue(getComputedStyle(circle, 0), "fill"), "rgb(0, 0, 255)"));
    CHECK(SVGSVGElement_getElementById(svg, "nope").h == 0);
    Element_set_id(circle, "dot");
    CHECK(js_same(SVGSVGElement_getElementById(svg, "dot"), circle));

    // Created points and rectangles, free of the document.
    DOMPoint point = SVGSVGElement_createSVGPoint(svg);
    DOMPoint_set_x(point, 7);
    CHECK(DOMPoint_get_x(point) == 7 && DOMPoint_get_y(point) == 0);
}
