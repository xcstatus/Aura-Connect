pub mod form;
pub mod modal;
pub mod picker;

pub use form::quick_connect_new_form;
pub use modal::{quick_connect_modal_stack, quick_connect_panel_content};
pub use picker::quick_connect_picker;

use crate::session::{SessionProfile, TransportConfig};

/// Groups SSH profiles by folder, returning Vec<(folder_name, profiles)>.
pub fn grouped_ssh_profiles(profiles: &[SessionProfile]) -> Vec<(String, Vec<SessionProfile>)> {
    let mut default_v: Vec<SessionProfile> = Vec::new();
    let mut groups: std::collections::BTreeMap<String, Vec<SessionProfile>> =
        std::collections::BTreeMap::new();
    for p in profiles {
        let TransportConfig::Ssh(_) = &p.transport else {
            continue;
        };
        match p
            .folder
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            None => default_v.push(p.clone()),
            Some(name) => groups.entry(name).or_default().push(p.clone()),
        }
    }
    default_v.sort_by_key(|s| s.name.to_lowercase());
    for v in groups.values_mut() {
        v.sort_by_key(|s| s.name.to_lowercase());
    }
    let mut out = Vec::new();
    if !default_v.is_empty() {
        out.push(("__default__".to_string(), default_v));
    }
    for (k, v) in groups {
        out.push((k, v));
    }
    out
}
