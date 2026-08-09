//! Drug trend chart — port of `DrugTrendChart.vue`.
//!
//! The Vue version rendered with Apache ECharts; with no JS bundler in the
//! Leptos build the chart is drawn on a `<canvas>` with the 2D API instead.
//! Visual parity is preserved: 12 bars with a top-to-bottom gradient, a
//! 3-month moving-average line, dashed grid lines, compact `K` / `฿M` y-axis
//! labels and an HTML axis tooltip that mirrors the ECharts one.  The chart
//! keeps the whole stage mounted and redraws on data / size / hover changes;
//! the loading and empty states are overlaid on top.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use leptos::html::{Canvas, Div};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlElement, MouseEvent};

use crate::components::icons::{Icon, IconKind};
use crate::models::{format_baht, format_qty, ChartSeries, Side, THAI_MONTHS_SHORT};

/// Canvas plot layout of the last redraw; used to map mouse events to months.
#[derive(Clone, Copy)]
struct Layout {
  left: f64,
  top: f64,
  right: f64,
  bottom: f64,
  band: f64,
}

/// Props for [`DrugTrendChart`].
#[component]
pub fn DrugTrendChart(
  /// Which database panel this chart belongs to.
  side: Side,
  /// The monthly trend to render (`None` = empty state).
  data: RwSignal<Option<ChartSeries>>,
  /// Whether a chart fetch is in flight (shows the skeleton).
  loading: RwSignal<bool>,
) -> impl IntoView {
  let canvas_ref = NodeRef::<Canvas>::new();
  let tooltip_ref = NodeRef::<Div>::new();
  let layout_slot: Rc<RefCell<Option<Layout>>> = Rc::new(RefCell::new(None));
  // Hovered month index; also drives the hovered-bar emphasis.
  let hover = RwSignal::new(None::<usize>);
  // Bumped on window resize to force a redraw.
  let size = RwSignal::new(());

  let layout_slot_effect = layout_slot.clone();
  Effect::new(move |_| {
    let series = data.get();
    let hovered = hover.get();
    let _ = size.get();
    if let Some(canvas) = canvas_ref.get() {
      draw_chart(&canvas, side, series.as_ref(), hovered, &layout_slot_effect);
    }
  });

  // First draw + the window-resize listener once the canvas is mounted.
  canvas_ref.on_load(move |_canvas| {
    if let Some(win) = web_sys::window() {
      let size = size;
      let on_resize = Closure::wrap(Box::new(move || {
        size.set(());
      }) as Box<dyn FnMut()>);
      // Intentionally leaked: the listener lives for the whole app.
      let _ = win.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
      on_resize.forget();
    }
    size.set(());
  });

  let on_mousemove = {
    let canvas_ref = canvas_ref;
    let tooltip_ref = tooltip_ref;
    let layout_slot = layout_slot.clone();
    let hover = hover;
    let data = data;
    let side = side;
    move |ev: MouseEvent| {
      let x = ev.offset_x() as f64;
      let y = ev.offset_y() as f64;
      let Some(layout) = *layout_slot.borrow() else { return };
      if x < layout.left || x > layout.right || y < layout.top || y > layout.bottom {
        hover.set(None);
        return;
      }
      let idx = (((x - layout.left) / layout.band) as usize).min(11);
      hover.set(Some(idx));

      let Some(series) = data.get_untracked() else { return };
      let Some(tip) = tooltip_ref.get() else { return };
      let val = series.values().get(idx).copied().unwrap_or(0.0);
      let total = series.total().max(1.0);
      let pct = format!("{:.1}", val / total * 100.0);
      let (label, formatted) = if series.is_value() {
        ("มูลค่า", format_baht(val, 0))
      } else {
        ("จำนวน", format_qty(val))
      };
      let bar_color = css_var(
        if side == Side::Hosxp {
          "--chart-hosxp"
        } else {
          "--chart-invs"
        },
        if side == Side::Hosxp { "#7132f5" } else { "#149e61" },
      );
      let tooltip_bg = css_var(
        if side == Side::Hosxp {
          "--chart-hosxp-tooltip-bg"
        } else {
          "--chart-invs-tooltip-bg"
        },
        "#1a1040",
      );
      let html = format!(
        "<span class=\"chart-tooltip-label\">{}</span><br/>{label}: <span class=\"chart-tooltip-value\" style=\"color:{bar_color}\">{formatted}</span> ({pct}%)",
        series.months()[idx]
      );
      tip.set_inner_html(&html);
      let style = HtmlElement::style(&tip);
      let _ = style.set_property("background", &tooltip_bg);
      let _ = style.set_property("border-color", &bar_color);
      let _ = style.set_property("display", "block");
      let css_w = canvas_ref
        .get()
        .map(|c| c.client_width() as f64)
        .unwrap_or(0.0);
      let mut px = x + 14.0;
      if px + 180.0 > css_w {
        px = (x - 180.0).max(0.0);
      }
      let py = (y - 12.0).max(8.0);
      let _ = style.set_property("left", &format!("{px}px"));
      let _ = style.set_property("top", &format!("{py}px"));
    }
  };

  let on_mouseleave = move |_ev: MouseEvent| {
    hover.set(None);
    if let Some(tip) = tooltip_ref.get() {
      let _ = HtmlElement::style(&tip).set_property("display", "none");
    }
  };

  view! {
      <div class="chart-container">
          <div class="chart-header">
              <div class="chart-title-group">
                  <Show when=move || data.get().is_some()>
                      <span class="chart-drug-code font-mono">
                          {move || {
                              data.get()
                                  .as_ref()
                                  .map(|d| d.code().to_owned())
                                  .unwrap_or_default()
                          }}
                      </span>
                  </Show>
                  <span class="chart-title">
                      {move || {
                          data.get()
                              .as_ref()
                              .map(|d| d.name().to_owned())
                              .unwrap_or_else(|| "เลือกรายการยาเพื่อดูแนวโน้ม".to_owned())
                      }}
                  </span>
              </div>
              <Show when=move || data.get().is_some()>
                  <div class="chart-total">
                      {move || if side == Side::Hosxp { "รวมทั้งปี:" } else { "มูลค่ารวม:" }}
                      <span class="chart-total-value">
                          {move || {
                              data.get()
                                  .as_ref()
                                  .map(|d| {
                                      if d.is_value() {
                                          format_baht(d.total(), 0)
                                      } else {
                                          format_qty(d.total())
                                      }
                                  })
                                  .unwrap_or_default()
                          }}
                      </span>
                  </div>
              </Show>
          </div>

          <div class="chart-stage" style="position:relative;flex:1;min-height:0">
              <canvas
                  class="chart-canvas"
                  node_ref=canvas_ref
                  on:mousemove=on_mousemove
                  on:mouseleave=on_mouseleave
              ></canvas>

              <Show when=move || loading.get()>
                  <div class="chart-loading" style="position:absolute;inset:0;background:var(--bg-base)">
                      <div class="skeleton" style="width:100%;height:100%;border-radius:8px"></div>
                  </div>
              </Show>

              <Show when=move || { !loading.get() && data.get().is_none() }>
                  <div class="chart-empty" style="position:absolute;inset:0">
                      <Icon kind=IconKind::BarChart2 class="chart-empty-icon" size=40 />
                      <p>
                          "คลิกชื่อยาในรายการทางซ้าย"
                          <br/>
                          "หรือค้นหายาเพื่อดูกราฟแนวโน้ม"
                      </p>
                  </div>
              </Show>

              <div class="chart-tooltip" style="display:none" node_ref=tooltip_ref></div>
          </div>
      </div>
  }
}

/// Read a CSS custom property from `:root`, falling back to `fallback`.
fn css_var(name: &str, fallback: &str) -> String {
  let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
    return fallback.to_owned();
  };
  let Some(root) = doc.document_element() else {
    return fallback.to_owned();
  };
  let Some(win) = doc.default_view() else {
    return fallback.to_owned();
  };
  let Ok(Some(styles)) = win.get_computed_style(&root) else {
    return fallback.to_owned();
  };
  styles
    .get_property_value(name)
    .unwrap_or_else(|_| fallback.to_owned())
}

/// Round `rough` up to a "nice" axis step (1 / 2 / 5 × power of ten).
fn nice_step(rough: f64) -> f64 {
  let p = 10f64.powf(rough.max(1e-9).log10().floor());
  let f = rough / p;
  let n = if f < 1.5 {
    1.0
  } else if f < 3.0 {
    2.0
  } else if f < 7.0 {
    5.0
  } else {
    10.0
  };
  n * p
}

/// Compact y-axis label: `1.5K` for quantities, `฿1.2M` / `฿3K` / `฿80`
/// for values (same rules as the original ECharts formatter).
fn fmt_y(v: f64, side: Side) -> String {
  match side {
    Side::Hosxp => {
      if v >= 1000.0 {
        format!("{:.1}K", v / 1000.0)
      } else {
        format!("{}", v.round() as i64)
      }
    }
    Side::Invs => {
      if v >= 1_000_000.0 {
        format!("฿{:.1}M", v / 1_000_000.0)
      } else if v >= 1000.0 {
        format!("฿{}K", (v / 1000.0).round() as i64)
      } else {
        format!("฿{}", v.round() as i64)
      }
    }
  }
}

/// Redraw the whole chart onto `canvas`.
fn draw_chart(
  canvas: &web_sys::HtmlCanvasElement,
  side: Side,
  series: Option<&ChartSeries>,
  hover: Option<usize>,
  layout_slot: &RefCell<Option<Layout>>,
) {
  let css_w = canvas.client_width().max(1) as f64;
  let css_h = canvas.client_height().max(1) as f64;
  let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
  canvas.set_width((css_w * dpr).round() as u32);
  canvas.set_height((css_h * dpr).round() as u32);

  let Ok(Some(obj)) = canvas.get_context("2d") else { return };
  let Ok(ctx) = obj.dyn_into::<CanvasRenderingContext2d>() else { return };
  let _ = ctx.scale(dpr, dpr);
  ctx.clear_rect(0.0, 0.0, css_w, css_h);

  let (vals, months, _total) = match series {
    Some(s) => (s.values().to_vec(), s.months(), s.total().max(0.0)),
    None => (vec![0.0; 12], &THAI_MONTHS_SHORT, 0.0),
  };

  // 3-month trailing moving average (same window as the original).
  let avg: Vec<f64> = vals
    .iter()
    .enumerate()
    .map(|(i, _)| {
      let start = i.saturating_sub(2);
      vals[start..=i].iter().sum::<f64>() / (i - start + 1) as f64
    })
    .collect();

  let max_val = vals.iter().chain(avg.iter()).fold(0.0f64, |m, v| m.max(*v));
  let step = nice_step(max_val.max(1.0) / 4.0);
  let max_tick = ((max_val / step).ceil() * step).max(step);
  let tick_count = (max_tick / step).round() as i32;

  let bar_color = css_var(
    if side == Side::Hosxp {
      "--chart-hosxp"
    } else {
      "--chart-invs"
    },
    if side == Side::Hosxp { "#7132f5" } else { "#149e61" },
  );
  let bar_light = if side == Side::Hosxp {
    "rgba(113,50,245,0.3)"
  } else {
    "rgba(20,158,97,0.3)"
  };
  let line_color = css_var(
    if side == Side::Hosxp {
      "--chart-hosxp-line"
    } else {
      "--chart-invs-line"
    },
    if side == Side::Hosxp { "#5741d8" } else { "#026b3f" },
  );
  let text_secondary = css_var("--text-secondary", "#686b82");
  let text_muted = css_var("--text-muted", "#9497a9");

  let font = "10px 'IBM Plex Mono', 'Courier New', monospace";
  let _ = ctx.set_font(font);

  let mut label_w = 0.0f64;
  for k in 0..=tick_count {
    let label = fmt_y(k as f64 * step, side);
    if let Ok(metrics) = ctx.measure_text(&label) {
      label_w = label_w.max(metrics.width());
    }
  }

  let left = label_w + 8.0;
  let top = 10.0;
  let right = (css_w - 12.0).max(left + 1.0);
  let bottom = (css_h - 24.0).max(top + 1.0);
  let band = (right - left) / 12.0;

  // Dashed horizontal grid lines + right-aligned y labels.
  let _ = ctx.set_stroke_style_str("rgba(104,107,130,0.08)");
  let _ = ctx.set_fill_style_str(&text_muted);
  let _ = ctx.set_text_align("right");
  let _ = ctx.set_text_baseline("middle");
  let dash = js_sys::Array::new();
  dash.push(&JsValue::from_f64(4.0));
  dash.push(&JsValue::from_f64(4.0));
  let _ = ctx.set_line_dash(&dash);
  for k in 0..=tick_count {
    let v = k as f64 * step;
    let y = bottom - (v / max_tick) * (bottom - top);
    ctx.begin_path();
    ctx.move_to(left, y);
    ctx.line_to(right, y);
    ctx.stroke();
    let _ = ctx.fill_text(&fmt_y(v, side), left - 6.0, y);
  }
  let _ = ctx.set_line_dash(&js_sys::Array::new());
  let _ = ctx.set_stroke_style_str("rgba(104,107,130,0.15)");
  ctx.begin_path();
  ctx.move_to(left, bottom);
  ctx.line_to(right, bottom);
  ctx.stroke();

  // Bars — gradient from the bar colour to a light tint, solid when hovered.
  for (i, v) in vals.iter().enumerate() {
    let x_center = left + band * (i as f64 + 0.5);
    let w = band.min(72.0) * 0.6;
    let h = (v / max_tick) * (bottom - top);
    let x = x_center - w / 2.0;
    let y = bottom - h;
    let _ = if hover == Some(i) {
      ctx.set_fill_style_str(&bar_color)
    } else {
      let grad = ctx.create_linear_gradient(0.0, y, 0.0, bottom);
      let _ = grad.add_color_stop(0.0, &bar_color);
      let _ = grad.add_color_stop(1.0, bar_light);
      ctx.set_fill_style_canvas_gradient(&grad)
    };
    ctx.fill_rect(x, y, w, h);
  }

  // 3-month moving-average line + points.
  let _ = ctx.set_stroke_style_str(&line_color);
  let _ = ctx.set_line_width(2.0);
  ctx.begin_path();
  let mut first = true;
  for (i, a) in avg.iter().enumerate() {
    let x = left + band * (i as f64 + 0.5);
    let y = bottom - (a / max_tick) * (bottom - top);
    if first {
      ctx.move_to(x, y);
      first = false;
    } else {
      ctx.line_to(x, y);
    }
  }
  ctx.stroke();
  let _ = ctx.set_fill_style_str(&line_color);
  for (i, a) in avg.iter().enumerate() {
    let x = left + band * (i as f64 + 0.5);
    let y = bottom - (a / max_tick) * (bottom - top);
    ctx.begin_path();
    let _ = ctx.arc(x, y, 2.5, 0.0, PI * 2.0);
    ctx.fill();
  }

  // Month labels under the x-axis.
  let _ = ctx.set_fill_style_str(&text_secondary);
  let _ = ctx.set_text_align("center");
  let _ = ctx.set_text_baseline("top");
  for (i, month) in months.iter().enumerate() {
    let x = left + band * (i as f64 + 0.5);
    let _ = ctx.fill_text(month, x, bottom + 6.0);
  }

  layout_slot.replace(Some(Layout {
    left,
    top,
    right,
    bottom,
    band,
  }));
}
