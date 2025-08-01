//! Structs that allow setting some options to initialize the app
//! with some predefined values.

use std::{ops::Deref, sync::Arc};

use naviz_import::ImportOptions;
use serde::{Deserialize, Serialize};

/// Something that is either specified by an `id` from the [Repository]
/// or manually loaded.
#[derive(Serialize, Deserialize)]
pub enum IdOrManual<ID, MAN> {
    Id(ID),
    Manual(MAN),
}

impl<ID, MAN> IdOrManual<ID, MAN> {
    /// Maps the `id` or `manual` values contained using their respective mappers.
    pub fn map<IO, MO>(
        self,
        id: impl FnOnce(ID) -> IO,
        manual: impl FnOnce(MAN) -> MO,
    ) -> IdOrManual<IO, MO> {
        match self {
            Self::Id(v) => IdOrManual::Id(id(v)),
            Self::Manual(v) => IdOrManual::Manual(manual(v)),
        }
    }

    /// References the contained `id`/`manual`
    pub fn as_ref(&self) -> IdOrManual<&ID, &MAN> {
        match self {
            Self::Id(v) => IdOrManual::Id(v),
            Self::Manual(v) => IdOrManual::Manual(v),
        }
    }
}

impl<ID: Deref, MAN: Deref> IdOrManual<ID, MAN> {
    /// Dereferences the contained `id`/`manual`
    pub fn as_deref(&self) -> IdOrManual<&<ID as Deref>::Target, &<MAN as Deref>::Target> {
        match self {
            Self::Id(v) => IdOrManual::Id(v),
            Self::Manual(v) => IdOrManual::Manual(v),
        }
    }
}

type IdOrManualInit<'a> = IdOrManual<&'a str, &'a [u8]>;
type IdOrManualPersistence = IdOrManual<String, Arc<[u8]>>;

/// Options to start the app with.
/// Leave [None] to keep unset or set to [Some] value to initialize with the value.
/// Can be passed to [App::new_with_init][crate::App::new_with_init].
#[derive(Default)]
pub struct InitOptions<'a> {
    /// The machine to load
    pub machine: Option<IdOrManualInit<'a>>,
    /// The style to load
    pub style: Option<IdOrManualInit<'a>>,
    /// The visualization input to load.
    /// Pass [Some] [ImportOptions] if the content needs to be imported.
    pub input: Option<(Option<ImportOptions>, &'a [u8])>,
}

impl InitOptions<'_> {
    /// Merges two [InitOptions].
    /// Values in `other` take precedence.
    pub fn merge(self, other: Self) -> Self {
        Self {
            machine: other.machine.or(self.machine),
            style: other.style.or(self.style),
            input: other.input.or(self.input),
        }
    }
}

/// Persistent values stored by the application
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct Persistence {
    /// The last used machine
    pub machine: Option<IdOrManualPersistence>,
    /// The last used style
    pub style: Option<IdOrManualPersistence>,
}

impl Persistence {
    /// Loads persisted values from the passed [eframe::CreationContext].
    /// Will return [None] if nothing was previously persisted.
    pub fn load(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        cc.storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
    }
}

impl<'a> From<&'a Persistence> for InitOptions<'a> {
    fn from(value: &'a Persistence) -> Self {
        Self {
            machine: value.machine.as_ref().map(IdOrManual::as_deref),
            style: value.style.as_ref().map(IdOrManual::as_deref),
            ..Default::default()
        }
    }
}
