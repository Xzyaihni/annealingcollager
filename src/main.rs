use std::{
    fs,
    env,
    process,
    fmt::Display
};

use image::imageops::FilterType;

pub use point::Point2;

use config::Config;
use collager::{CollagerConfig, Collager};

mod config;
mod point;
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
        amount: config.amount
    };

    let collager = Collager::new(collager_config, input_image);

    collager.collage(&images).save(config.output).unwrap();
}
