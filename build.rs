extern crate texture_packer;
extern crate image;
extern crate codegen;

use texture_packer::{*, importer::*, exporter::*, texture::*};
use codegen::*;
use std::path::*;
use std::fs::*;
use image::DynamicImage;

fn load_image(filename: &str) -> DynamicImage {
    ImageImporter::import_from_file(Path::new(filename)).unwrap()
}

fn main() {
     let mut packer = TexturePacker::new_skyline(TexturePackerConfig {
        trim: false,
        texture_padding: 0,
        .. Default::default()
     });

    packer.pack_own("star".to_string(), load_image("resources/star.png"));
    packer.pack_own("smoke".to_string(), load_image("resources/smoke.png"));
    packer.pack_own("mine".to_string(), load_image("resources/mine.png"));
    packer.pack_own("move".to_string(), load_image("resources/move.png"));

    let mut scope = Scope::new();

    {
        let impl_block = scope.new_impl("Image");    

        let mut offset_match_block = Block::new("match *self");

        let mut dimensions_match_block = Block::new("match *self");

        for (name, frame) in packer.get_frames() {
            let width = packer.width() as f32;
            let height = packer.height() as f32;

            dimensions_match_block.line(&format!(
                "Image::{} => Vector2::new({:?}, {:?}),",
                capitalize(name), frame.frame.w as f32 / width, frame.frame.h as f32 / height
            ));

            offset_match_block.line(&format!(
                "Image::{} => Vector2::new({:?}, {:?}),",
                capitalize(name), frame.frame.x as f32 / width, frame.frame.y as f32 / height
            ));
        }

        impl_block.new_fn("dimensions")
            .arg_ref_self()
            .ret("Vector2<f32>")
            .push_block(dimensions_match_block);

        impl_block.new_fn("offset")
            .arg_ref_self()
            .ret("Vector2<f32>")
            .push_block(offset_match_block);
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("packed_textures.rs");
    std::fs::write(dest_path, &scope.to_string()).unwrap();

    print!("{}", scope.to_string());

    //
    // Save the result
    //
    let exporter = ImageExporter::export(&packer).unwrap();
    let mut file = File::create("resources/output/packed.png").unwrap();
    exporter.write_to(&mut file, image::PNG).unwrap();
}

fn capitalize(string: &str) -> String {
    string[0..1].to_uppercase() + &string[1..]
}