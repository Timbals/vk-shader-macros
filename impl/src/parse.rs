use crate::build::{BuildOptions, Builder, Output};
use crate::IncludeGlsl;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use shaderc::ShaderKind;
use std::borrow::Cow;
use std::fs;
use std::time::SystemTime;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Token};

impl Output {
    pub fn expand(self) -> TokenStream {
        let Self {
            sources,
            spv,
            #[cfg(feature = "reflection")]
            entry_points,
        } = self;

        let hot_reloading_data = if cfg!(feature = "hot-reloading") {
            quote!(hot_reloading: None,)
        } else {
            TokenStream::default()
        };

        #[cfg(feature = "reflection")]
        let reflection_data = reflection_data(&entry_points);
        #[cfg(not(feature = "reflection"))]
        let reflection_data = TokenStream::default();

        quote!(
            {
                #({ const _FORCE_DEP: &[u8] = include_bytes!(#sources); })*
                ::vk_shader_macros::ShaderData {
                    compile_time_spv: &[#(#spv),*],
                    #hot_reloading_data
                    #reflection_data
                }
            }
        )
    }
}

impl Parse for BuildOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut out = Self::default();

        while input.peek(Ident) {
            let key = input.parse::<Ident>()?;
            match key.to_string().as_str() {
                "kind" => {
                    input.parse::<Token![:]>()?;

                    let value = input.parse::<Ident>()?;
                    if let Some(kind) = crate::build::extension_kind(&value.to_string()) {
                        out.kind = Some(kind);
                    } else {
                        return Err(syn::Error::new(value.span(), "unknown shader kind"));
                    }
                }
                "version" => {
                    input.parse::<Token![:]>()?;

                    let value = input.parse::<LitInt>()?;
                    out.version = Some(value.base10_parse()?);
                }
                "strip" => {
                    out.debug = false;
                }
                "debug" => {
                    out.debug = true;
                }
                "define" => {
                    input.parse::<Token![:]>()?;

                    let name = input.parse::<Ident>()?;
                    let value = if input.peek(Token![,]) || input.is_empty() {
                        None
                    } else {
                        Some(Cow::Owned(input.parse::<LitStr>()?.value()))
                    };
                    out.definitions
                        .to_mut()
                        .push((Cow::Owned(name.to_string()), value));
                }
                "optimize" => {
                    input.parse::<Token![:]>()?;

                    let value = input.parse::<Ident>()?;
                    if let Some(level) = optimization_level(&value.to_string()) {
                        out.optimization = level;
                    } else {
                        return Err(
                            syn::Error::new(value.span(), "unknown optimization level").into()
                        );
                    }
                }
                "target" => {
                    input.parse::<Token![:]>()?;

                    let value = input.parse::<Ident>()?;
                    if let Some(version) = target(&value.to_string()) {
                        out.target_version = version as u32;
                    } else {
                        return Err(syn::Error::new(value.span(), "unknown target").into());
                    }
                }
                _ => {
                    return Err(syn::Error::new(key.span(), "unknown shader compile option").into());
                }
            }

            if input.peek(Token![,]) && input.peek2(Ident) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(out)
    }
}

impl ToTokens for IncludeGlsl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            output:
                Output {
                    sources,
                    spv,
                    #[cfg(feature = "reflection")]
                    entry_points,
                },
            builder:
                Builder {
                    options: build_options,
                    ..
                },
        } = self;

        let hot_reloading_data = if cfg!(feature = "hot-reloading") {
            let paths = sources.iter().map(|source| {
                let modified = fs::metadata(source)
                    .unwrap()
                    .modified()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                let secs = modified.as_secs();
                let nanos = modified.subsec_nanos();
                quote!((#source, std::time::Duration::new(#secs, #nanos)))
            });

            quote!(
                hot_reloading: Some(std::sync::Mutex::new(::vk_shader_macros::HotReloadingData {
                    data: None,
                    paths: &[#(#paths),*],
                    initialized: false,
                    build_options: #build_options,
                })),
            )
        } else {
            TokenStream::default()
        };

        #[cfg(feature = "reflection")]
        let reflection_data = reflection_data(entry_points);
        #[cfg(not(feature = "reflection"))]
        let reflection_data = TokenStream::default();

        tokens.append_all(quote!(
            {
                #({ const _FORCE_DEP: &[u8] = include_bytes!(#sources); })*
                ::vk_shader_macros::ShaderData {
                    compile_time_spv: &[#(#spv),*],
                    #hot_reloading_data
                    #reflection_data
                }
            }
        ))
    }
}

impl ToTokens for BuildOptions {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            kind,
            version,
            debug,
            definitions,
            optimization,
            target_version,
            ..
        } = self;

        let kind = kind_extension(*kind);
        let version = if let Some(version) = version.as_ref() {
            quote!(Some(#version))
        } else {
            quote!(None)
        };
        let optimization = serialize_optimization_level(*optimization);
        #[allow(unused_variables)] // false positive? with `quote!`
        let definitions = definitions.iter().map(|(a, b)| {
            let b = if let Some(b) = b.as_ref() {
                quote!(Some(::std::borrow::Cow::Borrowed(#b)))
            } else {
                quote!(None)
            };
            quote!((::std::borrow::Cow::Borrowed(#a), #b))
        });

        tokens.append_all(quote!(::vk_shader_macros::BuildOptions {
            kind: #kind,
            version: #version,
            debug: #debug,
            definitions: ::std::borrow::Cow::Borrowed(&[#(#definitions),*]),
            optimization: #optimization,
            target_version: #target_version,
        }))
    }
}

pub(crate) fn kind_extension(shader_kind: Option<ShaderKind>) -> syn::Expr {
    let mut s = "Some(::vk_shader_macros::ShaderKind::".to_owned();

    if shader_kind.is_none() {
        return syn::parse_str("None").unwrap();
    }
    let shader_kind = shader_kind.unwrap();

    use shaderc::ShaderKind::*;
    s += match shader_kind {
        Vertex => "Vertex",
        Fragment => "Fragment",
        Compute => "Compute",
        Geometry => "Geometry",
        TessControl => "TessControl",
        TessEvaluation => "TessEvaluation",
        SpirvAssembly => "SpirvAssembly",
        RayGeneration => "RayGeneration",
        AnyHit => "AnyHit",
        ClosestHit => "ClosestHit",
        Miss => "Miss",
        Intersection => "Intersection",
        Callable => "Callable",
        Task => "Task",
        Mesh => "Mesh",
        _ => {
            return syn::parse_str("None").unwrap();
        }
    };
    s += ")";
    syn::parse_str(&s).unwrap()
}

pub(crate) fn serialize_optimization_level(level: shaderc::OptimizationLevel) -> syn::Expr {
    match level {
        shaderc::OptimizationLevel::Zero => {
            syn::parse_str("::vk_shader_macros::OptimizationLevel::Zero").unwrap()
        }
        shaderc::OptimizationLevel::Size => {
            syn::parse_str("::vk_shader_macros::OptimizationLevel::Size").unwrap()
        }
        shaderc::OptimizationLevel::Performance => {
            syn::parse_str("::vk_shader_macros::OptimizationLevel::Performance").unwrap()
        }
    }
}

pub(crate) fn optimization_level(level: &str) -> Option<shaderc::OptimizationLevel> {
    match level {
        "zero" => Some(shaderc::OptimizationLevel::Zero),
        "size" => Some(shaderc::OptimizationLevel::Size),
        "performance" => Some(shaderc::OptimizationLevel::Performance),
        _ => None,
    }
}

pub(crate) fn target(s: &str) -> Option<shaderc::EnvVersion> {
    Some(match s {
        "vulkan" | "vulkan1_0" => shaderc::EnvVersion::Vulkan1_0,
        "vulkan1_1" => shaderc::EnvVersion::Vulkan1_1,
        "vulkan1_2" => shaderc::EnvVersion::Vulkan1_2,
        "vulkan1_3" => shaderc::EnvVersion::Vulkan1_3,
        _ => return None,
    })
}

#[cfg(feature = "reflection")]
fn reflection_data(entry_points: &Vec<spirq::entry_point::EntryPoint>) -> TokenStream {
    use spirq::ty::{ScalarType, Type};
    use spirq::var::Variable;

    // TODO support multiple entry points
    let mut spec_constants = entry_points[0]
        .vars
        .iter()
        .filter(|var| matches!(var, Variable::SpecConstant { .. }))
        .collect::<Vec<_>>();
    spec_constants.sort_unstable_by_key(|var| {
        let Variable::SpecConstant { spec_id, .. } = var else {
            unreachable!()
        };
        *spec_id
    });

    let mut specialization_constants = Vec::new();
    for spec_constant in spec_constants {
        let Variable::SpecConstant { spec_id, ty, .. } = spec_constant else {
            unreachable!()
        };

        let discriminant = match ty {
            Type::Scalar(ScalarType::Boolean) => {
                quote!(&::vk_shader_macros::SpecializationConstant::Bool(false))
            }
            Type::Scalar(ScalarType::Integer {
                bits: 8,
                is_signed: true,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::I8(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 16,
                is_signed: true,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::I16(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 32,
                is_signed: true,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::I32(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 64,
                is_signed: true,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::I64(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 8,
                is_signed: false,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::U8(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 16,
                is_signed: false,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::U16(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 32,
                is_signed: false,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::U32(0)),
            Type::Scalar(ScalarType::Integer {
                bits: 64,
                is_signed: false,
            }) => quote!(&::vk_shader_macros::SpecializationConstant::U64(0)),
            Type::Scalar(ScalarType::Float { bits: 16 }) => {
                quote!(&::vk_shader_macros::SpecializationConstant::F16(0))
            }
            Type::Scalar(ScalarType::Float { bits: 32 }) => {
                quote!(&::vk_shader_macros::SpecializationConstant::F32(0.0))
            }
            Type::Scalar(ScalarType::Float { bits: 64 }) => {
                quote!(&::vk_shader_macros::SpecializationConstant::F64(0.0))
            }
            _ => unimplemented!(),
        };

        specialization_constants.push(quote!((#spec_id, ::std::mem::discriminant(#discriminant))));
    }

    quote!(
        reflection: ::vk_shader_macros::ReflectionData {
            specialization_constants: &[#(#specialization_constants),*],
        },
    )
}
