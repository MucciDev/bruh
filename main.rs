#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui_extras::RetainedImage;

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};

use image::{self, GenericImageView};
use rayon::prelude::*;
use skia_safe::{AlphaType, Color4f, ColorType, EncodedImageFormat, ImageInfo, Paint, Rect, Surface};

static TEMP_RESULT_PATH: &str = "temp.png";

fn vec_to_u32_ne(bytes: &[u8]) -> u32 {
    let mut result = [0u8; 4];
    result.copy_from_slice(bytes);
    u32::from_ne_bytes(result)
}

fn png_to_bruh(path: PathBuf) -> Result<(), std::io::Error> {
    let img = image::open(&path).expect("File not found!");

    let mut last_color = [0, 0, 0];
    let mut run_length = 0;
    let mut encoded_data = Vec::new();

    for pixel in img.pixels() {
        let current_color = pixel.2 .0;
        if current_color == last_color && run_length < 255 {
            run_length += 1;
        } else {
            if run_length > 0 {
                encoded_data.push((run_length as u8, last_color));
            }
            last_color = current_color;
            run_length = 1;
        }
    }

    if run_length > 0 {
        encoded_data.push((run_length as u8, last_color));
    }

    if let Some(path_str) = &path.to_str() {
        let height: u32 = img.height();
        let width: u32 = img.width();

        let height_bytes: [u8; 4] = height.to_ne_bytes();
        let width_bytes: [u8; 4] = width.to_ne_bytes();
        let path_to_bruh = path_str.replace(".png", ".bruh");

        // Ensure the .bruh file is created
        let _ = File::create(&path_to_bruh);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path_to_bruh)
            .expect("Couldn't write");

        file.write_all(&width_bytes)?;
        file.write_all(&height_bytes)?;

        for (run_length, color) in encoded_data {
            file.write_all(&[run_length])?;
            file.write_all(&color)?;
        }

        file.flush()?;
    } else {
        println!("couldn't find")
    }

    Ok(())
}

fn bruh_to_png(path: PathBuf) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let contents = fs::read(&path).expect("Couldn't read file.");
    let width = vec_to_u32_ne(&contents[0..4]);
    let height = vec_to_u32_ne(&contents[4..8]);

    let mut decoded_data = vec![[0, 0, 0]; (width * height) as usize];
    let mut idx = 8;
    let mut pos = 0;

    while idx < contents.len() {
        let run_length = contents[idx] as usize;
        let color = [contents[idx + 1], contents[idx + 2], contents[idx + 3]];

        for _ in 0..run_length {
            decoded_data[pos] = color;
            pos += 1;
        }

        idx += 4;
    }

    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Opaque,
        None,
    );

    let mut surface = Surface::new_raster(&info, None, None).unwrap();
    let canvas = surface.canvas();

    decoded_data
        .par_iter()
        .enumerate()
        .for_each(|(i, color)| {
            let color4f = Color4f::new(
                color[0] as f32 / 255.0,
                color[1] as f32 / 255.0,
                color[2] as f32 / 255.0,
                1.0,
            );
            let paint = Paint::new(color4f, None);

            let x = i % width as usize;
            let y = i / width as usize;
            let rect = Rect::from_point_and_size((x as f32, y as f32), (1.0, 1.0));
            canvas.draw_rect(rect, &paint);
        });

    let image = surface.image_snapshot();

    if let Some(data) = image.encode(None, EncodedImageFormat::PNG, 100) {
        fs::write(TEMP_RESULT_PATH, &*data).expect("Failed to write image data to file");
    }

    Ok((width, height))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let file_path: PathBuf = (&args[1]).into();

    if &args[1] == "compile" {
        if args.len() < 3 {
            panic!("Secondary argument ('path') not provided. Example: `cargo run compile ~/image.png`")
        }

        let path: PathBuf = (&args[2]).into();

        match png_to_bruh(path) {
            Ok(()) => println!("Successfully converted PNG to BRUH"),
            Err(e) => eprintln!("Failed to convert PNG to BRUH: {}", e),
        }

        Ok(())
    } else {
        let (width, height) = bruh_to_png(file_path)?;
        println!("{} {}", width, height);
        let options = eframe::NativeOptions {
            resizable: false,
            initial_window_size: Some(egui::vec2(width as f32, height as f32)),
            ..Default::default()
        };

        eframe::run_native(
            "Image preview",
            options,
            Box::new(|_cc| Box::<ImagePreview>::default()),
        )
    }
}

struct ImagePreview {
    image: RetainedImage,
    width: u32,
    height: u32,
}

impl ImagePreview {
    fn new(width: u32, height: u32) -> Self {
        let image_data = std::fs::read(TEMP_RESULT_PATH).expect("Failed to read image file");

        fs::remove_file(TEMP_RESULT_PATH).expect("File delete failed on TEMP_RESULT_PATH");

        Self {
            image: RetainedImage::from_image_bytes(TEMP_RESULT_PATH, &image_data).unwrap(),
            width,
            height,
        }
    }
}

impl eframe::App for ImagePreview {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let aspect_ratio = self.width as f32 / self.height as f32;
            let available_size = ui.available_size();
            let (width, height) = if available_size.x / aspect_ratio > available_size.y {
                (available_size.y * aspect_ratio, available_size.y)
            } else {
                (available_size.x, available_size.x / aspect_ratio)
            };
            ui.image(self.image.texture_id(ctx), [width, height]);
        });
    }
}

impl Default for ImagePreview {
    fn default() -> Self {
        ImagePreview::new(0, 0) // Dummy default, should be replaced in actual use
    }
}
