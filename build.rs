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
        texture_padding: 1,
        .. Default::default()
    });

    packer.pack_own("Star".to_string(), load_image("resources/star.png")).unwrap();
    packer.pack_own("Smoke".to_string(), load_image("resources/smoke.png")).unwrap();
    packer.pack_own("Mine".to_string(), load_image("resources/mine.png")).unwrap();
    packer.pack_own("Move".to_string(), load_image("resources/move.png")).unwrap();
    packer.pack_own("Attack".to_string(), load_image("resources/attack.png")).unwrap();
    packer.pack_own("Explosion1".to_string(), load_image("resources/explosion/1.png")).unwrap();
    packer.pack_own("Explosion2".to_string(), load_image("resources/explosion/2.png")).unwrap();
    packer.pack_own("Explosion3".to_string(), load_image("resources/explosion/3.png")).unwrap();
    packer.pack_own("Explosion4".to_string(), load_image("resources/explosion/4.png")).unwrap();
    packer.pack_own("Explosion5".to_string(), load_image("resources/explosion/5.png")).unwrap();
    packer.pack_own("Explosion6".to_string(), load_image("resources/explosion/6.png")).unwrap();

    let mut scope = Scope::new();

    {
        {
            let image_enum = scope.new_enum("Image")
                .derive("Copy, Clone, Component, Serialize, Deserialize, PartialEq")
                .vis("pub");

            for (name, _) in packer.get_frames() {
                image_enum.push_variant(Variant::new(name));
            }
        }

        let impl_block = scope.new_impl("Image");    

        let mut offset_match_block = Block::new("match self");

        let mut dimensions_match_block = Block::new("match self");

        for (name, frame) in packer.get_frames() {
            let width = packer.width() as f32;
            let height = packer.height() as f32;

            dimensions_match_block.line(&format!(
                "Image::{} => [{:?}, {:?}],",
                name, frame.frame.w as f32 / width, frame.frame.h as f32 / height
            ));

            offset_match_block.line(&format!(
                "Image::{} => [{:?}, {:?}],",
                name, frame.frame.x as f32 / width, frame.frame.y as f32 / height
            ));
        }

        impl_block.new_fn("dimensions")
            .arg_self()
            .vis("pub")
            .ret("[f32; 2]")
            .push_block(dimensions_match_block);

        impl_block.new_fn("offset")
            .arg_self()
            .vis("pub")
            .ret("[f32; 2]")
            .push_block(offset_match_block);

        impl_block.new_fn("translate")
            .arg_self()
            .vis("pub")
            .arg("uv", "[f32; 2]")
            .ret("[f32; 2]")
            .line("let offset = self.offset();")
            .line("let dim = self.dimensions();")
            .line("[
                offset[0] + uv[0] * dim[0],
                1.0 - (offset[1] + uv[1] * dim[1])
            ]");
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("packed_textures.rs");
    std::fs::write(dest_path, &scope.to_string()).unwrap();

    print!("{}", scope.to_string());

    //
    // Save the result
    //
    let exporter = ImageExporter::export(&packer).unwrap();
    exporter.save("resources/output/packed.png").unwrap();
}
