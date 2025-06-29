use shaderc::Result;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs, mem, str};

pub struct Output {
    #[allow(dead_code)]
    pub sources: Vec<String>,
    pub spv: Vec<u32>,
}

#[derive(Clone)]
pub struct BuildOptions {
    pub kind: Option<shaderc::ShaderKind>,
    pub version: Option<u32>,
    pub debug: bool,
    /// When parsing build options in the proc macro,
    /// the definitions could have the `Vec<(String, Option<String>)>` type.
    /// But when outputting the [`BuildOptions`] as a constant,
    /// the type is `&'static [(&'static str, Option<&'static str>)]`,
    /// because there are allocations in constants.
    /// Using [`Cow`] combines both these types.
    #[allow(clippy::type_complexity)]
    pub definitions: Cow<'static, [(Cow<'static, str>, Option<Cow<'static, str>>)]>,
    pub optimization: shaderc::OptimizationLevel,
    pub target_version: u32,
}

impl Hash for BuildOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.map(|kind| mem::discriminant(&kind)).hash(state);
        self.version.hash(state);
        self.debug.hash(state);
        self.definitions.hash(state);
        mem::discriminant(&self.optimization).hash(state);
        self.target_version.hash(state);
    }
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            kind: None,
            version: None,
            debug: !cfg!(feature = "strip"),
            definitions: Cow::default(),
            optimization: if cfg!(feature = "default-optimize-zero") {
                shaderc::OptimizationLevel::Zero
            } else {
                shaderc::OptimizationLevel::Performance
            },
            target_version: if cfg!(feature = "default-version-vulkan1_4") {
                shaderc::EnvVersion::Vulkan1_4
            } else if cfg!(feature = "default-version-vulkan1_3") {
                shaderc::EnvVersion::Vulkan1_3
            } else if cfg!(feature = "default-version-vulkan1_2") {
                shaderc::EnvVersion::Vulkan1_2
            } else if cfg!(feature = "default-version-vulkan1_1") {
                shaderc::EnvVersion::Vulkan1_1
            } else {
                shaderc::EnvVersion::Vulkan1_0
            } as u32,
        }
    }
}

#[derive(Clone)]
pub struct Builder {
    pub src: String,
    pub path: PathBuf,
    pub options: BuildOptions,
}

impl Builder {
    pub fn build(self) -> Result<Output> {
        let Self {
            src,
            path: src_path,
            options: build_options,
        } = self;

        let path_str = src_path.to_string_lossy().into_owned();
        let sources = RefCell::new(vec![path_str.clone()]);

        // compute a hash over the source code and the build options
        let mut hasher = DefaultHasher::new();
        src.hash(&mut hasher);
        build_options.hash(&mut hasher);
        let hash = hasher.finish();

        let path = option_env!("OUT_DIR").map(|out_dir| Path::new(out_dir).join(hash.to_string()));

        if let Some(path) = &path {
            let data = fs::read(path);

            // check if a cached compilation exists
            if let Ok(data) = data {
                assert_eq!(0, data.len() % 4);
                let spv = data
                    .chunks_exact(4)
                    .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>();

                return Ok(Output {
                    sources: sources.into_inner(),
                    spv,
                });
            }
        }

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
        if let Some(version) = build_options.version {
            options.set_forced_version_profile(version, shaderc::GlslProfile::None);
        }
        for (name, value) in &*build_options.definitions {
            options.add_macro_definition(name, value.as_ref().map(|x| &x[..]));
        }
        if build_options.debug {
            options.set_generate_debug_info();
        }
        options.set_optimization_level(build_options.optimization);
        options.set_target_env(shaderc::TargetEnv::Vulkan, build_options.target_version);
        options.set_auto_bind_uniforms(true);

        let kind = build_options
            .kind
            .or_else(|| {
                src_path
                    .extension()
                    .and_then(|x| x.to_str().and_then(extension_kind))
            })
            .unwrap_or(shaderc::ShaderKind::InferFromSource);

        static COMPILER: OnceLock<shaderc::Compiler> = OnceLock::new();
        let compiler = COMPILER.get_or_init(|| shaderc::Compiler::new().unwrap());
        let out = compiler.compile_into_spirv(&src, kind, &path_str, "main", Some(&options))?;
        if out.get_num_warnings() != 0 {
            return Err(shaderc::Error::InternalError(out.get_warning_messages()));
        }
        mem::drop(options);

        if let Some(path) = path {
            // Write out the compilation result for caching
            let _ = fs::write(path, out.as_binary_u8());
        }

        Ok(Output {
            sources: sources.into_inner(),
            spv: out.as_binary().into(),
        })
    }
}

pub fn extension_kind(ext: &str) -> Option<shaderc::ShaderKind> {
    use shaderc::ShaderKind::*;
    Some(match ext {
        "vert" => Vertex,
        "frag" => Fragment,
        "comp" => Compute,
        "geom" => Geometry,
        "tesc" => TessControl,
        "tese" => TessEvaluation,
        "spvasm" => SpirvAssembly,
        "rgen" => RayGeneration,
        "rahit" => AnyHit,
        "rchit" => ClosestHit,
        "rmiss" => Miss,
        "rint" => Intersection,
        "rcall" => Callable,
        "task" => Task,
        "mesh" => Mesh,
        _ => {
            return None;
        }
    })
}
