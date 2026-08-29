//! Benchmark facade for `benches/`. `#[doc(hidden)]` and unreachable from `rn`,
//! so LTO strips it from the shipped binary.
//!
//! `benches/` is a separate crate and cannot see `pub(crate)` items, so fixtures
//! here expose them as public types with private fields.
//!
//! # Adding a fixture
//!
//! Give it `new` for setup, `run` for the measured call, and `reset` if `run`
//! mutates its input. Only `run` is timed; see `benches/sort.rs` for the loop.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;

use crate::core::cache::{DirCache, DirListOptions};
use crate::core::metadata::CachedMetaKey;
use crate::core::sort::{SortConfig, SortMode, SortOrder};
use crate::core::{FileEntry, Formatter, fm, formatter};
use crate::utils::text::StrBuffer;

/// Payload bytes held by a structure, excluding allocator overhead.
#[derive(Debug, Clone, Copy, Default)]
pub struct Footprint {
    pub inline: usize,
    pub heap: usize,
    pub items: usize,
}

impl Footprint {
    pub fn total(&self) -> usize {
        self.inline + self.heap
    }

    pub fn per_item(&self) -> f64 {
        if self.items == 0 {
            return 0.0;
        }
        self.total() as f64 / self.items as f64
    }
}

fn entries_footprint(entries: &[FileEntry], capacity: usize) -> Footprint {
    let heap = entries
        .iter()
        .map(|e| {
            e.name().len()
                + e.name_str().len()
                + e.lowered().len()
                + e.symlink().map_or(0, |p| p.as_os_str().len())
        })
        .sum();

    Footprint {
        inline: capacity * std::mem::size_of::<FileEntry>(),
        heap,
        items: entries.len(),
    }
}

fn str_buffer_footprint(buffer: &StrBuffer) -> usize {
    let data: usize = buffer.iter().map(|s| s.len()).sum();
    data + buffer.len() * std::mem::size_of::<u32>()
}

/// Fixed-seed LCG so every run produces byte-identical input.
fn synthetic_entries(count: usize) -> Vec<FileEntry> {
    const EXTS: [&str; 8] = ["rs", "txt", "md", "toml", "png", "log", "tar.gz", ""];

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut entries = Vec::with_capacity(count);

    for i in 0..count {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let r = (state >> 33) as usize;

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
        entries.push(FileEntry::new(OsString::from(name), flags, None));
    }

    entries
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

/// Memory cost of `FileEntry`, one of which exists per directory entry.
pub struct EntryFootprint;

impl EntryFootprint {
    pub fn measure(count: usize) -> Footprint {
        let entries = synthetic_entries(count);
        entries_footprint(&entries, entries.capacity())
    }

    pub fn struct_size() -> usize {
        std::mem::size_of::<FileEntry>()
    }

    /// One allocation per boxed field: `name`, `name_str`, `lowered`, `symlink`.
    pub fn allocations_per_entry(is_symlink: bool) -> usize {
        if is_symlink { 4 } else { 3 }
    }
}

/// Entries are `Arc`-shared, so many keys pointing at one listing pay for it
/// once. The per-directory cost is the key.
#[derive(Debug, Clone, Copy)]
pub struct CacheFootprint {
    pub directories: usize,
    pub key_bytes: usize,
    pub shared_entry_bytes: usize,
    pub shared_column_bytes: usize,
}

impl CacheFootprint {
    pub fn total(&self) -> usize {
        self.key_bytes + self.shared_entry_bytes + self.shared_column_bytes
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
    pub fn new(dirs: usize, entries_per_dir: usize) -> Self {
        let entries: Arc<[FileEntry]> = Arc::from(synthetic_entries(entries_per_dir));
        let sort_column = Some(Arc::new(StrBuffer::from_iter(
            entries.iter().map(|e| e.name_str()),
        )));

        let mut fixture = Self {
            cache: DirCache::new(),
            dirs: (0..dirs)
                .map(|i| PathBuf::from(format!("/bench/dir_{i}")))
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

    pub fn run_hit(&self) -> bool {
        let last = self.dirs.len().saturating_sub(1);
        self.cache
            .get(&self.dirs[last], self.sort, &self.list)
            .is_some()
    }

    pub fn run_miss(&self) -> bool {
        self.cache
            .get(Path::new("/bench/absent"), self.sort, &self.list)
            .is_some()
    }

    /// At capacity this includes the `min_by_key` eviction scan.
    pub fn run_insert(&mut self) {
        let index = (self.next_id as usize) % self.dirs.len();
        self.insert_at(index);
    }

    pub fn run_invalidate(&self) {
        let last = self.dirs.len().saturating_sub(1);
        self.cache.invalidate_path(&self.dirs[last]);
    }

    pub fn footprint(&self) -> CacheFootprint {
        let key_bytes = self
            .dirs
            .iter()
            .map(|p| p.as_os_str().len() + std::mem::size_of::<DirListOptions>())
            .sum();

        CacheFootprint {
            directories: self.dirs.len(),
            key_bytes,
            shared_entry_bytes: entries_footprint(&self.entries, self.entries.len()).total(),
            shared_column_bytes: self
                .sort_column
                .as_ref()
                .map_or(0, |b| str_buffer_footprint(b)),
        }
    }

    pub fn measure_footprint(dirs: usize, entries_per_dir: usize) -> CacheFootprint {
        Self::new(dirs, entries_per_dir).footprint()
    }
}

/// Sorting via `Formatter::sort_entries`.
///
/// `Size` and `Modified` stat every entry, and synthetic paths do not exist, so
/// those modes measure a fast failure path rather than real work. Use a real
/// directory for them.
pub struct SortFixture {
    formatter: Formatter,
    pristine: Vec<FileEntry>,
    working: Vec<FileEntry>,
    cache: DashMap<PathBuf, CachedMetaKey>,
    dir: PathBuf,
    date_format: String,
}

impl SortFixture {
    /// Sorts by plain name comparison.
    pub fn by_name(root: &Path, count: usize) -> Self {
        Self::build(root, count, SortMode::Name)
    }

    /// Sorts with the natural (digit-aware) comparator.
    pub fn by_natural(root: &Path, count: usize) -> Self {
        Self::build(root, count, SortMode::Natural)
    }

    /// Sorts by file extension.
    pub fn by_extension(root: &Path, count: usize) -> Self {
        Self::build(root, count, SortMode::Extension)
    }

    /// `root` is read only by the metadata sort modes, which are not exposed
    /// here: synthetic entries have no files behind them, so those would
    /// measure a fast failure path rather than real work.
    fn build(root: &Path, count: usize, mode: SortMode) -> Self {
        let pristine = synthetic_entries(count);
        Self {
            formatter: Formatter::new(
                list_options(),
                SortConfig::from((mode, SortOrder::Ascending)),
                Arc::new(HashSet::new()),
            ),
            working: pristine.clone(),
            pristine,
            cache: DashMap::new(),
            dir: root.to_path_buf(),
            date_format: "%Y-%m-%d".to_string(),
        }
    }

    /// Restores the unsorted input. Must run outside the timer.
    pub fn reset(&mut self) {
        self.working.clear();
        self.working.extend_from_slice(&self.pristine);
    }

    pub fn run(&mut self) -> usize {
        let _ = self.formatter.sort_entries(
            &self.dir,
            &mut self.working,
            &self.date_format,
            &self.cache,
        );
        self.working.len()
    }
}

/// The filter pass that runs before every sort.
pub struct FilterFixture {
    formatter: Formatter,
    pristine: Vec<FileEntry>,
    working: Vec<FileEntry>,
}

impl FilterFixture {
    pub fn new(count: usize) -> Self {
        let pristine = synthetic_entries(count);
        Self {
            formatter: Formatter::new(
                list_options(),
                SortConfig::default(),
                Arc::new(HashSet::new()),
            ),
            working: pristine.clone(),
            pristine,
        }
    }

    /// Restores the unfiltered input. Must run outside the timer.
    pub fn reset(&mut self) {
        self.working.clear();
        self.working.extend_from_slice(&self.pristine);
    }

    pub fn run(&mut self) -> usize {
        self.formatter.filter_entries(&mut self.working);
        self.working.len()
    }
}

/// `browse_dir` alone, and the full pipeline the io worker runs on a directory
/// change. I/O bound, so comparable run-to-run on one machine only.
pub struct ListingFixture {
    dir: PathBuf,
    formatter: Formatter,
    cache: DashMap<PathBuf, CachedMetaKey>,
    date_format: String,
}

impl ListingFixture {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            formatter: Formatter::new(
                list_options(),
                SortConfig::from((SortMode::Natural, SortOrder::Ascending)),
                Arc::new(HashSet::new()),
            ),
            cache: DashMap::new(),
            date_format: "%Y-%m-%d".to_string(),
        }
    }

    pub fn run_browse(&self) -> usize {
        fm::browse_dir(&self.dir).map(|e| e.len()).unwrap_or(0)
    }

    pub fn run_full(&self) -> usize {
        self.load().len()
    }

    fn load(&self) -> Vec<FileEntry> {
        let Ok(mut entries) = fm::browse_dir(&self.dir) else {
            return Vec::new();
        };
        self.formatter.filter_entries(&mut entries);
        let _ =
            self.formatter
                .sort_entries(&self.dir, &mut entries, &self.date_format, &self.cache);
        entries
    }

    pub fn footprint(&self) -> Footprint {
        let entries = self.load();
        entries_footprint(&entries, entries.capacity())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PreviewFootprint {
    pub lines: usize,
    pub text_bytes: usize,
    pub inline_bytes: usize,
}

impl PreviewFootprint {
    pub fn total(&self) -> usize {
        self.text_bytes + self.inline_bytes
    }
}

/// The internal preview reader, `formatter::safe_read_preview`.
pub struct PreviewFixture {
    path: PathBuf,
    max_lines: usize,
    pane_width: usize,
    scroll: usize,
}

impl PreviewFixture {
    pub fn new<P: AsRef<Path>>(path: P, max_lines: usize, pane_width: usize) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            max_lines,
            pane_width,
            scroll: 0,
        }
    }

    pub fn with_scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    fn read(&self) -> Vec<String> {
        formatter::safe_read_preview(&self.path, self.max_lines, self.pane_width, self.scroll)
    }

    pub fn run(&self) -> usize {
        self.read().len()
    }

    pub fn footprint(&self) -> PreviewFootprint {
        let lines = self.read();
        PreviewFootprint {
            lines: lines.len(),
            text_bytes: lines.iter().map(|l| l.len()).sum(),
            inline_bytes: lines.capacity() * std::mem::size_of::<String>(),
        }
    }
}
