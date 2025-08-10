#![cfg(feature = "reflection")]

use std::mem::discriminant;
use vk_shader_macros::{ShaderData, SpecializationConstant};
use vk_shader_macros_impl::include_glsl;

#[allow(dead_code)]
static REFLECTION: ShaderData = include_glsl!("reflection.frag", optimize: zero);

#[test]
fn specialization() {
    assert_eq!(
        REFLECTION.reflection.specialization_constants,
        [
            (0, discriminant(&SpecializationConstant::Bool(false))),
            (1, discriminant(&SpecializationConstant::I8(0))),
            (2, discriminant(&SpecializationConstant::I16(0))),
            (3, discriminant(&SpecializationConstant::I32(0))),
            (4, discriminant(&SpecializationConstant::I64(0))),
            (5, discriminant(&SpecializationConstant::U8(0))),
            (6, discriminant(&SpecializationConstant::U16(0))),
            (7, discriminant(&SpecializationConstant::U32(0))),
            (8, discriminant(&SpecializationConstant::U64(0))),
            (9, discriminant(&SpecializationConstant::F16(0))),
            (10, discriminant(&SpecializationConstant::F32(0.0))),
            (11, discriminant(&SpecializationConstant::F64(0.0))),
            (12, discriminant(&SpecializationConstant::Bool(false))),
            (64, discriminant(&SpecializationConstant::Bool(false))),
        ]
    );
}
