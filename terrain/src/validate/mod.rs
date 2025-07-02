use std::collections::BTreeMap;
pub mod identifiers;

pub trait Validate {
    fn validate();
}

impl Validate for &BTreeMap<String, String> {
    fn validate() {}
}
