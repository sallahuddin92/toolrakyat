use crate::syntax::object::ObjectRef;

/// Widget annotation associated with an interactive form field.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetAnnotation {
    pub object_ref: ObjectRef,
    pub page_index: Option<usize>,
    pub rect: [f64; 4],
    pub appearance_state: Option<String>,
    pub normal_appearance_states: Vec<String>,
    pub has_normal_appearance: bool,
    pub has_rollover_appearance: bool,
    pub has_down_appearance: bool,
    pub flags: u32,
    pub parent_ref: Option<ObjectRef>,
}

impl WidgetAnnotation {
    /// Returns the "on" appearance state name (the non-"Off" state), if defined in `/AP /N`.
    pub fn on_state_name(&self) -> Option<&str> {
        self.normal_appearance_states
            .iter()
            .find(|s| s.as_str() != "Off")
            .map(|s| s.as_str())
    }

    /// Checks whether the current appearance state corresponds to the checked / on state.
    pub fn is_checked(&self) -> bool {
        match (&self.appearance_state, self.on_state_name()) {
            (Some(as_state), Some(on_state)) => as_state.as_str() == on_state,
            (Some(as_state), None) => as_state.as_str() != "Off",
            _ => false,
        }
    }
}
