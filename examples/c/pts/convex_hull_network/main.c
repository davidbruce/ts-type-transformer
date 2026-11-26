#include "pts/pts.h"
#include <math.h>

static Pts_Pt mk_pt(double x, double y) {
    Pts_Pt p = Pts_Pt_make(2.0);
    Pts_Pt_set_x(p, x);
    Pts_Pt_set_y(p, y);
    return p;
}

static Pts_Group mk_group2(Pts_Pt a, Pts_Pt b) {
    return Pts_Group_new(a, b);
}

static Pts_CanvasSpace g_space;
static Pts_CanvasForm g_form;

static void on_animate(void* ctx, double time_ms, double ftime_ms, Pts_Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope();

    Pts_Pt c = Pts_CanvasSpace_get_center(g_space);
    Pts_Pt size = Pts_CanvasSpace_get_size(g_space);
    Pts_Pt pointer = Pts_CanvasSpace_get_pointer(g_space);
    double sx = Pts_Pt_get_x(size), sy = Pts_Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    double base_r = min_dim * 0.34;
    if (base_r < 70.0) base_r = 70.0;
    double cx = Pts_Pt_get_x(c), cy = Pts_Pt_get_y(c);

    Pts_Group radial = Pts_Create_radialPts(c, base_r, 20.0, time_ms * 0.00025);
    JsArray_Pts_Pt r_arr = Pts_Group_as_array(radial);
    uint32_t r_len = JsArray_Pts_Pt_length(r_arr);

    JsArray_Pts_Pt cloud_arr = JsArray_Pts_Pt_new();
    for (uint32_t i = 0; i < r_len; i++) {
        Pts_Pt p = JsArray_Pts_Pt_get(r_arr, i);
        double fi = (double)i;
        double scale = 0.48 + 0.52 * (0.5 + 0.5 * sin(time_ms * 0.0011 + fi * 0.9) * cos(time_ms * 0.0007 - fi * 0.5));
        double nx = cx + (Pts_Pt_get_x(p) - cx) * scale;
        double ny = cy + (Pts_Pt_get_y(p) - cy) * scale;
        JsArray_Pts_Pt_push(cloud_arr, mk_pt(nx, ny));
    }
    double px = Pts_Pt_get_x(pointer), py = Pts_Pt_get_y(pointer);
    if (px > 1.0 || py > 1.0) {
        JsArray_Pts_Pt_push(cloud_arr, mk_pt(px, py));
    }

    Pts_Group cloud = Pts_Group_new_list(cloud_arr);
    Pts_Group hull = Pts_Polygon_convexHull(cloud, false);
    Pts_Group bbox = Pts_Geom_boundingBox(hull);
    Pts_Pt cent = Pts_Polygon_centroid(hull);

    Pts_CanvasForm_rect(Pts_CanvasForm_strokeOnly(g_form, "#b5b5b5", 1.0), bbox);
    Pts_CanvasForm_polygon(
        Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#f7f7f7"), "#111111", 2.0),
        hull
    );

    JsArray_Pts_Group net0 = Pts_Polygon_network(hull, 0.0);
    uint32_t net0_len = JsArray_Pts_Group_length(net0);
    for (uint32_t i = 0; i < net0_len; i++) {
        Pts_CanvasForm_line(Pts_CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), JsArray_Pts_Group_get(net0, i));
    }
    JsArray_Pts_Pt hull_arr = Pts_Group_as_array(hull);
    uint32_t hull_len = JsArray_Pts_Pt_length(hull_arr);
    if (hull_len > 3) {
        JsArray_Pts_Group net_mid = Pts_Polygon_network(hull, (double)(hull_len / 2));
        uint32_t nm_len = JsArray_Pts_Group_length(net_mid);
        for (uint32_t i = 0; i < nm_len; i++) {
            Pts_CanvasForm_line(Pts_CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), JsArray_Pts_Group_get(net_mid, i));
        }
    }

    uint32_t cl_len = JsArray_Pts_Pt_length(cloud_arr);
    for (uint32_t i = 0; i < cl_len; i++) {
        Pts_CanvasForm_point(Pts_CanvasForm_fillOnly(g_form, "#737373"), JsArray_Pts_Pt_get(cloud_arr, i), 3.0, "circle");
    }
    for (uint32_t i = 0; i < hull_len; i++) {
        Pts_CanvasForm_point(
            Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#111111"), "#ffffff", 1.5),
            JsArray_Pts_Pt_get(hull_arr, i), 5.0, "circle"
        );
    }

    double cent_x = Pts_Pt_get_x(cent), cent_y = Pts_Pt_get_y(cent);
    Pts_CanvasForm_line(
        Pts_CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        mk_group2(mk_pt(cent_x - 10.0, cent_y), mk_pt(cent_x + 10.0, cent_y))
    );
    Pts_CanvasForm_line(
        Pts_CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        mk_group2(mk_pt(cent_x, cent_y - 10.0), mk_pt(cent_x, cent_y + 10.0))
    );
    Pts_CanvasForm_point(
        Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#ffffff"), "#111111", 1.6),
        cent, 5.5, "circle"
    );
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    g_space = Pts_CanvasSpace_setup(Pts_CanvasSpace_new("#pt"),
        Pts_CanvasSpaceOptions_create(.bgcolor = "#ffffff", .resize = true, .retina = true));
    g_form = Pts_CanvasSpace_getForm(g_space);

    Pts_IPlayer player = Pts_IPlayer_create();
    Pts_IPlayer_set_animate(player, on_animate);
    Pts_CanvasSpace_add(g_space, player);
    Pts_CanvasSpace_bindMouse(g_space);
    Pts_CanvasSpace_bindTouch(g_space);
    Pts_CanvasSpace_play(g_space);

    js_retain(g_space);
    js_retain(g_form);
}
