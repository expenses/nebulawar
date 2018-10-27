#version 140

in vec3 v_normal;
in vec2 v_texture;
in vec3 o_normal;

out vec4 color;
uniform vec3 light_direction;
uniform sampler2D tex;
uniform int mode;

float map(float min, float max, float value) {
    return value * (max - min) + min;
}

const int NORMAL = 1;
const int SHADELESS = 2;
const int WHITE = 3;
const int VERTEX_COLORED = 4;

const float MIN_LIGHT = 0.1;

void main() {
    vec4 texture_color = texture(tex, v_texture);

    if (mode == VERTEX_COLORED) {
        color = vec4(o_normal, 1.0);
    } else if (mode == SHADELESS) {
        color = texture_color;
    } else if (mode == WHITE) {
        color = vec4(vec3(1.0), v_texture.x);
    } else {
        vec3 light_dir = normalize(light_direction);  

        float brightness = dot(normalize(v_normal), normalize(light_dir));
        
        float norm_brightness = mix(MIN_LIGHT, 1.0, max(brightness, 0.0));

        color = vec4(norm_brightness * texture_color.rgb, texture_color.a);
    }
}