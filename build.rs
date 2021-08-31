use codegen::*;
use image::DynamicImage;
use std::path::*;
use texture_packer::{exporter::*, importer::*, texture::*, *};

fn load_image(path: &Path) -> DynamicImage {
    println!("{}", path.display());
    ImageImporter::import_from_file(path).unwrap()
}

fn load_dir(path: &str, packer: &mut TexturePacker<DynamicImage>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();

        if path.extension() == Some(std::ffi::OsStr::new("png")) {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let capitalised = case_style::CaseStyle::guess(stem).unwrap().to_pascalcase();
            packer.pack_own(capitalised, load_image(&path)).unwrap();
        }
    }
}

fn main() {
    let mut packer = TexturePacker::new_skyline(TexturePackerConfig {
        trim: false,
        texture_padding: 1,
        ..Default::default()
    });

    load_dir("resources", &mut packer);
    load_dir("resources/models", &mut packer);

    let width = packer.width();

    if width % 64 != 0 {
        let needed = 64 - (width % 64);
        packer.set_row_padding(needed);
    }

    let mut scope = Scope::new();

    {
        {
            let image_enum = scope
                .new_enum("Image")
                .derive("Copy, Clone, Component, Serialize, Deserialize, PartialEq, Debug")
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
                name,
                frame.frame.w as f32 / width,
                frame.frame.h as f32 / height
            ));

            offset_match_block.line(&format!(
                "Image::{} => [{:?}, {:?}],",
                name,
                frame.frame.x as f32 / width,
                frame.frame.y as f32 / height
            ));
        }

        impl_block
            .new_fn("dimensions")
            .arg_self()
            .vis("pub")
            .ret("[f32; 2]")
            .push_block(dimensions_match_block);

        impl_block
            .new_fn("offset")
            .arg_self()
            .vis("pub")
            .ret("[f32; 2]")
            .push_block(offset_match_block);

        impl_block
            .new_fn("translate")
            .arg_self()
            .vis("pub")
            .arg("uv", "[f32; 2]")
            .ret("[f32; 2]")
            .line("let offset = self.offset();")
            .line("let dim = self.dimensions();")
            .line(
                "[
                offset[0] + uv[0] * dim[0],
                offset[1] + uv[1] * dim[1]
            ]",
            );
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
