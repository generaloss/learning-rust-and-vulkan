#version 450

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;

layout(location = 0) out vec2 v_uv;

layout(push_constant) uniform PushConstants {
    mat4 u_combined;
} pcs;

void main() {
    gl_Position = pcs.u_combined * vec4(a_pos, 0.0, 1.0);
    v_uv = a_uv;
}