//! Benchmark fixtures for `benches/`. Not part of runa's API; LTO strips it
//! from `rn`.
//!
//! Fixtures wrap `pub(crate)` types so `benches/` can use them:
//! - `new(..)`: setup, not measured.
//! - `input()`: fresh input when `run` consumes or mutates it, not measured.
//! - `run(..)`: the measured call.
//!
//! Nothing here writes to disk.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use dashmap::DashMap;

use crate::app::AppState;
use crate::config::Config;
use crate::core::cache::{DirCache, DirListOptions};
use crate::core::metadata::CachedMetaKey;
use crate::core::sort::{SortConfig, SortMode, SortOrder};
use crate::core::workers::Workers;
use crate::core::{FileEntry, Formatter, fm, formatter};
use crate::utils::text::StrBuffer;

/// Directory the synthetic entries pretend to live in. Never touched on disk.
const SYNTHETIC_DIR: &str = "/bench";

/// Includes the year, so the formatted dates do not change with the current date.
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M";

/// Owned entries, opaque to `benches/`.
pub struct Entries(Vec<FileEntry>);

/// Raw names and flags, the input `FileEntry::new` consumes.
pub type Names = Vec<(OsString, u8)>;

/// Fixed-seed LCG, so every run gets byte-identical input.
struct Lcg(u64);

impl Lcg {
    fn new() -> Self {
        Self(0x2545_F491_4F6C_DD1D)
    }

    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

fn synthetic_names(count: usize) -> Names {
    const EXTS: [&str; 8] = ["rs", "txt", "md", "toml", "png", "log", "tar.gz", ""];

    let mut rng = Lcg::new();
    (0..count)
        .map(|i| {
            let r = rng.next();
            let ext = EXTS[r % EXTS.len()];
            let name = if ext.is_empty() {
                format!("entry_{}_{}", r % 100_000, i)
            } else {
                format!("entry_{}_{}.{}", r % 100_000, i, ext)
            };
            let flags = if r.is_multiple_of(5) {
                FileEntry::IS_DIR
            } else {
                0
            };
            (OsString::from(name), flags)
        })
        .collect()
}

fn build_entries(names: Names) -> Vec<FileEntry> {
    names
        .into_iter()
        .map(|(name, flags)| FileEntry::new(name, flags, None))
        .collect()
}

/// Metadata for the synthetic entries, keyed like `sort_entries` looks it up,
/// so metadata sorts never touch the disk.
fn synthetic_metadata(entries: &[FileEntry]) -> DashMap<PathBuf, CachedMetaKey> {
    const START: u64 = 1_420_070_400; // 2015-01-01
    const SPAN: u64 = 6 * 365 * 24 * 60 * 60;

    let mut rng = Lcg::new();
    let mut time = || Some(UNIX_EPOCH + Duration::from_secs(START + rng.next() as u64 % SPAN));
    let cache = DashMap::with_capacity(entries.len());
    for entry in entries {
        let key = CachedMetaKey {
            size: (!entry.is_dir()).then(|| entry.name_str().len() as u64 * 4_099),
            modified: time(),
            created: time(),
            accessed: time(),
        };
        cache.insert(Path::new(SYNTHETIC_DIR).join(entry.name()), key);
    }
    cache
}

fn list_options() -> DirListOptions {
    DirListOptions {
        dirs_first: true,
        show_hidden: true,
        show_symlink: true,
        show_system: false,
        case_insensitive: true,
    }
}

fn formatter_for(mode: SortMode) -> Formatter {
    Formatter::new(
        list_options(),
        SortConfig::from((mode, SortOrder::Ascending)),
        Arc::new(HashSet::new()),
    )
}

/// Created and accessed are left out: they run the modified code path.
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Name,
    Natural,
    Extension,
    Size,
    Modified,
}

impl SortBy {
    fn mode(self) -> SortMode {
        match self {
            SortBy::Name => SortMode::Name,
            SortBy::Natural => SortMode::Natural,
            SortBy::Extension => SortMode::Extension,
            SortBy::Size => SortMode::Size,
            SortBy::Modified => SortMode::Modified,
        }
    }
}

/// `FileEntry::new`: the per-entry cost of every directory listing.
pub struct EntryFixture {
    names: Names,
}

impl EntryFixture {
    pub fn new(count: usize) -> Self {
        Self {
            names: synthetic_names(count),
        }
    }

    pub fn input(&self) -> Names {
        self.names.clone()
    }

    pub fn run(&self, names: Names) -> Entries {
        Entries(build_entries(names))
    }
}

/// `Formatter::sort_entries` on unsorted synthetic entries.
pub struct SortFixture {
    formatter: Formatter,
    entries: Vec<FileEntry>,
    metadata: DashMap<PathBuf, CachedMetaKey>,
}

impl SortFixture {
    pub fn new(by: SortBy, count: usize) -> Self {
        let entries = build_entries(synthetic_names(count));
        Self {
            formatter: formatter_for(by.mode()),
            metadata: synthetic_metadata(&entries),
            entries,
        }
    }

    pub fn input(&self) -> Entries {
        Entries(self.entries.clone())
    }

    pub fn run(&self, input: &mut Entries) -> usize {
        let column = self.formatter.sort_entries(
            Path::new(SYNTHETIC_DIR),
            &mut input.0,
            DATE_FORMAT,
            &self.metadata,
        );
        input.0.len() + column.map_or(0, |c| c.len())
    }
}

/// `Formatter::filter_entries`, the pass that runs before every sort.
pub struct FilterFixture {
    formatter: Formatter,
    entries: Vec<FileEntry>,
}

impl FilterFixture {
    pub fn new(count: usize) -> Self {
        Self {
            formatter: formatter_for(SortMode::Natural),
            entries: build_entries(synthetic_names(count)),
        }
    }

    pub fn input(&self) -> Entries {
        Entries(self.entries.clone())
    }

    pub fn run(&self, input: &mut Entries) -> usize {
        self.formatter.filter_entries(&mut input.0);
        input.0.len()
    }
}

/// `DirCache` lookup, insertion and eviction.
pub struct CacheFixture {
    cache: DirCache,
    dirs: Vec<PathBuf>,
    entries: Arc<[FileEntry]>,
    sort_column: Option<Arc<StrBuffer>>,
    list: DirListOptions,
    sort: SortConfig,
    next_id: u64,
}

impl CacheFixture {
    /// Fills the cache with `dirs` directories sharing one listing.
    pub fn new(dirs: usize, entries_per_dir: usize) -> Self {
        let entries: Arc<[FileEntry]> = Arc::from(build_entries(synthetic_names(entries_per_dir)));
        let sort_column = Some(Arc::new(StrBuffer::from_iter(
            entries.iter().map(|e| e.name_str()),
        )));

        let mut fixture = Self {
            cache: DirCache::new(),
            dirs: (0..dirs)
                .map(|i| Path::new(SYNTHETIC_DIR).join(format!("dir_{i}")))
                .collect(),
            entries,
            sort_column,
            list: list_options(),
            sort: SortConfig::default(),
            next_id: 0,
        };
        for i in 0..fixture.dirs.len() {
            fixture.insert_at(i);
        }
        fixture
    }

    fn insert_at(&mut self, index: usize) {
        self.next_id += 1;
        self.cache.insert_if_newer(
            &self.dirs[index],
            self.sort,
            &self.list,
            Arc::clone(&self.entries),
            self.sort_column.clone(),
            self.next_id,
        );
    }

    pub fn hit(&self) -> bool {
        let last = self.dirs.len().saturating_sub(1);
        self.cache
            .get(&self.dirs[last], self.sort, &self.list)
            .is_some()
    }

    pub fn miss(&self) -> bool {
        self.cache
            .get(Path::new("/bench/absent"), self.sort, &self.list)
            .is_some()
    }

    /// Re-inserts the directories in turn. At capacity (30) this includes the
    /// eviction scan.
    pub fn insert(&mut self) {
        let index = (self.next_id as usize) % self.dirs.len();
        self.insert_at(index);
    }

    /// Measures the key scan; the key itself is gone after the first run.
    pub fn invalidate(&self) {
        let last = self.dirs.len().saturating_sub(1);
        self.cache.invalidate_path(&self.dirs[last]);
    }
}

/// A directory load as the io worker runs it: browse, filter, sort. Metadata
/// sorts stat every entry, like the worker.
pub struct LoadFixture {
    dir: PathBuf,
    formatter: Formatter,
}

impl LoadFixture {
    pub fn new(dir: &Path, by: SortBy) -> Self {
        Self {
            dir: dir.to_path_buf(),
            formatter: formatter_for(by.mode()),
        }
    }

    /// `browse_dir` alone.
    pub fn browse(&self) -> usize {
        fm::browse_dir(&self.dir).map_or(0, |e| e.len())
    }

    /// The full load.
    pub fn run(&self) -> Entries {
        let Ok(mut entries) = fm::browse_dir(&self.dir) else {
            return Entries(Vec::new());
        };
        let metadata = DashMap::with_capacity(entries.len());
        self.formatter.filter_entries(&mut entries);
        let _ = self
            .formatter
            .sort_entries(&self.dir, &mut entries, DATE_FORMAT, &metadata);
        Entries(entries)
    }
}

/// The internal preview reader. `bat` is an external process and not covered.
pub struct PreviewFixture {
    path: PathBuf,
    lines: usize,
    width: usize,
    scroll: usize,
}

impl PreviewFixture {
    pub fn new(path: &Path, lines: usize, width: usize, scroll: usize) -> Self {
        Self {
            path: path.to_path_buf(),
            lines,
            width,
            scroll,
        }
    }

    pub fn run(&self) -> Vec<String> {
        formatter::safe_read_preview(&self.path, self.lines, self.width, self.scroll)
    }
}

/// A running `AppState` with real workers, for measuring idle behaviour.
pub struct IdleFixture {
    app: AppState,
    workers: Workers,
}

impl IdleFixture {
    /// Opens `dir` and lets the initial loads and previews finish.
    pub fn new(dir: &Path) -> std::io::Result<Self> {
        let workers = Workers::spawn();
        let mut app = AppState::from_dir(Arc::new(Config::default()), dir)?;
        app.initialize(&workers, None);
        let mut fixture = Self { app, workers };
        fixture.run(Duration::from_millis(500));
        Ok(fixture)
    }

    /// Runs the event loop with no input and returns how many frames would
    /// have been redrawn. Should be zero.
    pub fn run(&mut self, duration: Duration) -> usize {
        // The real loop wakes at least every 16 ms to poll for input.
        const FRAME: Duration = Duration::from_millis(16);

        let end = Instant::now() + duration;
        let mut redraws = 0;
        while Instant::now() < end {
            let mut changed = false;
            while let Ok(response) = self.workers.response_rx().try_recv() {
                self.app.handle_worker_response(response, &self.workers);
                changed = true;
            }
            changed |= self.app.tick(&self.workers);
            redraws += usize::from(changed);
            std::thread::sleep(FRAME);
        }
        redraws
    }
}

/// `FileEntry` size in bytes, and heap allocations per entry without and with
/// a symlink target.
pub fn entry_layout() -> (usize, usize, usize) {
    (std::mem::size_of::<FileEntry>(), 3, 4)
}
