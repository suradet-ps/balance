//! Match-status chip shown on each dashboard panel (Phase 1).
//!
//! Once a drug is selected on a panel, the chip shows whether it is linked
//! to the other system ("↔ INVS: <working_code>"), marked as having no INVS
//! equivalent, or still unmapped — so the pharmacist sees the link without
//! opening the mapping drawer.  Data comes from [`MappingContext`], refreshed
//! whenever a drug is selected.

use leptos::prelude::*;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::MappingContext;
use crate::models::Side;

/// Props for [`MappingStatusChip`].
#[component]
pub fn MappingStatusChip(side: Side) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();

  let link = move || match side {
    Side::Hosxp => mapping.hosxp_link.get(),
    Side::Invs => mapping.invs_link.get(),
  };

  let status = move || link().map_or(String::new(), |s| s.status);
  let is_mapped = move || status() == "mapped";
  let is_no_invs = move || status() == "no_invs";

  view! {
      <Show when=move || link().is_some()>
          <div class="mapping-chip">
              <Show when=is_mapped>
                  <span class="badge badge-connected">
                      <Icon kind=IconKind::Link2 size=12 />
                      {move || {
                          let s = link().unwrap();
                          match side {
                              Side::Hosxp => format!(
                                  "แมปแล้ว ↔ INVS: {}",
                                  s.link.map_or(String::new(), |l| l.working_code)
                              ),
                              Side::Invs => format!(
                                  "แมปแล้ว ↔ HOSxP: {}",
                                  s.link.map_or(String::new(), |l| l.icode)
                              ),
                          }
                      }}
                  </span>
              </Show>

              <Show when=move || is_no_invs() && side == Side::Hosxp>
                  <span
                      class="badge badge-muted"
                      title=move || {
                          link().map_or(String::new(), |s| s.reason.unwrap_or_default())
                      }
                  >
                      <Icon kind=IconKind::XCircle size=12 />
                      "ไม่มีใน INVS"
                  </span>
              </Show>

              <Show when=move || { status() == "unmapped" }>
                  <span class="badge badge-unmapped">"ยังไม่แมป"</span>
              </Show>
          </div>
      </Show>
  }
}
