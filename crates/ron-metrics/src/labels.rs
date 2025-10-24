//! RO:WHAT — Base label helpers (service, instance, build_version, amnesia).

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BaseLabels {
    pub service: String,
    pub instance: String,
    pub build_version: String,
    pub amnesia: String,
}

impl BaseLabels {
    pub fn to_const_labels(&self) -> HashMap<String, String> {
        let mut m = HashMap::with_capacity(4);
        m.insert("service".into(), self.service.clone());
        m.insert("instance".into(), self.instance.clone());
        m.insert("build_version".into(), self.build_version.clone());
        m.insert("amnesia".into(), self.amnesia.clone());
        m
    }
}
