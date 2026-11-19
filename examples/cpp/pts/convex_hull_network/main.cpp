#include <algorithm>
#include <cmath>
#include <string>

#include "pts/pts.hpp"

using namespace pts;

static Pt mk_pt(double x, double y) {
    Pt p = Pt::make(2.0);
    p.x(x);
    p.y(y);
    return p;
}

static Group mk_group2(const Pt& a, const Pt& b) {
    return Group::create(ts::Array<Pt>{ a, b });
}

extern "C" __attribute__((export_name("start"))) void start() {
    CanvasSpaceOptions opts = CanvasSpaceOptions::create();
    opts.bgcolor("#ffffff");
    opts.resize(true);
    opts.retina(true);

    CanvasSpace space = CanvasSpace::create(std::string("#pt")).setup(opts);
    CanvasForm<CanvasSpace> form = space.getForm();

    IPlayer player = IPlayer::create();
    player.animate(AnimateCallbackFn([space, form](double time_ms, double, Space) {
        Pt c = space.center();
        Pt size = space.size();
        Pt pointer = space.pointer();
        double base_r = std::max(70.0, std::min(size.x(), size.y()) * 0.34);
        Group radial = Create::radialPts(PtLike(c), base_r, 20.0, time_ms * 0.00025);

        ts::Array<Pt> cloud_arr = ts::Array<Pt>::create();
        uint32_t r_len = radial.size();
        for (uint32_t i = 0; i < r_len; i++) {
            Pt p = radial[i];
            double fi = double(i);
            double scale = 0.48 + 0.52 * (0.5 + 0.5 * std::sin(time_ms * 0.0011 + fi * 0.9) * std::cos(time_ms * 0.0007 - fi * 0.5));
            cloud_arr.push(mk_pt(c.x() + (p.x() - c.x()) * scale, c.y() + (p.y() - c.y()) * scale));
        }
        if (pointer.x() > 1.0 || pointer.y() > 1.0) {
            cloud_arr.push(mk_pt(pointer.x(), pointer.y()));
        }

        Group cloud = Group::create(cloud_arr);
        Group hull = Polygon::convexHull(PtLikeIterable(cloud), false);
        Group bbox = Geom::boundingBox(PtIterable(hull));
        Pt cent = Polygon::centroid(PtLikeIterable(hull));

        form.strokeOnly(CanvasForm_fill_c(std::string("#b5b5b5")), 1.0).rect(PtLikeIterable(bbox));
        form.fill(CanvasForm_fill_c(std::string("#f7f7f7")))
            .stroke(CanvasForm_fill_c(std::string("#111111")), 2.0)
            .polygon(PtLikeIterable(hull));

        for (Group seg : Polygon::network(PtIterable(hull), 0.0)) {
            form.strokeOnly(CanvasForm_fill_c(std::string("#d4d4d4")), 1.0).line(PtLikeIterable(seg));
        }
        uint32_t hull_len = hull.size();
        if (hull_len > 3) {
            for (Group seg : Polygon::network(PtIterable(hull), double(hull_len / 2))) {
                form.strokeOnly(CanvasForm_fill_c(std::string("#d4d4d4")), 1.0).line(PtLikeIterable(seg));
            }
        }

        for (Pt p : cloud) {
            form.fillOnly(CanvasForm_fill_c(std::string("#737373"))).point(PtLike(p), 3.0, "circle");
        }
        for (Pt h : hull) {
            form.fill(CanvasForm_fill_c(std::string("#111111")))
                .stroke(CanvasForm_fill_c(std::string("#ffffff")), 1.5)
                .point(PtLike(h), 5.0, "circle");
        }

        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 1.4)
            .line(PtLikeIterable(mk_group2(mk_pt(cent.x() - 10.0, cent.y()), mk_pt(cent.x() + 10.0, cent.y()))));
        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 1.4)
            .line(PtLikeIterable(mk_group2(mk_pt(cent.x(), cent.y() - 10.0), mk_pt(cent.x(), cent.y() + 10.0))));
        form.fill(CanvasForm_fill_c(std::string("#ffffff")))
            .stroke(CanvasForm_fill_c(std::string("#111111")), 1.6)
            .point(PtLike(cent), 5.5, "circle");
    }));

    space.add(player);
    space.bindMouse().bindTouch().play();
}
