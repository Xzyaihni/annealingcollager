use std::ops::{Index, IndexMut};

use image::{
    Rgb,
    RgbImage,
    Rgb32FImage,
    Rgba32FImage,
    buffer::ConvertBuffer
};

use crate::{Lab, Laba, Point2};


#[derive(Debug, Clone, Copy)]
struct Indexer(Point2<usize>);

impl Indexer
{
    pub fn new(width: usize, height: usize) -> Self
    {
        Self(Point2{x: width, y: height})
    }

    pub fn to_position(&self, index: usize) -> Point2<i32>
    {
        let x = (index % self.0.x) as i32;
        let y = (index / self.0.x) as i32;

        Point2{x, y}
    }

    pub fn to_index(&self, position: Point2<i32>) -> usize
    {
        position.x as usize + position.y as usize * self.0.x
    }
}

#[derive(Debug, Clone)]
pub struct GenericImage<T>
{
    data: Vec<T>,
    indexer: Indexer
}

impl<T> GenericImage<T>
{
    pub fn from_raw(data: Vec<T>, width: usize, height: usize) -> Self
    {
        Self{data, indexer: Indexer::new(width, height)}
    }

    pub fn from_fn<F>(width: usize, height: usize, f: F) -> Self
    where
        F: FnMut(Point2<i32>) -> T
    {
        let indexer = Indexer::new(width, height);

        let data = (0..width * height).map(|index|
        {
            indexer.to_position(index)
        }).map(f).collect();

        Self{
            data,
            indexer
        }
    }

    pub fn repeat(pixel: T, width: usize, height: usize) -> Self
    where
        T: Clone
    {
        Self{data: vec![pixel; width * height], indexer: Indexer::new(width, height)}
    }

    pub fn map<F, U>(self, f: F) -> GenericImage<U>
    where
        F: FnMut(T) -> U
    {
        GenericImage{
            indexer: self.indexer,
            data: self.data.into_iter().map(f).collect()
        }
    }

    pub fn width(&self) -> usize
    {
        self.indexer.0.x
    }

    pub fn height(&self) -> usize
    {
        self.indexer.0.y
    }

    pub fn pixels(&self) -> impl Iterator<Item=&T>
    {
        self.data.iter()
    }

    pub fn pixels_mut(&mut self) -> impl Iterator<Item=&mut T>
    {
        self.data.iter_mut()
    }

    pub fn pixels_positions(&self) -> impl Iterator<Item=(Point2<i32>, &T)>
    {
        self.pixels().enumerate().map(|(index, pixel)|
        {
            (self.indexer.to_position(index), pixel)
        })
    }

    pub fn pixels_positions_mut(&mut self) -> impl Iterator<Item=(Point2<i32>, &mut T)>
    {
        let indexer = self.indexer;

        self.pixels_mut().enumerate().map(move |(index, pixel)|
        {
            (indexer.to_position(index), pixel)
        })
    }

    pub fn pixels_between_mut(
        &mut self,
        low: Point2<i32>,
        high: Point2<i32>
    ) -> impl Iterator<Item=(Point2<i32>, &mut T)>
    {
        self.pixels_positions_mut().filter(move |(position, _x)|
        {
            Self::between(low, high, *position)
        })
    }

    pub fn get(&self, position: Point2<i32>) -> Option<&T>
    {
        self.inbounds(position).then(||
        {
            let index = self.indexer.to_index(position);

            &self.data[index]
        })
    }

    pub fn get_mut(&mut self, position: Point2<i32>) -> Option<&mut T>
    {
        self.inbounds(position).then(||
        {
            let index = self.indexer.to_index(position);

            &mut self.data[index]
        })
    }

    pub fn resized_nearest(&self, size: Point2<usize>) -> Self
    where
        T: Clone
    {
        let this_size = self.size_point();
        let scale = this_size.map(|x| x as f32) / size.map(|x| x as f32);

        Self::from_fn(size.x, size.y, |position|
        {
            let scaled_position = (position.map(|x| x as f32) * scale)
                .zip(this_size)
                .map(|(value, limit)|
                {
                    (value as i32).clamp(0, limit as i32 - 1)
                });

            self[scaled_position].clone()
        })
    }

    pub fn size_point(&self) -> Point2<usize>
    {
        self.indexer.0
    }

    fn inbounds(&self, position: Point2<i32>) -> bool
    {
        Self::between(Point2::repeat(0), self.size_point().map(|x| x as i32), position)
    }

    fn between(low: Point2<i32>, high: Point2<i32>, position: Point2<i32>) -> bool
    {
        let c = position.zip(low.zip(high)).map(|(position, (low, high))|
        {
            (low..high).contains(&position)
        });

        c.x && c.y
    }
}

impl<T> Index<Point2<i32>> for GenericImage<T>
{
    type Output = T;

    fn index(&self, index: Point2<i32>) -> &Self::Output
    {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<Point2<i32>> for GenericImage<T>
{
    fn index_mut(&mut self, index: Point2<i32>) -> &mut Self::Output
    {
        self.get_mut(index).unwrap()
    }
}

pub type LabaImage = GenericImage<Laba>;

impl LabaImage
{
    pub fn to_rgb(self) -> RgbImage
    {
        LabImage::from(self).to_rgb()
    }

    pub fn overlay(mut self, other: &LabaImage, position: Point2<i32>) -> LabaImage
    {
        other.pixels_positions().for_each(|(pixel_position, pixel)|
        {
            let position = position + pixel_position;
            if let Some(this_pixel) = self.get_mut(position)
            {
                *this_pixel = this_pixel.blend(*pixel);
            }
        });

        self
    }
}

impl From<Rgba32FImage> for LabaImage
{
    fn from(value: Rgba32FImage) -> Self
    {
        Self::from(&value)
    }
}

impl From<&Rgba32FImage> for LabaImage
{
    fn from(value: &Rgba32FImage) -> Self
    {
        let data = value.pixels().map(|pixel|
        {
            Laba::from(*pixel)
        }).collect();

        GenericImage::from_raw(data, value.width() as usize, value.height() as usize)
    }
}

pub type LabImage = GenericImage<Lab>;

impl LabImage
{
    pub fn to_rgb(self) -> RgbImage
    {
        RgbImage::from_raw(
            self.width() as u32,
            self.height() as u32,
            self.pixels().flat_map(|pixel|
            {
                let rgb = Rgb::from(*pixel);

                rgb.0
            }).collect()
        ).unwrap()
    }

    pub fn overlay(mut self, other: &LabaImage, position: Point2<i32>) -> LabImage
    {
        other.pixels_positions().for_each(|(pixel_position, pixel)|
        {
            let position = position + pixel_position;
            if let Some(this_pixel) = self.get_mut(position)
            {
                *this_pixel = this_pixel.blend(*pixel);
            }
        });

        self
    }

    pub fn overlay_rotated(
        mut self,
        other: &LabaImage,
        position: Point2<i32>,
        angle: f32
    ) -> LabImage
    {
        let rotate = |origin: Point2<f32>, position: Point2<i32>, angle: f32|
        {
            let position = position.map(|x| x as f32) - origin;

            let a_cos = angle.cos();
            let a_sin = angle.sin();

            Point2{
                x: a_cos * position.x - a_sin * position.y,
                y: a_sin * position.x + a_cos * position.y
            } + origin
        };

        let middle = other.size_point().map(|x| x as f32) / 2.0;

        let global_middle = position.map(|x| x as f32) + middle;

        let this_rotate = |position: Point2<i32>|
        {
            rotate(global_middle, position, angle)
        };

        let size = other.size_point().map(|x| x as i32);

        let rotated_ll = this_rotate(position);
        let rotated_lh = this_rotate(position + Point2{x: 0, ..size});
        let rotated_hl = this_rotate(position + Point2{y: 0, ..size});
        let rotated_hh = this_rotate(position + size);

        let rotated = rotated_ll.zip(rotated_lh).zip(rotated_hl).zip(rotated_hh);

        fn select<F>(f: F) -> impl FnMut((((f32, f32), f32), f32)) -> f32
        where
            F: Fn(f32, f32) -> f32
        {
            move |(((ll, lh), hl), hh)| f(ll, f(lh, f(hl, hh)))
        }

        // bounding boxes
        let bb_low = rotated.map(select(f32::min)).map(|x| x.floor() as i32);
        let bb_high = rotated.map(select(f32::max)).map(|x| x.ceil() as i32);

        self.pixels_between_mut(bb_low, bb_high).for_each(|(pixel_position, pixel)|
        {
            let position = rotate(global_middle, pixel_position, angle)
                .map(|x| x.round() as i32) - position;

            if let Some(other_pixel) = other.get(position)
            {
                *pixel = pixel.blend(*other_pixel);
            }
        });

        self
    }
}

impl From<LabaImage> for LabImage
{
    fn from(value: LabaImage) -> Self
    {
        value.map(|pixel| pixel.no_alpha())
    }
}

impl From<RgbImage> for LabImage
{
    fn from(value: RgbImage) -> Self
    {
        <Self as From<Rgb32FImage>>::from(value.convert())
    }
}

impl From<Rgb32FImage> for LabImage
{
    fn from(value: Rgb32FImage) -> Self
    {
        let data = value.pixels().map(|pixel|
        {
            Lab::from(*pixel)
        }).collect();

        GenericImage::from_raw(data, value.width() as usize, value.height() as usize)
    }
}
