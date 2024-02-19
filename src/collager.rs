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

        let mut output = Annealer::new(background, 10.0).anneal(self.config.steps).applied();

        let tenth = self.config.amount / 10;
        for i in 0..self.config.amount
        {
            if i % tenth == 0
            {
                let percentage = i as f32 / self.config.amount as f32 * 100.0;

                println!("progress: {percentage:.1}%");
            }

            let params =
                Node::cons(
                    IndexParam::random(images),
                    Node::cons(
                        ScaleParam::random(),
                        Node::cons(
                            AngleParam::random(),
                            Node::cons(
                                PositionParam::random(),
                                Node::nil()))));

            let annealable = ImageAnnealable::new(&self.image, &output, params);

            output = Annealer::new(annealable, 0.2).anneal(self.config.steps).applied();

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
        let do_pick_index = (fastrand::f32() + 0.05) < temperature;
        if do_pick_index
        {
            Self{index: fastrand::usize(0..self.images.len()), ..self}
        } else
        {
            self
        }
    }
}

#[derive(Clone)]
struct ScaleParam(Point2<f32>);

impl ScaleParam
{
    fn random() -> Self
    {
        Self(Point2{
            x: fastrand::f32() + 0.5,
            y: fastrand::f32() + 0.5
        })
    }
}

impl Paramable for ScaleParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        let raw = state.add_image.as_ref().unwrap();

        let original_size = Point2{x: raw.width(), y: raw.height()};
        let size = (original_size.map(|x| x as f32) * self.0).map(|x| x as usize);

        state.add_image = Some(raw.resized_nearest(size));

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(self.0.map(|x| change(x, 0.5).max(0.05)))
    }
}

#[derive(Clone)]
struct AngleParam(f32);

impl AngleParam
{
    fn random() -> Self
    {
        Self(fastrand::f32() * (2.0 * consts::PI))
    }
}

impl Paramable for AngleParam
{
    fn apply(&self, mut state: ImageState) -> ImageState
    {
        state.angle = Some(self.0);

        state
    }

    fn neighbor(self, temperature: f32) -> Self
    {
        let change = |v, scale|
        {
            UsefulOps::float_changed(v, temperature * scale)
        };

        Self(change(self.0, 0.01) % (2.0 * consts::PI))
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
        let r = || fastrand::f32() * 100.0;

        Self{original, color: Lab{l: r(), a: r(), b: r()}}
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

trait Annealable
{
    fn random_neighbor(&self, temperature: f32) -> Self;
    fn energy(&self) -> f32;
}

#[derive(Debug, Clone)]
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
    best_neighbor: Option<StateEnergy<S>>,
    max_temperature: f32
}

impl<S: Annealable + Clone> Annealer<S>
{
    pub fn new(start: S, max_temperature: f32) -> Self
    {
        Self{state: StateEnergy::new(start), best_neighbor: None, max_temperature}
    }

    pub fn anneal(mut self, steps: u32) -> S
    {
        for k in 0..steps
        {
            let fraction = (k + 1) as f32 / steps as f32;

            self.improve(self.temperature(1.0 - fraction));
        }

        self.best_neighbor.expect("steps must be above 0").state
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
