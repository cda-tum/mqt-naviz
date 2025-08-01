//! [Repository] for loading configs.
//! Also contains bundled configs.

#[cfg(test)]
use std::cell::RefCell;
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};

#[cfg(not(test))]
use directories::ProjectDirs;
use error::{Error, Result};
use include_dir::{include_dir, Dir};
use naviz_parser::config::{generic::Config, machine::MachineConfig, visual::VisualConfig};
#[cfg(test)]
use tempfile::TempDir;

pub mod error;

static BUNDLED_MACHINES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../configs/machines");
static BUNDLED_STYLES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../configs/styles");

const MACHINES_SUBDIR: &str = "machines";
const STYLES_SUBDIR: &str = "styles";

/// A repository of config files.
pub struct Repository(HashMap<String, RepositoryEntry>);

/// The project directories for this application
#[cfg(not(test))]
fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("tv", "floeze", "naviz").ok_or(Error::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Failed to get user directory",
    )))
}

/// Creates a new temporary directory.
/// Can be used for testing.
#[cfg(test)]
fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

#[cfg(test)]
thread_local! {
    /// Temporary directory for testing.
    /// Use [reset_temp_dir] to reset the contents of the directory.
    static TEMP_DIR: RefCell<TempDir> = RefCell::new(create_temp_dir());
}

/// Creates a new [TEMP_DIR].
/// Will delete the old [TEMP_DIR] and all contents.
#[cfg(test)]
fn reset_temp_dir() {
    TEMP_DIR.replace(create_temp_dir());
}

impl Repository {
    /// Creates a new empty repository
    pub fn empty() -> Self {
        Self(Default::default())
    }

    /// Loads the passed bundled config into the passed [Repository]
    fn load_bundled(mut self, bundled: &Dir<'static>) -> Result<Self> {
        self.0 = insert_results(
            self.0,
            bundled.files().map(|f| {
                RepositoryEntry::new_with_id(
                    f.path()
                        .file_stem()
                        .ok_or(Error::IdError)?
                        .to_string_lossy()
                        .into_owned(),
                    RepositorySource::Bundled(f.contents()),
                )
            }),
        )?;
        Ok(self)
    }

    /// Loads the bundled machines into the passed [Repository]
    pub fn bundled_machines(self) -> Result<Self> {
        self.load_bundled(&BUNDLED_MACHINES)
    }

    /// Loads the bundles styles into the passed [Repository]
    pub fn bundled_styles(self) -> Result<Self> {
        self.load_bundled(&BUNDLED_STYLES)
    }

    /// Gets the path of the passed `subdir` of the user-directory
    #[cfg(not(test))]
    fn user_dir(subdir: &str) -> Result<PathBuf> {
        let directory = project_dirs()?.data_dir().join(subdir);

        if !directory.exists() {
            fs::create_dir_all(&directory).map_err(Error::IoError)?;
        }

        Ok(directory)
    }

    /// Gets the path of the passed `subdir` of the user-directory.
    /// Uses [TEMP_DIR] for testing.
    #[cfg(test)]
    fn user_dir(subdir: &str) -> Result<PathBuf> {
        let directory = TEMP_DIR.with_borrow(|t| t.path().join(subdir));

        if !directory.exists() {
            fs::create_dir_all(&directory).map_err(Error::IoError)?;
        }

        Ok(directory)
    }

    /// Loads the configs from the passed `subdir` of the user-directory
    /// into the passed [Repository]
    fn load_user_dir(mut self, subdir: &str) -> Result<Self> {
        self.0 = insert_results(
            self.0,
            Self::user_dir(subdir)?
                .read_dir()
                .map_err(Error::IoError)?
                .filter_map(|x| {
                    if let Ok(x) = x {
                        if x.path().is_file() {
                            Some(x.path())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .map(|p| {
                    RepositoryEntry::new_with_id(
                        p.file_stem()
                            .ok_or(Error::IdError)?
                            .to_string_lossy()
                            .into_owned(),
                        RepositorySource::UserDir(p),
                    )
                }),
        )?;
        Ok(self)
    }

    /// Loads the machines from the user-directory into the passed [Repository]
    pub fn user_dir_machines(self) -> Result<Self> {
        self.load_user_dir(MACHINES_SUBDIR)
    }

    /// Loads the styles from the user-directory into the passed [Repository]
    pub fn user_dir_styles(self) -> Result<Self> {
        self.load_user_dir(STYLES_SUBDIR)
    }

    /// Imports a `file` into the passed `subdir` in the user-directory.
    /// Will validate that the config can be parsed into a valid `C`.
    fn import_to_user_dir<C>(&mut self, subdir: &str, file: &Path) -> Result<()>
    where
        Config: TryInto<C, Error = naviz_parser::config::error::Error>,
    {
        if !file.is_file() {
            return Err(Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File to import is not a file.",
            )));
        }

        let id = file
            .file_stem()
            .ok_or(Error::IdError)?
            .to_string_lossy()
            .into_owned();
        // Create temporary entry with the source path to check if the config is valid
        let entry = RepositoryEntry::new(RepositorySource::UserDir(file.to_owned()))?;
        // Ensure the config is valid (i.e., can be parsed correctly)
        entry
            .contents_as_config()?
            .try_into()
            .map_err(Error::ConfigReadError)?;

        // Import: Copy to target path
        let target_path = Self::user_dir(subdir)?.join(file.file_name().unwrap());
        fs::copy(file, &target_path).map_err(Error::IoError)?;

        self.0.insert(
            id,
            // New repository entry with correct target path
            RepositoryEntry::new(RepositorySource::UserDir(target_path))?,
        );

        Ok(())
    }

    /// Import a machine into the user-directory.
    /// Will validate that the config can be parsed into a valid [MachineConfig].
    pub fn import_machine_to_user_dir(&mut self, file: &Path) -> Result<()> {
        self.import_to_user_dir::<MachineConfig>(MACHINES_SUBDIR, file)
    }

    /// Import a style into the user-directory.
    /// Will validate that the config can be parsed into a valid [VisualConfig].
    pub fn import_style_to_user_dir(&mut self, file: &Path) -> Result<()> {
        self.import_to_user_dir::<VisualConfig>(STYLES_SUBDIR, file)
    }

    /// Delete an imported config from the user dir.
    pub fn remove_from_user_dir(&mut self, id: &str) -> Result<()> {
        let (id, entry) = self.0.remove_entry(id).ok_or(Error::IdError)?;

        let RepositorySource::UserDir(path) = entry.source else {
            // Not imported from user-dir
            // => add back entry and return error
            self.0.insert(id, entry);
            return Err(Error::NotRemovableError);
        };

        fs::remove_file(path).map_err(Error::IoError)?;

        Ok(())
    }

    /// The list of entries of this repository: `(id, name, removable)`-pairs
    pub fn list(&self) -> Vec<(&str, &str, bool)> {
        self.0
            .iter()
            .map(|(id, entry)| (id.as_str(), entry.name(), entry.source.is_removable()))
            .collect()
    }

    /// Checks whether the repository has an entry with `id`
    pub fn has(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    /// Tries to get the raw contents of the entry with the passed `id`.
    ///
    /// Returns:
    /// - `None`: No entry with the passed `id` exists
    /// - `Some(Err)`: An entry exists, but failed to load the data
    /// - `Some(Ok)`: The data of the found entry
    pub fn get_raw(&self, id: &str) -> Option<Result<Cow<[u8]>>> {
        self.0.get(id).map(|e| e.contents())
    }

    /// Tries to get the contents of the entry with the passed `id` as some [Config].
    ///
    /// Returns:
    /// - `None`: No entry with the passed `id` exists
    /// - `Some(Err)`: An entry exists, but failed to load the data or failed to convert to `C`
    /// - `Some(Ok)`: The config of the found entry
    pub fn get<C>(&self, id: &str) -> Option<Result<C>>
    where
        Config: TryInto<C, Error = naviz_parser::config::error::Error>,
    {
        self.0.get(id).map(|e| {
            e.contents_as_config()?
                .try_into()
                .map_err(Error::ConfigReadError)
        })
    }

    /// Try to get any config from this repository
    pub fn try_get_any<C>(&self) -> Option<(&str, C)>
    where
        Config: TryInto<C>,
    {
        self.0
            .iter()
            .filter_map(|(id, entry)| {
                Some((
                    id.as_str(),
                    entry.contents_as_config().ok()?.try_into().ok()?,
                ))
            })
            .next()
    }
}

/// An entry in the repository.
/// Contains a cached `name`, an `id`, and the `source`.
/// Is hashed and checked for equality only by `id`.
struct RepositoryEntry {
    /// The name as read from the config-file
    name: String,
    /// The source of the entry
    source: RepositorySource,
}

impl RepositoryEntry {
    /// Creates a new [RepositoryEntry] from the passed `source`.
    /// Will also return the `id` for usage as a [HashMap]-entry.
    /// Will extract the name from the `source`.
    fn new_with_id(id: String, source: RepositorySource) -> Result<(String, Self)> {
        Ok((id, Self::new(source)?))
    }

    /// Creates a new [RepositoryEntry] from the passed `source`.
    /// Will extract the name from the `source`.
    fn new(source: RepositorySource) -> Result<Self> {
        Ok(Self {
            name: source.name()?,
            source,
        })
    }

    /// The name of this [RepositoryEntry]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The contents of this [RepositoryEntry]
    pub fn contents(&self) -> Result<Cow<[u8]>> {
        self.source.contents()
    }

    /// The name of this [RepositoryEntry] as a [Config]
    pub fn contents_as_config(&self) -> Result<Config> {
        self.source.contents_as_config()
    }
}

/// A data-source for the repository.
enum RepositorySource {
    /// Bundled in the executable
    Bundled(&'static [u8]),
    /// Stored in the user-directory
    UserDir(PathBuf),
}

impl RepositorySource {
    /// Extract the config-name from this [RepositorySource]
    pub fn name(&self) -> Result<String> {
        naviz_parser::config::generic::get_item(&mut self.contents_as_config()?, "name")
            .map_err(Error::ConfigReadError)
    }

    /// Read the contents of this [RepositorySource]
    pub fn contents(&self) -> Result<Cow<[u8]>> {
        Ok(match self {
            Self::Bundled(c) => Cow::Borrowed(c),
            Self::UserDir(p) => fs::read(p).map(Cow::Owned).map_err(Error::IoError)?,
        })
    }

    /// Get the contents of this [RepositorySource] as a [Config]
    pub fn contents_as_config(&self) -> Result<Config> {
        config_from_bytes(self.contents()?.borrow())
    }

    /// Check whether a [RepositoryEntry] from this [RepositorySource] can be removed.
    pub fn is_removable(&self) -> bool {
        match self {
            Self::Bundled(_) => false,
            Self::UserDir(_) => true,
        }
    }
}

/// Try to parse a [Config] from the passed `bytes`
pub fn config_from_bytes(bytes: &[u8]) -> Result<Config> {
    let config =
        naviz_parser::config::lexer::lex(std::str::from_utf8(bytes).map_err(Error::UTF8Error)?)
            .map_err(Error::lex_error)?;
    let config = naviz_parser::config::parser::parse(&config).map_err(Error::parse_error)?;
    Ok(config.into())
}

/// Insert an [Iterator] of [Result]s into the `target` [HashMap].
///
/// Returns [Ok] with the updated [HashMap] if all [Result]s were [Ok]
/// or the first [Err].
fn insert_results<K: Eq + Hash, V>(
    mut target: HashMap<K, V>,
    source: impl IntoIterator<Item = Result<(K, V)>>,
) -> Result<HashMap<K, V>> {
    for result in source.into_iter() {
        let (key, value) = result?;
        target.insert(key, value);
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use naviz_parser::config::{machine::MachineConfig, visual::VisualConfig};

    use super::*;

    /// Check if all bundled machines can be loaded and parsed successfully.
    #[test]
    fn bundled_machines() {
        let machines = Repository::empty()
            .bundled_machines()
            .expect("Failed to load bundled machines");

        for (id, name, _) in machines.list() {
            machines
                .get::<MachineConfig>(id)
                .expect("Machine exists in `list`, but `get` returned `None`")
                .unwrap_or_else(|e| panic!("Machine \"{name}\" ({id}) is invalid:\n{e:#?}"));
        }
    }

    /// Check if all bundled styles can be loaded and parsed successfully.
    #[test]
    fn bundled_styles() {
        let styles = Repository::empty()
            .bundled_styles()
            .expect("Failed to load bundled styles");

        for (id, name, _) in styles.list() {
            styles
                .get::<VisualConfig>(id)
                .expect("Style exists in `list`, but `get` returned `None`")
                .unwrap_or_else(|e| panic!("Style \"{name}\" ({id}) is invalid:\n{e:#?}"));
        }
    }

    /// Test importing configs from the `subdir` of the bundled configs to the `subdir` of the [TEMP_DIR].
    /// Will use `import_fn` to import the configs to the repo.
    /// Takes care of resetting the [TEMP_DIR].
    fn test_import_configs(subdir: &str, import_fn: impl Fn(&mut Repository, &Path) -> Result<()>) {
        reset_temp_dir();

        // Directory where configs should be imported to
        let target_dir = Repository::user_dir(subdir).expect("Failed to get config subdirectory");

        // sanity-check: should be initially empty
        assert!(
            fs::read_dir(&target_dir)
                .expect("Failed to get contents of config subdirectory")
                .next()
                .is_none(),
            "New temporary config directory is not empty"
        );

        // Use bundled configs as source configs
        let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../configs/{subdir}"));

        let configs: Vec<_> = fs::read_dir(&source_dir)
            .expect("Failed to read bundled configs to import")
            .map(|f| f.expect("Cannot get file"))
            .collect();

        // Create new empty repository
        let mut repo = Repository::empty();

        for config in &configs {
            import_fn(&mut repo, &config.path()).expect("Failed to import config");

            // Check if imported config exists in repo
            assert!(
                repo.has(
                    config
                        .path()
                        .file_stem()
                        .expect("Failed to get id from filename")
                        .to_string_lossy()
                        .borrow()
                ),
                "Imported config does not exist in repository"
            );

            // Check if imported config now exists on disk
            assert!(
                fs::exists(target_dir.join(config.file_name())).unwrap_or(false),
                "Imported config does not exist on disk"
            );
        }

        // Check if correct number of configs was imported
        assert_eq!(
            configs.len(),
            repo.list().len(),
            "Wrong number of files imported in repo"
        );

        // Check if correct number of configs now exists on disk
        assert_eq!(
            configs.len(),
            fs::read_dir(&target_dir)
                .expect("Failed to get contents of config subdirectory")
                .count(),
            "Wrong number of files imported on disk"
        );

        // Check if loading from disk also has the same and correct configs
        let repo_new = Repository::empty()
            .load_user_dir(subdir)
            .expect("Failed to load configs from disk");
        let mut list_old = repo.list();
        let mut list_new = repo_new.list();
        list_old.sort();
        list_new.sort();
        assert_eq!(
            list_old, list_new,
            "Reading imported configs from disk did not give the same configs"
        );
    }

    /// Checks whether the [Repository] can successfully import the bundled machines.
    #[test]
    fn import_machines() {
        test_import_configs(MACHINES_SUBDIR, Repository::import_machine_to_user_dir);
    }

    /// Checks whether the [Repository] can successfully import the bundled styles.
    #[test]
    fn import_styles() {
        test_import_configs(STYLES_SUBDIR, Repository::import_style_to_user_dir);
    }

    /// Should not be able to remove bundled configs.
    #[test]
    fn cannot_remove_bundled_configs() {
        let mut repo = Repository::empty()
            .bundled_machines()
            .expect("Failed to load bundled machines")
            .bundled_styles()
            .expect("Failed to load bundled styles");

        let list: Vec<_> = repo
            .list()
            .into_iter()
            .map(|(id, _name, removable)| (id.to_owned(), removable))
            .collect();

        // sanity-check: repository should not be empty
        assert!(!list.is_empty(), "Did not load any bundled configs");

        for (id, removable) in list {
            // sanity-check: repository should contain the config
            assert!(
                repo.has(&id),
                "Repository does not have the config it claims to have"
            );

            assert!(!removable, "Bundled config is marked as removable");

            assert!(
                repo.remove_from_user_dir(&id).is_err(),
                "Trying to remove a bundled config did not return an error"
            );

            // Config should still exist
            assert!(
                repo.has(&id),
                "Repository deleted a config even though it returned an error"
            );
        }
    }

    /// Test removing imported configs by importing configs from the `subdir` of the bundled configs.
    /// Will use `import_fn` to import the configs to the repo.
    /// Takes care of resetting the [TEMP_DIR].
    /// If `RELOAD` is set, will recreate the repository and load the imported configs from disk.
    fn test_remove_configs<const RELOAD: bool>(
        subdir: &str,
        import_fn: impl Fn(&mut Repository, &Path) -> Result<()>,
    ) {
        reset_temp_dir();

        // Directory where configs should be imported to
        let target_dir = Repository::user_dir(subdir).expect("Failed to get config subdirectory");

        // Use bundled configs as source configs
        let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../configs/{subdir}"));

        let configs: Vec<_> = fs::read_dir(&source_dir)
            .expect("Failed to read bundled configs to import")
            .map(|f| f.expect("Cannot get file"))
            .collect();

        let mut repo = Repository::empty();

        for config in &configs {
            import_fn(&mut repo, &config.path()).expect("Failed to import config");
        }

        if RELOAD {
            repo = Repository::empty()
                .load_user_dir(subdir)
                .expect("Failed load configs from disk");
        }

        assert!(
            repo.list()
                .into_iter()
                .all(|(_id, _name, removable)| removable),
            "Imported configs not marked as removable"
        );

        for config in configs {
            let path = config.path();
            let id = path
                .file_stem()
                .expect("Failed to get id from filename")
                .to_string_lossy();

            // sanity-check: repository should contain the config
            assert!(
                repo.has(&id),
                "Repository does not have the config it claims to have"
            );

            let target_path = target_dir.join(config.file_name());

            // sanity-check: config should exist on disk
            assert!(
                fs::exists(&target_path).unwrap_or(false),
                "Imported config does not exist on disk"
            );

            // remove
            repo.remove_from_user_dir(&id)
                .expect("Failed to remove imported config from user dir");

            assert!(!repo.has(&id), "Repository still contains deleted config");

            assert!(
                !fs::exists(&target_path).unwrap_or(true),
                "Removed config still exists on disk"
            );
        }
    }

    /// Checks whether the [Repository] can successfully remove imported machines.
    #[test]
    fn remove_machines() {
        test_remove_configs::<false>(MACHINES_SUBDIR, Repository::import_machine_to_user_dir);
    }

    /// Checks whether the [Repository] can successfully remove imported machines.
    #[test]
    fn remove_styles() {
        test_remove_configs::<false>(STYLES_SUBDIR, Repository::import_style_to_user_dir);
    }

    /// Checks whether the [Repository] can successfully remove imported machines.
    /// Will first import the machines and then reload the repository.
    #[test]
    fn remove_machines_after_reload() {
        test_remove_configs::<true>(MACHINES_SUBDIR, Repository::import_machine_to_user_dir);
    }

    /// Checks whether the [Repository] can successfully remove imported machines.
    /// Will first import the styles and then reload the repository.
    #[test]
    fn remove_styles_after_reload() {
        test_remove_configs::<true>(STYLES_SUBDIR, Repository::import_style_to_user_dir);
    }
}
