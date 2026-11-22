#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static void push_slice(JsArray_Echarts_PieSeriesOption_data data, const char* name, double value) {
    JsArray_Echarts_PieSeriesOption_data_push(data, Echarts_PieDataItemOption_create(.name = Echarts_OptionName_of(name), .value = js_number(value)));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    JsArray palette = JsArray_new();
    const char* cols[] = { "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" };
    for (int i = 0; i < 5; i++) js_push_str(palette, cols[i]);

    JsArray_Echarts_PositionSizeOption radius = JsArray_Echarts_PositionSizeOption_new();
    JsArray_Echarts_PositionSizeOption_push(radius, "44%");
    JsArray_Echarts_PositionSizeOption_push(radius, "72%");

    JsArray_Echarts_PositionSizeOption center = JsArray_Echarts_PositionSizeOption_new();
    JsArray_Echarts_PositionSizeOption_push(center, "50%");
    JsArray_Echarts_PositionSizeOption_push(center, "46%");

    JsArray_Echarts_PieSeriesOption_data data = JsArray_Echarts_PieSeriesOption_data_new();
    push_slice(data, "Runtime", 34);
    push_slice(data, "Bindings", 26);
    push_slice(data, "Glue", 18);
    push_slice(data, "Parser", 14);
    push_slice(data, "Tests", 8);

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = Echarts_Color_of("#ffffff"),
        .color = Echarts_BarSeriesOption_color_of(palette),
        .legend = EChartsOption_legend_of(Echarts_PlainLegendComponentOption_create(
            .bottom = Echarts_PositionSizeOption_of(8), .textStyle = Echarts_LabelOption_create(.color = js_string("#111111")))),
        .tooltip = EChartsOption_tooltip_of(Echarts_TooltipComponentOption_create(.trigger = Echarts_TooltipComponentOption_trigger_item)),
        .series = EChartsOption_series_of(Echarts_PieSeriesOption_create(
            .name = Echarts_OptionName_of("Subsystems"),
            .radius = Echarts_string_or_number_or_string_or_number_array_of(radius),
            .center = Echarts_string_or_number_or_string_or_number_array_of(center),
            .itemStyle = Echarts_PieItemStyleOption_create(.borderColor = Echarts_Color_of("#ffffff"), .borderWidth = 2),
            .label = Echarts_PieLabelOption_create(.color = "#111111"),
            .data = data)));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
