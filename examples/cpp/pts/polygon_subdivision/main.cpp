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

static const char* SHADES[9] = {
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626"
};

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
        double min_dim = std::min(size.x(), size.y());
        double radius = std::max(80.0, min_dim * 0.40);

        double px_norm = (pointer.x() / std::max(1.0, size.x())) - 0.5;
        double t = Num::clamp(0.22 + 0.16 * std::sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

        Group poly = Polygon::fromCenter(PtLike(c), radius, 6.0);
        poly.rotate2D(time_ms * 0.00035, PtLike(c));
        Pt centroid = Polygon::centroid(PtLikeIterable(poly));

        for (int depth = 0; depth < 9; depth++) {
            double sw = depth == 0 ? 1.6 : 1.0;
            form.fill(CanvasForm_fill_c(std::string(SHADES[depth])))
                .stroke(CanvasForm_fill_c(std::string("#111111")), sw)
                .polygon(PtLikeIterable(poly));
            if (depth == 0) {
                for (Pt pt : poly) {
                    form.strokeOnly(CanvasForm_fill_c(std::string("#d4d4d4")), 1.0)
                        .line(PtLikeIterable(mk_group2(centroid, pt)));
                    form.fillOnly(CanvasForm_fill_c(std::string("#111111")))
                        .point(PtLike(pt), 3.5, "circle");
                }
            }
            poly = Polygon::midpoints(PtIterable(poly), true, t);
        }

        form.strokeOnly(CanvasForm_fill_c(std::string("#ffffff")), 1.5)
            .line(PtLikeIterable(mk_group2(mk_pt(c.x() - 10.0, c.y()), mk_pt(c.x() + 10.0, c.y()))));
        form.strokeOnly(CanvasForm_fill_c(std::string("#ffffff")), 1.5)
            .line(PtLikeIterable(mk_group2(mk_pt(c.x(), c.y() - 10.0), mk_pt(c.x(), c.y() + 10.0))));
        form.fill(CanvasForm_fill_c(std::string("#ffffff")))
            .stroke(CanvasForm_fill_c(std::string("#111111")), 1.5)
            .point(PtLike(centroid), 5.0, "circle");

        Group ring = Circle::fromCenter(PtLike(pointer), 10.0);
        form.strokeOnly(CanvasForm_fill_c(std::string("#737373")), 1.5).circle(PtLikeIterable(ring));
    }));

    space.add(player);
    space.bindMouse().bindTouch().play();
}
