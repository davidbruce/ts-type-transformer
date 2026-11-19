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
        Pt size = space.size();
        Pt c = space.center();
        Pt pointer = space.pointer();
        double w = std::max(120.0, size.x());
        double h = std::max(120.0, size.y());
        double amp = h * 0.24;
        int count = 11;

        form.strokeOnly(CanvasForm_fill_c(std::string("#e5e5e5")), 1.0)
            .line(PtLikeIterable(mk_group2(mk_pt(w * 0.06, c.y()), mk_pt(w * 0.94, c.y()))));

        ts::Array<Pt> arr = ts::Array<Pt>::create();
        for (int i = 0; i < count; i++) {
            double u = double(i) / double(count - 1);
            double x = w * (0.08 + 0.84 * u);
            double fi = double(i);
            double y = c.y()
                + std::sin(time_ms * 0.0014 + fi * 0.72) * amp
                + std::cos(time_ms * 0.0009 - fi * 0.45) * (amp * 0.42);
            double dx = std::fabs(x - pointer.x());
            if (dx < w * 0.25 && (pointer.x() > 1.0 || pointer.y() > 1.0)) {
                double pull = 1.0 - dx / (w * 0.25);
                y += (pointer.y() - y) * pull * 0.35;
            }
            arr.push(mk_pt(x, y));
        }

        Group ctrls = Group::create(arr);
        Group bs = Curve::bspline(GroupLike(ctrls), 12.0);
        Group card = Curve::cardinal(PtLikeIterable(ctrls), 12.0, 0.35);
        Group cr = Curve::catmullRom(PtLikeIterable(ctrls), 12.0);

        uint32_t cr_len = cr.size();
        for (uint32_t i = 0; i < cr_len; i += 3) {
            Pt p = cr[i];
            form.strokeOnly(CanvasForm_fill_c(std::string("#d4d4d4")), 1.0)
                .line(PtLikeIterable(mk_group2(mk_pt(p.x(), c.y()), p)));
        }

        form.strokeOnly(CanvasForm_fill_c(std::string("#d4d4d4")), 1.0).line(PtLikeIterable(ctrls));
        form.strokeOnly(CanvasForm_fill_c(std::string("#b5b5b5")), 2.0).line(PtLikeIterable(bs));
        form.strokeOnly(CanvasForm_fill_c(std::string("#555555")), 1.8).line(PtLikeIterable(card));
        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 2.6).line(PtLikeIterable(cr));

        for (Pt p : ctrls) {
            form.fill(CanvasForm_fill_c(std::string("#ffffff")))
                .stroke(CanvasForm_fill_c(std::string("#111111")), 1.5)
                .point(PtLike(p), 4.5, "circle");
        }
    }));

    space.add(player);
    space.bindMouse().bindTouch().play();
}
