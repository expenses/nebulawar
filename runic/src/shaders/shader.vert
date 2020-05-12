#version 140

in vec2 in_pos;
in vec2 in_uv;

out vec2 out_uv;

void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    out_uv = in_uv;
}