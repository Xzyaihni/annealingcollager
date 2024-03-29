use std::{
    fs,
    fmt::{self, Debug},
    path::PathBuf,
    f32::consts
};

use image::{
    Rgb32FImage,
    Rgba32FImage,
    RgbImage
};

use crate::{Point2, Lab, LabImage, LabaImage};


const SQRT_DISTANCE: bool = true;

pub struct CollagerConfig
{
    pub steps: u32,
    pub amount: u32,
    pub starts: u32,
    pub starting_temperature: f32,
    pub allow_scaling: bool,
    pub allow_rotation: bool,
    pub allow_hue: bool,
    pub allow_transparency: bool,
    pub debug: bool
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

        let background = BackgroundAnnealable::new(&self.image);

        let mut output = Annealer::new(background, 30.0).anneal(self.config.steps).applied();

        let tenth = (self.config.amount / 10).max(1);
        for i in 0..self.config.amount
        {
            if i % tenth == 0
            {
                let percentage = i as f32 / self.config.amount as f32 * 100.0;

                println!("progress: {percentage:.1}%");
            }

            let params = ||
            {
                Node::cons(
                    IndexParam::random(images),
                    Node::cons(
                        ScaleParam::random(self.config.allow_scaling),
                        Node::cons(
                            HueParam::random(self.config.allow_hue),
                            Node::cons(
                                TransparencyParam::random(self.config.allow_transparency),
                                Node::cons(
                                    AngleParam::random(self.config.allow_rotation),
                                    Node::cons(
                                        PositionParam::random(),
                                        Node::nil()))))))
            };

            let anneal = ||
            {
                let annealable = ImageAnnealable::new(&self.image, &output, params());

                Annealer::new(annealable, self.config.starting_temperature)
                    .anneal_with_energy(self.config.steps)
            };

            output = (0..self.config.starts).map(|_|
            {
                anneal()
            }).min_by(|a, b|
            {
                a.energy.partial_cmp(&b.energy).unwrap()
            }).expect("steps must be at least 1").state.applied();

            if self.config.debug
            {
                let debug_dir = PathBuf::from("test");

                if !debug_dir.exists()
                {
                    fs::create_dir(&debug_dir).unwrap();
                }

                let image_name = format!("image{i}.png");
                output.clone().to_rgb().save(debug_dir.join(image_name)).unwrap();
            }
        }

        let final_error = UsefulOps::image_difference(
            self.image.pixels().copied(),
            output.pixels().copied()
        );

        let error_per_pixel = final_error / (self.image.width() * self.image.height()) as f32;

        println!("final error per pixel: {error_per_pixel:.3}");

        output.to_rgb()
    }
}

// if lisp is so good why havent they made lisp 2?
#[derive(Clone)]
struct Node<T, C>(T, C);

impl<T, C> Node<T, C>
{
    pub fn cons(value: T, other: C) -> Self
    {
        Self(value, other)
    }
}

impl Node<(), ()>
{
    pub fn nil() -> () {}
}

trait NodeTrait
{
    type Item;
    type Child;

    // the word applies makes no sense here but i dont wanna be confused
    fn applies(&self, state: ImageState) -> ImageState;
    fn neighbors(self, temperature: f32) -> Self;
}

impl NodeTrait for ()
{
    type Item = ();
    type Child = ();

    fn applies(&self, state: ImageState) -> ImageState {state}
    fn neighbors(self, _temperature: f32) -> () {}
}

impl<T: Paramable, C: NodeTrait> NodeTrait for Node<T, C>
{
    type Item = T;
    type Child = C;

    fn applies(&self, state: ImageState) -> ImageState
    {
        self.1.applies(self.0.apply(state))
    }

    fn neighbors(self, temperature: f32) -> Self
    {
        Self(self.0.neighbor(temperature), self.1.neighbors(temperature))
    }
}

struct UsefulOps;

impl UsefulOps
{
    fn float_changed(v: f32, temperature: f32) -> f32
    {
        let delta = fastrand::f32() * 2.0 - 1.0;

        v + (delta * temperature)
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
}

struct ImageState
{
    image: LabImage,
    add_image: Option<LabaImage>,
    angle: Option<f32>,
}

// parametable? who cares its just a word
trait Paramable
{
    fn apply(&self, state: ImageState) -> ImageState;
    fn neighbor(self, temperature: f32) -> Self;
}

#[derive(Clone)]
struct IndexParam<'a>
{
    images: &'a [LabaImage],
    index: usize
}

impl<'a> IndexParam<'a>
{
    fn random(images: &'a [LabaImage]) -> Self
    {
        Self{index: fastrand::usize(0..images.len()), images}
    }
}

impl<'a> Paramable for IndexParam<'a>
{
    // i would love imagestate to contain a & but rust lifetimes r cancer
    // and i cant figure out if i can constrain self lifetime
    // to contain the & lifetime of the imagestate :/
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        state.add_image = Some(self.images[self.index].clone());

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        if fastrand::f32() < temperature
        {
            Self{index: fastrand::usize(0..self.images.len()), ..self}
        } else
        {
            self
        }
    }
}

#[derive(Clone)]
struct ScaleParam(Option<Point2<f32>>);

impl ScaleParam
{
    fn random(allow: bool) -> Self
    {
        Self(allow.then(||
        {
            Point2{
                x: fastrand::f32() + 0.5,
                y: fastrand::f32() + 0.5
            }
        }))
    }
}

impl Paramable for ScaleParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        if let Some(scale) = self.0
        {
            let raw = state.add_image.as_ref().unwrap();

            let original_size = Point2{x: raw.width(), y: raw.height()};
            let size = (original_size.map(|x| x as f32) * scale).map(|x| x as usize);

            state.add_image = Some(raw.resized_nearest(size));
        }

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|value| value.map(|x| change(x, 0.5).max(0.05))))
    }
}

#[derive(Clone)]
struct HueParam(Option<Lab>);

impl HueParam
{
    fn random(allow: bool) -> Self
    {
        let r = |value|
        {
            (fastrand::f32() * 2.0 - 1.0) * value
        };

        Self(allow.then(||
        {
            Lab{l: r(25.0), a: r(50.0), b: r(50.0)}
        }))
    }
}

impl Paramable for HueParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        if let Some(hue) = self.0
        {
            state.add_image.as_mut().unwrap().pixels_mut().for_each(|pixel|
            {
                pixel.l += hue.l;
                pixel.a += hue.a;
                pixel.b += hue.b;
            });
        }

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|value| value.map(|x| change(x, 20.0))))
    }
}

#[derive(Clone)]
struct TransparencyParam(Option<f32>);

impl TransparencyParam
{
    fn random(allow: bool) -> Self
    {
        Self(allow.then(||
        {
            fastrand::f32() * 2.0 - 1.0
        }))
    }
}

impl Paramable for TransparencyParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        if let Some(transparency) = self.0
        {
            state.add_image.as_mut().unwrap().pixels_mut().for_each(|pixel|
            {
                let lower_bound = 0.05;

                if pixel.alpha > lower_bound
                {
                    pixel.alpha = (pixel.alpha + transparency).clamp(lower_bound, 1.0);
                }
            });
        }

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|value| change(value, 0.01).clamp(-1.0, 1.0)))
    }
}

#[derive(Clone)]
struct AngleParam(Option<f32>);

impl AngleParam
{
    fn random(allow: bool) -> Self
    {
        Self(allow.then(||
        {
            fastrand::f32() * (2.0 * consts::PI)
        }))
    }
}

impl Paramable for AngleParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        state.angle = Some(self.0.unwrap_or(0.0));

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|value| change(value, 0.01) % (2.0 * consts::PI)))
    }
}

#[derive(Clone)]
struct PositionParam(Point2<f32>);

impl PositionParam
{
    fn random() -> Self
    {
        Self(Point2{
            x: fastrand::f32(),
            y: fastrand::f32()
        })
    }
}

impl Paramable for PositionParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        let add_image = state.add_image.take().unwrap();

        let size = state.image.size_point();
        let position = (self.0 * size.map(|x| x as f32))
            .zip(add_image.size_point()
                 .zip(size)
                 .map(|(small_size, total_size)| (total_size as i32 - small_size as i32).max(0)))
            .map(|(x, limit)| (x as i32).clamp(0, limit));

        state.image = state.image.overlay_rotated(&add_image, position, state.angle.unwrap());

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|x|
        {
            change(x, 1.0)
        }))
    }
}

#[derive(Clone)]
struct ImageAnnealable<'a, N>
{
    original: &'a LabImage,
    current: &'a LabImage,
    node: N
}

impl<'a, N> ImageAnnealable<'a, N>
{
    pub fn new(
        original: &'a LabImage,
        current: &'a LabImage,
        node: N
    ) -> Self
    where
        N: Clone
    {
        Self{original, current, node}
    }

    pub fn applied(&self) -> LabImage
    where
        N: NodeTrait
    {
        let state = ImageState{
            image: self.current.clone(),
            add_image: None,
            angle: None
        };

        self.node.applies(state).image
    }
}

impl<'a, N> Annealable for ImageAnnealable<'a, N>
where
    N: NodeTrait + Clone
{
    fn random_neighbor(&self, temperature: f32) -> Self
    {
        let mut output = self.clone();

        output.node = output.node.neighbors(temperature);

        output
    }

    fn energy(&self) -> f32
    {
        let pixels = self.applied();

        UsefulOps::image_difference(
            self.original.pixels().copied(),
            pixels.pixels().copied()
        )
    }
}

#[derive(Clone)]
struct BackgroundAnnealable<'a>
{
    original: &'a LabImage,
    color: Lab
}

impl<'a> Debug for BackgroundAnnealable<'a>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.debug_struct("BackgroundAnnealable")
            .field("color", &self.color)
            .finish()
    }
}

impl<'a> BackgroundAnnealable<'a>
{
    pub fn new(original: &'a LabImage) -> Self
    {
        Self{original, color: Lab::random()}
    }

    pub fn applied(&self) -> LabImage
    {
        LabImage::repeat(
            self.color,
            self.original.width(),
            self.original.height()
        )
    }
}

impl<'a> Annealable for BackgroundAnnealable<'a>
{
    fn random_neighbor(&self, temperature: f32) -> Self
    {
        let change = |v|
        {
            UsefulOps::float_changed(v, temperature)
        };

        let c = self.color;

        let mut output = self.clone();

        output.color = Lab{l: change(c.l), a: change(c.a), b: change(c.b)};

        output
    }

    fn energy(&self) -> f32
    {
        let pixels = self.applied();

        UsefulOps::image_difference(
            self.original.pixels().copied(),
            pixels.pixels().copied()
        )
    }
}

pub trait Annealable
{
    fn random_neighbor(&self, temperature: f32) -> Self;
    fn energy(&self) -> f32;
}

#[derive(Debug, Clone)]
pub struct StateEnergy<S>
{
    pub state: S,
    pub energy: f32
}

impl<S: Annealable> StateEnergy<S>
{
    fn new(state: S) -> Self
    {
        let energy = state.energy();

        Self{state, energy}
    }
}

#[derive(Clone)]
struct Annealer<S>
{
    state: StateEnergy<S>,
    best_neighbor: Option<StateEnergy<S>>,
    max_temperature: f32
}

impl<S: Annealable + Clone> Annealer<S>
{
    pub fn new(start: S, max_temperature: f32) -> Self
    {
        Self{state: StateEnergy::new(start), best_neighbor: None, max_temperature}
    }

    pub fn anneal(self, steps: u32) -> S
    {
        self.anneal_with_energy(steps).state
    }

    pub fn anneal_with_energy(mut self, steps: u32) -> StateEnergy<S>
    {
        for k in 0..steps
        {
            let fraction = (k + 1) as f32 / steps as f32;

            self.improve(self.temperature(1.0 - fraction));
        }

        self.best_neighbor.expect("steps must be above 0")
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

        let new_best = self.best_neighbor.is_none()
            || (neighbor.energy < self.best_neighbor.as_ref().unwrap().energy);

        if new_best
        {
            self.best_neighbor = Some(neighbor.clone());
        }

        if self.do_accept(self.state.energy, neighbor.energy, temperature)
        {
            self.state = neighbor;
        }
    }
}
