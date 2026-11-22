#include <algorithm>
#include <cmath>
#include <string>

#include "pts/pts.hpp"

using namespace pts;

static Pt mk_pt(double x, double y) {
    Pt p = Pt::make(2.0);
    return p.x(x).y(y);
}

TS_EXPORT(start) void start() {
    auto space = CanvasSpace::create("#pt").setup({{.bgcolor = "#ffffff", .resize = true, .retina = true}});
    auto form = space.getForm();

    auto player = IPlayer::create();
    player.animate([space, form](double time_ms, double, Space) {
        Pt c = space.center();
        Pt size = space.size();
        Pt pointer = space.pointer();
        double base_r = std::max(70.0, std::min(size.x(), size.y()) * 0.34);
        Group radial = Create::radialPts(c, base_r, 20.0, time_ms * 0.00025);

        auto cloud_arr = ts::Array<Pt>::create();
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
        Group hull = Polygon::convexHull(cloud, false);
        Group bbox = Geom::boundingBox(hull);
        Pt cent = Polygon::centroid(hull);

        form.strokeOnly("#b5b5b5", 1.0).rect(bbox);
        form.fill("#f7f7f7")
            .stroke("#111111", 2.0)
            .polygon(hull);

        for (Group seg : Polygon::network(hull, 0.0)) {
            form.strokeOnly("#d4d4d4", 1.0).line(seg);
        }
        uint32_t hull_len = hull.size();
        if (hull_len > 3) {
            for (Group seg : Polygon::network(hull, double(hull_len / 2))) {
                form.strokeOnly("#d4d4d4", 1.0).line(seg);
            }
        }

        for (Pt p : cloud) {
            form.fillOnly("#737373").point(p, 3.0, "circle");
        }
        for (Pt h : hull) {
            form.fill("#111111")
                .stroke("#ffffff", 1.5)
                .point(h, 5.0, "circle");
        }

        form.strokeOnly("#111111", 1.4)
            .line(Group::create(mk_pt(cent.x() - 10.0, cent.y()), mk_pt(cent.x() + 10.0, cent.y())));
        form.strokeOnly("#111111", 1.4)
            .line(Group::create(mk_pt(cent.x(), cent.y() - 10.0), mk_pt(cent.x(), cent.y() + 10.0)));
        form.fill("#ffffff")
            .stroke("#111111", 1.6)
            .point(cent, 5.5, "circle");
    });

    space.add(player);
    space.bindMouse().bindTouch().play();
}
