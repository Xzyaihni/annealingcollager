use image::{
    Rgb,
    Rgba,
    Pixel,
    Rgb32FImage,
    Rgba32FImage,
    RgbImage,
    imageops,
    buffer::ConvertBuffer
};

use crate::Point2;


pub struct CollagerConfig
{
    pub steps: u32,
    pub amount: u32
}

pub struct Collager
{
    config: CollagerConfig,
    image: Rgb32FImage
}

impl Collager
{
    pub fn new(config: CollagerConfig, image: Rgb32FImage) -> Self
    {
        Self{config, image}
    }

    pub fn collage(&self, images: &[Rgba32FImage]) -> RgbImage
    {
        let mut output = Rgba32FImage::from_pixel(
            self.image.width(),
            self.image.height(),
            Rgba::from([0.0, 0.0, 0.0, 0.0])
        );

        let tenth = self.config.amount / 10;
        for i in 0..self.config.amount
        {
            if i % tenth == 0
            {
                let percentage = i as f32 / self.config.amount as f32 * 100.0;

                println!("progress: {percentage:.1}%");
            }

            let annealable = ImageAnnealable::new(&self.image, &output, images);

            output = Annealer::new(annealable, 0.1).anneal(self.config.steps).applied();
        }

        let background = BackgroundAnnealable::new(&self.image, &output);

        let output = Annealer::new(background, 0.1).anneal(self.config.steps);

        println!("final error: {:.3}", output.energy());

        output.applied().convert()
    }
}

#[derive(Debug, Clone)]
struct ImageInfo
{
    index: usize,
    position: Point2<f32>
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
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ImageAnnealable<'a>
{
    original: &'a Rgb32FImage,
    current: &'a Rgba32FImage,
    images: &'a [Rgba32FImage],
    info: ImageInfo
}

impl<'a> ImageAnnealable<'a>
{
    pub fn new(
        original: &'a Rgb32FImage,
        current: &'a Rgba32FImage,
        images: &'a [Rgba32FImage]
    ) -> Self
    {
        let info = ImageInfo::random(images.len());

        Self{original, current, images, info}
    }

    pub fn applied(&self) -> Rgba32FImage
    {
        let mut output = self.current.clone();

        let add_image = self.add_image();

        let size = Point2{x: self.current.width(), y: self.current.height()}.map(|x| x as f32);
        let Point2{x, y} = (self.info.position * size).map(|x| x as i64);

        imageops::overlay(&mut output, &add_image, x, y);

        output
    }

    fn float_changed(v: f32, temperature: f32) -> f32
    {
        let delta = fastrand::f32() * 2.0 - 1.0;

        let value: f32 = v + (delta * temperature);

        value.clamp(0.0, 1.0)
    }

    fn add_image(&self) -> Rgba32FImage
    {
        let raw = self.images[self.info.index].clone();

        raw
    }

    fn image_difference(a: impl Iterator<Item=Rgb<f32>>, b: impl Iterator<Item=Rgb<f32>>) -> f32
    {
        a.zip(b).map(|(original, changed)|
        {
            original.0.into_iter()
                .zip(changed.0.into_iter())
                .map(|(a, b)|
                {
                    (a - b).powi(2)
                }).sum::<f32>().sqrt()
        }).sum()
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

        output.info.position = output.info.position.map(|x| change(x));

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
            pixels.pixels().copied().map(|Rgba([r, g, b, _a])| Rgb::from([r, g, b]))
        )
    }
}

#[derive(Debug, Clone)]
struct BackgroundAnnealable<'a>
{
    original: &'a Rgb32FImage,
    compare: &'a Rgba32FImage,
    color: Rgb<f32>
}

impl<'a> BackgroundAnnealable<'a>
{
    pub fn new(original: &'a Rgb32FImage, compare: &'a Rgba32FImage) -> Self
    {
        let r = fastrand::f32;

        Self{original, compare, color: Rgb::from([r(), r(), r()])}
    }

    pub fn applied(&self) -> Rgba32FImage
    {
        // who in the hell thought that storing the pixels as Vec<f32>
        // is more efficient than Vec<Rgb<f32>>????
        // oh yea let me just try to do this optimization that makes everything slower!
        let blended = self.compare.pixels().flat_map(|pixel|
        {
            let mut background = self.color.to_rgba();
            background.blend(pixel);

            // clearly this is the optimal way to store pixels!
            background.channels().iter().copied().collect::<Vec<_>>()
        }).collect();

        Rgba32FImage::from_raw(
            self.compare.width(),
            self.compare.height(),
            blended
        ).unwrap()
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

        let c = self.color.0;

        let mut output = self.clone();

        output.color = Rgb::from([change(c[0]), change(c[1]), change(c[2])]);

        output
    }

    fn energy(&self) -> f32
    {
        let pixels = self.applied();

        ImageAnnealable::image_difference(
            self.original.pixels().copied(),
            pixels.pixels().copied().map(|Rgba([r, g, b, _a])| Rgb::from([r, g, b]))
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
