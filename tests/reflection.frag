#version 460

#extension GL_EXT_shader_explicit_arithmetic_types: require
#extension GL_EXT_shader_explicit_arithmetic_types_int8: require
#extension GL_EXT_shader_explicit_arithmetic_types_int16: require
#extension GL_EXT_shader_explicit_arithmetic_types_int32: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64: require
#extension GL_EXT_shader_explicit_arithmetic_types_float16: require
#extension GL_EXT_shader_explicit_arithmetic_types_float32: require
#extension GL_EXT_shader_explicit_arithmetic_types_float64: require

layout (constant_id = 0) const bool CONST_bool = false;
layout (constant_id = 1) const int8_t CONST_i8 = int8_t(0);
layout (constant_id = 2) const int16_t CONST_i16 = 0s;
layout (constant_id = 3) const int32_t CONST_i32 = 0;
layout (constant_id = 4) const int64_t CONST_i64 = 0l;
layout (constant_id = 5) const uint8_t CONST_u8 = uint8_t(0);
layout (constant_id = 6) const uint16_t CONST_u16 = 0us;
layout (constant_id = 7) const uint32_t CONST_u32 = 0u;
layout (constant_id = 8) const uint64_t CONST_u64 = 0ul;
layout (constant_id = 9) const float16_t CONST_f16 = 0.0hf;
layout (constant_id = 10) const float32_t CONST_f32 = 0.0f;
layout (constant_id = 11) const float64_t CONST_f64 = 0.0lf;

layout (constant_id = 64) const bool non_contiguous_id = false;
layout (constant_id = 12) const bool sorting = false;

void main() {}
