//! Inline SVG icons.
//!
//! The original Vue frontend used the `lucide-vue-next` icon set; with no JS
//! bundler these are re-implemented as inline SVGs using the same (MIT)
//! lucide path data, keeping the identical visual style (24×24 view box,
//! `currentColor` strokes, 2px stroke width).

use leptos::prelude::*;

/// A single shape inside an icon.
enum Shape {
  Path(&'static str),
  Circle(f64, f64, f64),
  Ellipse(f64, f64, f64, f64),
}

/// Which lucide icon to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
  AlertTriangle,
  Banknote,
  BarChart2,
  Check,
  Database,
  Eye,
  EyeOff,
  Link2,
  Package,
  Pill,
  PlugZap,
  Save,
  Search,
  Settings,
  Settings2,
  Upload,
  X,
  XCircle,
}

fn shapes(kind: IconKind) -> &'static [Shape] {
  match kind {
    IconKind::AlertTriangle => &[
      Shape::Path("m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"),
      Shape::Path("M12 9v4"),
      Shape::Path("M12 17h.01"),
    ],
    IconKind::Banknote => &[
      Shape::Path("M2 8a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z"),
      Shape::Path("M6 12h.01"),
      Shape::Path("M18 12h.01"),
      Shape::Circle(12.0, 12.0, 2.0),
    ],
    IconKind::BarChart2 => &[
      Shape::Path("M18 20V10"),
      Shape::Path("M12 20V4"),
      Shape::Path("M6 20v-6"),
    ],
    IconKind::Check => &[Shape::Path("M20 6 9 17l-5-5")],
    IconKind::Database => &[
      Shape::Ellipse(12.0, 5.0, 9.0, 3.0),
      Shape::Path("M3 5V19A9 3 0 0 0 21 19V5"),
      Shape::Path("M3 12A9 3 0 0 0 21 12"),
    ],
    IconKind::Eye => &[
      Shape::Path("M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"),
      Shape::Circle(12.0, 12.0, 3.0),
    ],
    IconKind::EyeOff => &[
      Shape::Path("M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"),
      Shape::Path("M14.084 14.158a3 3 0 0 1-4.242-4.242"),
      Shape::Path("M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"),
      Shape::Path("m2 2 20 20"),
    ],
    IconKind::Link2 => &[
      Shape::Path("M9 17H7A5 5 0 0 1 7 7h2"),
      Shape::Path("M15 7h2a5 5 0 1 1 0 10h-2"),
      Shape::Path("M8 12h8"),
    ],
    IconKind::Package => &[
      Shape::Path("M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z"),
      Shape::Path("M12 22V12"),
      Shape::Path("M3.29 7 12 12l8.71-5.73"),
      Shape::Path("m7.5 4.27 9 5.15"),
    ],
    IconKind::Pill => &[
      Shape::Path("m10.5 20.5 10-10a4.95 4.95 0 1 0-7-7l-10 10a4.95 4.95 0 1 0 7 7Z"),
      Shape::Path("m8.5 8.5 7 7"),
    ],
    IconKind::PlugZap => &[
      Shape::Path("M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18"),
      Shape::Path("M13.5 2.5 4 12h6l-1.5 8 9.5-9.5H12l1.5-8z"),
    ],
    IconKind::Save => &[
      Shape::Path("M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"),
      Shape::Path("M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"),
      Shape::Path("M7 3v4a1 1 0 0 0 1 1h7"),
    ],
    IconKind::Search => &[
      Shape::Path("m21 21-4.3-4.3"),
      Shape::Circle(11.0, 11.0, 8.0),
    ],
    IconKind::Settings => &[
      Shape::Path("M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"),
      Shape::Circle(12.0, 12.0, 3.0),
    ],
    IconKind::Settings2 => &[
      Shape::Path("M20 7h-9"),
      Shape::Path("M14 17H5"),
      Shape::Circle(17.0, 17.0, 3.0),
      Shape::Circle(7.0, 7.0, 3.0),
    ],
    IconKind::Upload => &[
      Shape::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
      Shape::Path("m17 8-5-5-5 5"),
      Shape::Path("M12 3v12"),
    ],
    IconKind::X => &[
      Shape::Path("M18 6 6 18"),
      Shape::Path("m6 6 12 12"),
    ],
    IconKind::XCircle => &[
      Shape::Circle(12.0, 12.0, 10.0),
      Shape::Path("m15 9-6 6"),
      Shape::Path("m9 9 6 6"),
    ],
  }
}

/// Render a lucide-style icon at `size` pixels, stroked with `currentColor`.
#[component]
pub fn Icon(
  kind: IconKind,
  #[prop(into, default = String::new())] class: String,
  #[prop(default = 14)] size: u16,
) -> impl IntoView {
  let icon_shapes = shapes(kind);
  view! {
      // Note: SVG-element attributes are passed verbatim by the Leptos 0.8
      // view! macro (no snake_case → kebab-case conversion), so the exact
      // DOM spellings (`viewBox`, `stroke-width`, …) must be used here.
      <svg
          xmlns="http://www.w3.org/2000/svg"
          width=size
          height=size
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class=class
          aria-hidden="true"
      >
          {icon_shapes
              .iter()
              .map(|s| match s {
                  Shape::Path(d) => view! { <path d=*d></path> }.into_any(),
                  Shape::Circle(cx, cy, r) => {
                      view! { <circle cx=*cx cy=*cy r=*r></circle> }.into_any()
                  }
                  Shape::Ellipse(cx, cy, rx, ry) => {
                      view! { <ellipse cx=*cx cy=*cy rx=*rx ry=*ry></ellipse> }.into_any()
                  }
              })
              .collect_view()}
      </svg>
  }
}
