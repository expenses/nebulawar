#version 140

in vec4 v_color;
in vec2 v_uv;

out vec4 color;

uniform bool draw_image;
uniform sampler2D image;

void main() {
    if (draw_image) {
        vec4 image_color = texture(image, v_uv);

        color = mix(image_color, vec4(v_color.rgb, 1.0), v_color.a);
    } else {
        color = v_color;
    }
}