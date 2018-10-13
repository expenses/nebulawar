#version 150

in vec2 position;
in vec3 color;
out vec3 v_color;

uniform vec2 window_dimensions;
uniform vec2 offset;

vec2 screen_pos_to_opengl_pos(vec2 position) {
    return (vec2(0, 1) + vec2(1, -1) * position / window_dimensions - 0.5) * 2.0;
}

void main() {
    v_color = color;
    gl_Position = vec4(screen_pos_to_opengl_pos(position), 0.0, 1.0);
}