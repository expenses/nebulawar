#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 colour;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec4 v_colour;
layout(location = 1) out vec2 v_uv;

void main() {
    v_colour = colour;
    v_uv = uv;
    gl_Position = vec4(position, 1.0);
}
