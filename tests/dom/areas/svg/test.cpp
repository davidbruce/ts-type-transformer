// SVG from C++: elements made in the SVG namespace, attributes, lengths,
// view boxes, boxes and path lengths. Every CHECK must hold.
#include <cmath>
#include <string>

#include "areas/core/Document.hpp"
#include "areas/core/DOMPoint.hpp"
#include "areas/core/DOMRect.hpp"
#include "areas/core/functions.hpp"
#include "areas/svg/SVGAnimatedLength.hpp"
#include "areas/svg/SVGAnimatedRect.hpp"
#include "areas/svg/SVGCircleElement.hpp"
#include "areas/svg/SVGElement.hpp"
#include "areas/svg/SVGGElement.hpp"
#include "areas/svg/SVGLength.hpp"
#include "areas/svg/SVGLineElement.hpp"
#include "areas/svg/SVGPathElement.hpp"
#include "areas/svg/SVGRectElement.hpp"
#include "areas/svg/SVGSVGElement.hpp"
#include "areas/svg/SVGTextElement.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 0.01) { return std::fabs(a - b) < eps; }

static const char* SVG_NS = "http://www.w3.org/2000/svg";

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // An <svg> made in the SVG namespace, in the body.
    SVGSVGElement svg = doc.createElementNS_svg(SVGElementTagNameMapKeys::svg);
    svg.setAttribute("width", "200");
    svg.setAttribute("height", "100");
    svg.setAttribute("viewBox", "0 0 400 200");
    body.appendChild(svg);
    CHECK(svg.namespaceURI() == SVG_NS);
    CHECK(svg.localName() == "svg" && svg.tagName() == "svg");
    CHECK(svg.isConnected());
    CHECK(svg.getAttribute("viewBox") == "0 0 400 200");

    // Lengths: width as a SVGAnimatedLength with its base value.
    SVGLength width = svg.width().baseVal();
    CHECK(width.value() == 200 && width.unitType() == SVGLength::SVG_LENGTHTYPE_NUMBER);
    CHECK(width.valueAsString() == "200" && svg.height().baseVal().value() == 100);
    CHECK(svg.width().animVal().value() == 200);
    width.value(300);
    CHECK(svg.getAttribute("width") == "300");
    svg.setAttribute("width", "50%");
    CHECK(svg.width().baseVal().unitType() == SVGLength::SVG_LENGTHTYPE_PERCENTAGE);
    CHECK(svg.width().baseVal().valueInSpecifiedUnits() == 50);
    svg.width().baseVal().valueAsString("200px");
    CHECK(svg.width().baseVal().unitType() == SVGLength::SVG_LENGTHTYPE_PX && svg.width().baseVal().value() == 200);
    svg.width().baseVal().valueAsString("1in");
    CHECK(svg.width().baseVal().unitType() == SVGLength::SVG_LENGTHTYPE_IN && near(svg.width().baseVal().value(), 96));
    svg.width().baseVal().newValueSpecifiedUnits(SVGLength::SVG_LENGTHTYPE_PX, 200);
    CHECK(svg.getAttribute("width") == "200px");

    // The view box: baseVal is a rectangle.
    DOMRect view = svg.viewBox().baseVal();
    CHECK(view.x() == 0 && view.y() == 0 && view.width() == 400 && view.height() == 200);
    svg.setAttribute("viewBox", "10 20 300 150");
    DOMRect moved = svg.viewBox().baseVal();
    CHECK(moved.x() == 10 && moved.y() == 20 && moved.width() == 300 && moved.height() == 150);
    svg.setAttribute("viewBox", "0 0 400 200");

    // A rect: attributes in, lengths and box out.
    SVGRectElement rect = doc.createElementNS_svg(SVGElementTagNameMapKeys::rect);
    rect.setAttribute("x", "20");
    rect.setAttribute("y", "30");
    rect.setAttribute("width", "120");
    rect.setAttribute("height", "60");
    rect.setAttribute("rx", "5");
    rect.setAttribute("fill", "tomato");
    svg.appendChild(rect);
    CHECK(rect.namespaceURI() == SVG_NS && rect.tagName() == "rect");
    CHECK(rect.x().baseVal().value() == 20 && rect.y().baseVal().value() == 30);
    CHECK(rect.width().baseVal().value() == 120 && rect.height().baseVal().value() == 60);
    CHECK(rect.rx().baseVal().value() == 5);
    DOMRect box = rect.getBBox();
    CHECK(box.x() == 20 && box.y() == 30 && box.width() == 120 && box.height() == 60);
    CHECK(rect.ownerSVGElement() && *rect.ownerSVGElement() == svg);
    CHECK(rect.viewportElement() && *rect.viewportElement() == svg);
    rect.width().baseVal().value(150);
    CHECK(rect.getAttribute("width") == "150" && rect.getBBox().width() == 150);

    // A circle through the generic createElementNS.
    Element generic = doc.createElementNS(std::string_view(SVG_NS), "circle");
    CHECK(generic.namespaceURI() == SVG_NS && generic.localName() == "circle");
    svg.appendChild(generic);
    generic.setAttribute("cx", "200");
    generic.setAttribute("cy", "100");
    generic.setAttribute("r", "40");
    SVGCircleElement circle = doc.createElementNS_svg(SVGElementTagNameMapKeys::circle);
    circle.setAttribute("cx", "50");
    circle.setAttribute("cy", "150");
    circle.setAttribute("r", "10");
    svg.appendChild(circle);
    CHECK(circle.r().baseVal().value() == 10 && circle.cx().baseVal().value() == 50);
    DOMRect cb = circle.getBBox();
    CHECK(cb.x() == 40 && cb.y() == 140 && cb.width() == 20 && cb.height() == 20);
    circle.r().baseVal().value(25);
    CHECK(circle.getBBox().width() == 50 && circle.getAttribute("r") == "25");

    // A path: its length and points along it.
    SVGPathElement path = doc.createElementNS_svg(SVGElementTagNameMapKeys::path);
    path.setAttribute("d", "M 0 0 L 30 40");
    path.setAttribute("stroke", "black");
    svg.appendChild(path);
    CHECK(near(path.getTotalLength(), 50));
    DOMPoint mid = path.getPointAtLength(25);
    CHECK(near(mid.x(), 15) && near(mid.y(), 20));
    path.setAttribute("d", "M 0 0 H 100 V 100");
    CHECK(near(path.getTotalLength(), 200));
    DOMRect pb = path.getBBox();
    CHECK(pb.width() == 100 && pb.height() == 100);
    path.setAttribute("d", "M 10 10 h 20 v 20 h -20 z");
    CHECK(near(path.getTotalLength(), 80));

    // A line and a group: the group's box covers its members.
    SVGGElement group = doc.createElementNS_svg(SVGElementTagNameMapKeys::g);
    svg.appendChild(group);
    SVGLineElement line = doc.createElementNS_svg(SVGElementTagNameMapKeys::line);
    line.setAttribute("x1", "300");
    line.setAttribute("y1", "10");
    line.setAttribute("x2", "380");
    line.setAttribute("y2", "90");
    line.setAttribute("stroke", "blue");
    group.appendChild(line);
    SVGRectElement square = doc.createElementNS_svg(SVGElementTagNameMapKeys::rect);
    square.setAttribute("x", "280");
    square.setAttribute("y", "100");
    square.setAttribute("width", "20");
    square.setAttribute("height", "20");
    group.appendChild(square);
    CHECK(near(line.getTotalLength(), std::sqrt(80.0 * 80 + 80 * 80)));
    DOMRect gb = group.getBBox();
    CHECK(gb.x() == 280 && gb.y() == 10 && gb.width() == 100 && gb.height() == 110);
    CHECK(group.children().length() == 2);
    CHECK(svg.getElementById("hidden") == std::nullopt);

    // Text, and the DOM tree of it.
    SVGTextElement label = doc.createElementNS_svg(SVGElementTagNameMapKeys::text);
    label.setAttribute("x", "10");
    label.setAttribute("y", "190");
    label.textContent("hello svg");
    svg.appendChild(label);
    CHECK(label.textContent() == "hello svg" && label.getBBox().width() > 0);
    CHECK(svg.children().length() == 6);
    CHECK(svg.querySelectorAll("rect").length() == 2);
    CHECK(svg.querySelector("circle[r='40']").has_value());

    // Elements in the page lay out: the svg has a size on screen.
    DOMRect screen = svg.getBoundingClientRect();
    CHECK(near(screen.width(), 200) && near(screen.height(), 100));
    // A viewBox twice the size: 1 user unit is half a pixel.
    DOMRect inner = rect.getBoundingClientRect();
    CHECK(near(inner.width(), 75));

    // classList and style work on SVG elements as on HTML ones.
    rect.classList().add({"hot"});
    CHECK(rect.getAttribute("class") == "hot" && rect.classList().contains("hot"));
    rect.style().setProperty("opacity", "0.5");
    CHECK(rect.style().getPropertyValue("opacity") == "0.5");

    // Removed from the tree, an element keeps its attributes.
    rect.remove();
    CHECK(!rect.isConnected() && rect.getAttribute("fill") == "tomato");
    CHECK(svg.children().length() == 5);
}
