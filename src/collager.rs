use image::{
    Rgb32FImage,
    Rgba32FImage,
    RgbImage
};

use crate::{Point2, Lab, Laba, LabImage, LabaImage};


const SQRT_DISTANCE: bool = false;

pub struct CollagerConfig
{
    pub steps: u32,
    pub amount: u32
}

pub struct Collager
{
    config: CollagerConfig,
    image: LabImage
}

impl Collager
{
    pub fn new(config: CollagerConfig, image: Rgb32FImage) -> Self
    {
        Self{config, image: image.into()}
    }

    pub fn collage(&self, images: &[Rgba32FImage]) -> RgbImage
    {
        let images: Vec<_> = images.iter().map(|image|
        {
            LabaImage::from(image.clone())
        }).collect();

        let images = &images;

        let output = LabaImage::repeat(
            Laba{l: 0.0, a: 0.0, b: 0.0, alpha: 0.0},
            self.image.width(),
            self.image.height()
        );

        let background = BackgroundAnnealable::new(&self.image, &output);

        let mut output = Annealer::new(background, 10.0).anneal(self.config.steps).applied();

        let tenth = self.config.amount / 10;
        for i in 0..self.config.amount
        {
            if i % tenth == 0
            {
                let percentage = i as f32 / self.config.amount as f32 * 100.0;

                println!("progress: {percentage:.1}%");
            }

            let annealable = ImageAnnealable::new(&self.image, &output, images);

            output = Annealer::new(annealable, 0.5).anneal(self.config.steps).applied();
        }

        let final_error = ImageAnnealable::image_difference(
            self.image.pixels().copied(),
            output.pixels().map(|pixel| pixel.no_alpha())
        );

        println!("final error: {final_error:.1}");

        output.to_rgb()
    }
}

#[derive(Debug, Clone)]
struct ImageInfo
{
    index: usize,
    position: Point2<f32>,
    scale: Point2<f32>
}

impl ImageInfo
{
    pub fn random(len: usize) -> Self
    {
        Self{
            index: fastrand::usize(0..len),
            position: Point2{
                x: fastrand::f32(),
                y: fastrand::f32()
            },
            scale: Point2{
                x: fastrand::f32() + 0.5,
                y: fastrand::f32() + 0.5
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ImageAnnealable<'a>
{
    original: &'a LabImage,
    current: &'a LabaImage,
    images: &'a [LabaImage],
    info: ImageInfo
}

impl<'a> ImageAnnealable<'a>
{
    pub fn new(
        original: &'a LabImage,
        current: &'a LabaImage,
        images: &'a [LabaImage]
    ) -> Self
    {
        let info = ImageInfo::random(images.len());

        Self{original, current, images, info}
    }

    pub fn applied(&self) -> LabaImage
    {
        let add_image = self.add_image();

        let size = Point2{x: self.current.width(), y: self.current.height()}.map(|x| x as f32);
        let position = (self.info.position * size).map(|x| x as i32);

        self.current.clone().overlay(&add_image, position)
    }

    fn float_changed(v: f32, temperature: f32) -> f32
    {
        let delta = fastrand::f32() * 2.0 - 1.0;

        v + (delta * temperature)
    }

    fn add_image(&self) -> LabaImage
    {
        let raw = self.image();

        let original_size = Point2{x: raw.width(), y: raw.height()};
        let size = (original_size.map(|x| x as f32) * self.info.scale).map(|x| x as usize);

        raw.resized_nearest(size)
    }

    fn image_difference(a: impl Iterator<Item=Lab>, b: impl Iterator<Item=Lab>) -> f32
    {
        a.zip(b).map(|(original, changed)|
        {
            if SQRT_DISTANCE
            {
                original.distance(changed).sqrt()
            } else
            {
                original.distance(changed)
            }
        }).sum()
    }

    fn image(&self) -> &LabaImage
    {
        &self.images[self.info.index]
    }

    fn current_size(&self) -> Point2<f32>
    {
        let image = self.image();

        Point2{x: image.width(), y: image.height()}.map(|x| x as f32) * self.info.scale
    }
}

impl<'a> Annealable for ImageAnnealable<'a>
{
    fn random_neighbor(&self, temperature: f32) -> Self
    {
        let mut output = self.clone();

        let change = |v|
        {
            ImageAnnealable::float_changed(v, temperature)
        };

        output.info.scale = output.info.scale.map(|x| change(x).max(0.01));

        let current_size = Point2{x: self.current.width(), y: self.current.height()};

        let less_size = output.current_size().map(|x| (x - 1.0).max(0.0));
        let size_ratio = less_size / current_size.map(|x| x as f32);
        output.info.position = output.info.position
            .zip(size_ratio)
            .map(|(x, limit)|
            {
                change(x).clamp(-limit, 1.0)
            });

        let do_pick_index = fastrand::f32() < temperature;
        if do_pick_index
        {
            output.info.index = fastrand::usize(0..self.images.len());
        }

        output
    }

    fn energy(&self) -> f32
    {
        let pixels = self.applied();

        ImageAnnealable::image_difference(
            self.original.pixels().copied(),
            pixels.pixels().map(|pixel| pixel.no_alpha())
        )
    }
}

#[derive(Debug, Clone)]
struct BackgroundAnnealable<'a>
{
    original: &'a LabImage,
    compare: &'a LabaImage,
    color: Lab
}

impl<'a> BackgroundAnnealable<'a>
{
    pub fn new(original: &'a LabImage, compare: &'a LabaImage) -> Self
    {
        let r = || fastrand::f32() * 100.0;

        Self{original, compare, color: Lab{l: r(), a: r(), b: r()}}
    }

    pub fn applied(&self) -> LabaImage
    {
        // who in the hell thought that storing the pixels as Vec<f32>
        // is more efficient than Vec<Rgb<f32>>????
        // oh yea let me just try to do this optimization that makes everything slower!
        let blended = self.compare.pixels().map(|pixel|
        {
            Laba::from(self.color).blend(*pixel)
        }).collect();

        LabaImage::from_raw(
            blended,
            self.compare.width(),
            self.compare.height()
        )
    }
}

impl<'a> Annealable for BackgroundAnnealable<'a>
{
    fn random_neighbor(&self, temperature: f32) -> Self
    {
        let change = |v|
        {
            ImageAnnealable::float_changed(v, temperature)
        };

        let c = self.color;

        let mut output = self.clone();

        output.color = Lab{l: change(c.l), a: change(c.a), b: change(c.b)};

        output
    }

    fn energy(&self) -> f32
    {
        let pixels = self.applied();

        ImageAnnealable::image_difference(
            self.original.pixels().copied(),
            pixels.pixels().map(|pixel| pixel.no_alpha())
        )
    }
}

trait Annealable
{
    fn random_neighbor(&self, temperature: f32) -> Self;
    fn energy(&self) -> f32;
}

struct StateEnergy<S>
{
    state: S,
    energy: f32
}

impl<S: Annealable> StateEnergy<S>
{
    pub fn new(state: S) -> Self
    {
        let energy = state.energy();

        Self{state, energy}
    }
}

struct Annealer<S>
{
    state: StateEnergy<S>,
    max_temperature: f32
}

impl<S: Annealable> Annealer<S>
{
    pub fn new(start: S, max_temperature: f32) -> Self
    {
        Self{state: StateEnergy::new(start), max_temperature}
    }

    pub fn anneal(mut self, steps: u32) -> S
    {
        for k in 0..steps
        {
            let fraction = (k + 1) as f32 / steps as f32;

            self.improve(self.temperature(1.0 - fraction));
        }

        self.state.state
    }

    fn temperature(&self, fraction: f32) -> f32
    {
        self.max_temperature * fraction
    }

    fn do_accept(&self, energy: f32, neighbor_energy: f32, temperature: f32) -> bool
    {
        let energy_delta = neighbor_energy - energy;

        energy_delta <= temperature
    }

    fn improve(&mut self, temperature: f32)
    {
        let neighbor = StateEnergy::new(self.state.state.random_neighbor(temperature));

        if self.do_accept(self.state.energy, neighbor.energy, temperature)
        {
            self.state = neighbor;
        }
    }
}
