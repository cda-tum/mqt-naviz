/// The currently selected machine
pub enum CurrentMachine {
    /// No machine selected
    None,
    /// Manually opened machine
    Manual,
    /// Machine with the specified `id` is opened
    Id(String),
}

impl Default for CurrentMachine {
    fn default() -> Self {
        Self::None
    }
}

impl CurrentMachine {
    /// Check if the currently opened machine is compatible with any of the passed machine ids.
    /// If this returns `false`, a new machine should be selected.
    /// If this returns `true`, the current machine can be kept.
    pub fn compatible_with(&self, compatible_machines: &[String]) -> bool {
        match self {
            Self::None => false,  // No machine loaded: change machine
            Self::Manual => true, // Machine manually opened: keep
            Self::Id(id) => compatible_machines.contains(id),
        }
    }

    /// Gets the `id`` of this [CurrentMachine] if it is set to [CurrentMachine::Id].
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Id(id) => Some(id),
            _ => None,
        }
    }
}
