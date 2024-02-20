use std::{
    fs,
    env,
    process,
    fmt::Display
};

use image::imageops::{self, FilterType};

pub use point::Point2;
pub use colors::{Lab, Laba};
pub use lab_image::{LabImage, LabaImage};

use config::Config;
use collager::{CollagerConfig, Collager};

mod config;
mod point;
mod colors;
mod lab_image;
mod collager;


fn complain(message: impl Display) -> !
{
    println!("{message}");

    process::exit(1)
}

fn main()
{
    let config = Config::parse(env::args().skip(1));

    let images: Vec<_> = fs::read_dir(config.directory).unwrap()
        .map(Result::unwrap)
        .filter_map(|entry|
        {
            entry.file_type().unwrap().is_file().then(||
            {
                entry.path()
            })
        }).map(|path|
        {
            image::open(path).unwrap().into_rgba32f()
        }).map(|image|
        {
            if let Some(little_size) = config.little_size
            {
                imageops::resize(&image, little_size, little_size, FilterType::CatmullRom)
            } else
            {
                image
            }
        }).collect();

    let input_image = image::open(config.input).unwrap();

    let input_image = if let Some(max_size) = config.max_size
    {
        input_image.resize(max_size, max_size, FilterType::CatmullRom)
    } else
    {
        input_image
    }.into_rgb32f();

    let collager_config = CollagerConfig{
        steps: config.steps,
        amount: config.amount,
        starts: config.starts.max(1),
        starting_temperature: config.starting_temperature,
        allow_scaling: config.allow_scaling,
        allow_rotation: config.allow_rotation,
        allow_hue: config.allow_hue,
        debug: config.debug
    };

    let collager = Collager::new(collager_config, input_image);

    collager.collage(&images).save(config.output).unwrap();
}
