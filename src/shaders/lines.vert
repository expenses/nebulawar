#version 150

in vec2 pos;
uniform vec2 window_dimensions;

vec2 screen_pos_to_opengl_pos(vec2 position) {
    return (vec2(0, 1) + vec2(1, -1) * position / window_dimensions - 0.5) * 2.0;
}

void main() {
    gl_Position = vec4(screen_pos_to_opengl_pos(pos), 0.0, 1.0);
}