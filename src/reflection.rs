use crate::ShaderData;
use std::mem::Discriminant;

pub struct ReflectionData {
    /// Specialization constants defined in the shader.
    /// Note that missing specialization constants could mean they got optimized out.
    /// Stored as `[(constant_id, constant_type)]`.
    /// Will be sorted by `constant_id`, but ids might not be contiguous.
    pub specialization_constants: &'static [(u32, Discriminant<SpecializationConstant>)],
}

impl ShaderData {}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum SpecializationConstant {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(u16), // TODO use `f16` when stabilized
    F32(f32),
    F64(f64),
}

impl SpecializationConstant {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            SpecializationConstant::Bool(true) => bytemuck::bytes_of(&1_u32),
            SpecializationConstant::Bool(false) => bytemuck::bytes_of(&0_u32),
            SpecializationConstant::I8(x) => bytemuck::bytes_of(x),
            SpecializationConstant::I16(x) => bytemuck::bytes_of(x),
            SpecializationConstant::I32(x) => bytemuck::bytes_of(x),
            SpecializationConstant::I64(x) => bytemuck::bytes_of(x),
            SpecializationConstant::U8(x) => bytemuck::bytes_of(x),
            SpecializationConstant::U16(x) => bytemuck::bytes_of(x),
            SpecializationConstant::U32(x) => bytemuck::bytes_of(x),
            SpecializationConstant::U64(x) => bytemuck::bytes_of(x),
            SpecializationConstant::F16(x) => bytemuck::bytes_of(x),
            SpecializationConstant::F32(x) => bytemuck::bytes_of(x),
            SpecializationConstant::F64(x) => bytemuck::bytes_of(x),
        }
    }
}

impl From<bool> for SpecializationConstant {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i8> for SpecializationConstant {
    fn from(value: i8) -> Self {
        Self::I8(value)
    }
}
impl From<i16> for SpecializationConstant {
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}

impl From<i32> for SpecializationConstant {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<i64> for SpecializationConstant {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<u8> for SpecializationConstant {
    fn from(value: u8) -> Self {
        Self::U8(value)
    }
}

impl From<u16> for SpecializationConstant {
    fn from(value: u16) -> Self {
        Self::U16(value)
    }
}

impl From<u32> for SpecializationConstant {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

impl From<u64> for SpecializationConstant {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl From<f32> for SpecializationConstant {
    fn from(value: f32) -> Self {
        Self::F32(value)
    }
}

impl From<f64> for SpecializationConstant {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}
