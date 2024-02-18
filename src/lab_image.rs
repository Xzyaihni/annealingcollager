use std::ops::{Index, IndexMut};

use image::{
    Rgb,
    RgbImage,
    Rgb32FImage,
    Rgba32FImage,
    buffer::ConvertBuffer
};

use crate::{Lab, Laba, Point2};


#[derive(Debug, Clone)]
pub struct GenericImage<T>
{
    data: Vec<T>,
    width: usize,
    height: usize
}

impl<T> GenericImage<T>
{
    pub fn from_raw(data: Vec<T>, width: usize, height: usize) -> Self
    {
        Self{data, width, height}
    }

    pub fn from_fn<F>(width: usize, height: usize, f: F) -> Self
    where
        F: FnMut(Point2<i32>) -> T
    {
        let data = (0..width * height).map(|index|
        {
            Self::to_position_assoc(width, index)
        }).map(f).collect();

        Self{
            data,
            width,
            height
        }
    }

    pub fn repeat(pixel: T, width: usize, height: usize) -> Self
    where
        T: Clone
    {
        Self{data: vec![pixel; width * height], width, height}
    }

    pub fn map<F, U>(self, f: F) -> GenericImage<U>
    where
        F: FnMut(T) -> U
    {
        GenericImage{
            width: self.width,
            height: self.height,
            data: self.data.into_iter().map(f).collect()
        }
    }

    pub fn width(&self) -> usize
    {
        self.width
    }

    pub fn height(&self) -> usize
    {
        self.height
    }

    pub fn pixels(&self) -> impl Iterator<Item=&T>
    {
        self.data.iter()
    }

    pub fn pixels_positions(&self) -> impl Iterator<Item=(Point2<i32>, &T)>
    {
        self.data.iter().enumerate().map(|(index, pixel)|
        {
            let position = self.to_position(index);

            (position, pixel)
        })
    }

    pub fn get(&self, position: Point2<i32>) -> Option<&T>
    {
        self.inbounds(position).then(||
        {
            let index = self.to_index(position);

            &self.data[index]
        })
    }

    pub fn get_mut(&mut self, position: Point2<i32>) -> Option<&mut T>
    {
        self.inbounds(position).then(||
        {
            let index = self.to_index(position);

            &mut self.data[index]
        })
    }

    pub fn resized_nearest(&self, size: Point2<usize>) -> Self
    where
        T: Clone
    {
        let this_size = Point2{x: self.width, y: self.height};
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

    fn inbounds(&self, position: Point2<i32>) -> bool
    {
        (0..self.width as i32).contains(&position.x)
            && (0..self.height as i32).contains(&position.y)
    }

    fn to_position(&self, index: usize) -> Point2<i32>
    {
        Self::to_position_assoc(self.width, index)
    }

    fn to_position_assoc(width: usize, index: usize) -> Point2<i32>
    {
        let x = (index % width) as i32;
        let y = (index / width) as i32;

        Point2{x, y}
    }

    fn to_index(&self, position: Point2<i32>) -> usize
    {
        Self::to_index_assoc(self.width, position)
    }

    fn to_index_assoc(width: usize, position: Point2<i32>) -> usize
    {
        position.x as usize + position.y as usize * width
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
