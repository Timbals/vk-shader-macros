pub use build::BuildOptions;
use build::Builder;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::borrow::Cow;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
pub use vk_shader_macros_impl::*;
// TODO hide from public API
pub use shaderc::{OptimizationLevel, ShaderKind};

#[path = "../shared/build.rs"]
mod build;

static DIRTY: AtomicBool = AtomicBool::new(false);

/// Returns `true` when any shader has been changed
/// since the last time this function was called.
pub fn should_recompile() -> bool {
    DIRTY.swap(false, Ordering::AcqRel)
}

pub struct ShaderData {
    pub inner: Mutex<ShaderDataInner>,
}

pub struct ShaderDataInner {
    pub compile_time_spv: &'static [u32],
    /// Latest compiled SPIR-V
    pub data: Option<Vec<u32>>,
    /// All paths for dependencies of the shader.
    /// The second tuple field stores the last file modification.
    /// Should store a `SystemTime`, but that is an opaque type,
    /// so store `modified.duration_since(SystemTime::UNIX_EPOCH)`.
    /// The first entry is always the actual shader,
    /// and other entries are dependencies.
    pub paths: &'static [(&'static str, Duration)],

    pub initialized: bool,

    pub build_options: BuildOptions,
}

impl ShaderData {
    pub fn data(&self) -> impl Deref<Target = [u32]> {
        self.inner.lock().unwrap().data()
    }
}

impl ShaderDataInner {
    fn data(&mut self) -> impl Deref<Target = [u32]> {
        if !self.initialized {
            self.initialized = true;

            static WATCHER: Mutex<Option<RecommendedWatcher>> = Mutex::new(None);
            let mut watcher = WATCHER.lock().unwrap();
            let watcher = watcher.get_or_insert_with(|| {
                notify::recommended_watcher(|event: notify::Result<notify::Event>| {
                    let event = event.unwrap();
                    if let EventKind::Modify(_) = event.kind {
                        DIRTY.store(true, Ordering::Release);
                    }
                })
                .unwrap()
            });

            for (path, _) in self.paths.iter() {
                watcher
                    .watch(Path::new(path), RecursiveMode::NonRecursive)
                    .unwrap();
            }
        }

        if self.paths.iter().any(|(path, modified)| {
            let modified = SystemTime::UNIX_EPOCH + *modified;
            fs::metadata(path).unwrap().modified().unwrap() != modified
        }) {
            self.compile();
        }

        match &self.data {
            Some(data) => Cow::Owned(data.clone()),
            None => Cow::Borrowed(self.compile_time_spv),
        }
    }

    fn compile(&mut self) {
        let src_path = Path::new(self.paths[0].0);
        let src_name = self.paths[0].0;
        let src = fs::read_to_string(src_path).unwrap();
        let builder = Builder {
            src,
            name: src_name.to_string(),
            path: Some(src_path.to_path_buf()),
            options: self.build_options.clone(),
        };
        match builder.build() {
            Ok(output) => {
                self.data = Some(output.spv);
                // TODO update sources
            }
            Err(error) => eprintln!("{error:?}"),
        }
    }
}
