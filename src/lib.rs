pub use vk_shader_macros_impl::*;
// TODO hide from public API
#[cfg(feature = "hot-reloading")]
pub use shaderc::{OptimizationLevel, ShaderKind};

#[cfg(feature = "hot-reloading")]
mod hot_reloading;
#[cfg(feature = "hot-reloading")]
pub use hot_reloading::*;

#[cfg(feature = "reflection")]
mod reflection;
#[cfg(feature = "reflection")]
pub use reflection::*;

pub struct ShaderData {
    pub compile_time_spv: &'static [u32],
    #[cfg(feature = "hot-reloading")]
    #[doc(hidden)]
    pub hot_reloading: Option<std::sync::Mutex<HotReloadingData>>,
    #[cfg(feature = "reflection")]
    pub reflection: ReflectionData,
}

#[cfg(not(feature = "hot-reloading"))]
impl ShaderData {
    pub fn data(&self) -> impl std::ops::Deref<Target = [u32]> {
        self.compile_time_spv
    }
}
