use crate::build::extension_kind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::borrow::{Cow, ToOwned};
use std::cell::RefCell;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};
use std::{env, fs};
pub use vk_shader_macros_impl::*;

#[path = "../impl/src/build.rs"]
pub mod build; // TODO code duplication

// TODO hide from public API
pub use shaderc::{OptimizationLevel, ShaderKind};

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

#[derive(Clone)]
pub struct BuildOptions {
    pub kind: Option<ShaderKind>,
    pub version: Option<u32>,
    pub debug: bool,
    pub definitions: &'static [(&'static str, Option<&'static str>)],
    pub optimization: OptimizationLevel,
    pub target_version: u32,
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
        // TODO this function is duplicated from the proc macro crate
        let src_path = Some(Path::new(self.paths[0].0));
        let path_str = src_path.map(|x| x.to_string_lossy().into_owned());
        let sources = RefCell::new(path_str.map(|x| vec![x]).unwrap_or_default());

        let src = fs::read_to_string(self.paths[0].0).unwrap();
        let src_name = self.paths[0].0;
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_include_callback(|name, ty, src, _depth| {
            let path = match ty {
                shaderc::IncludeType::Relative => Path::new(src).parent().unwrap().join(name),
                shaderc::IncludeType::Standard => {
                    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join(name)
                }
            };
            let path_str = path.to_str().ok_or("non-unicode path")?.to_owned();
            sources.borrow_mut().push(path_str.clone());
            Ok(shaderc::ResolvedInclude {
                resolved_name: path_str,
                content: fs::read_to_string(path).map_err(|x| x.to_string())?,
            })
        });
        if let Some(version) = self.build_options.version {
            options.set_forced_version_profile(version, shaderc::GlslProfile::None);
        }
        for (name, value) in self.build_options.definitions {
            options.add_macro_definition(name, value.as_ref().map(|x| &x[..]));
        }
        if self.build_options.debug {
            options.set_generate_debug_info();
        }
        options.set_optimization_level(self.build_options.optimization);
        options.set_target_env(
            shaderc::TargetEnv::Vulkan,
            self.build_options.target_version,
        );

        let kind = self
            .build_options
            .kind
            .or_else(|| {
                src_path.and_then(|x| {
                    x.extension()
                        .and_then(|x| x.to_str().and_then(extension_kind))
                })
            })
            .unwrap_or(ShaderKind::InferFromSource);

        static COMPILER: OnceLock<shaderc::Compiler> = OnceLock::new();
        let compiler = COMPILER.get_or_init(|| shaderc::Compiler::new().unwrap());
        let compilation_artifact =
            compiler.compile_into_spirv(&src, kind, src_name, "main", Some(&options));

        match compilation_artifact {
            Ok(compilation_artifact) => {
                self.data = Some(compilation_artifact.as_binary().into());
            }
            Err(error) => {
                eprintln!("{error:?}");
            }
        }
    }
}
