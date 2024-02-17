use image::{
    Rgb,
    RgbImage,
    Rgb32FImage,
    GenericImageView,
    buffer::ConvertBuffer
};

use crate::{Lab, Point2};


type LabInner = Rgb32FImage;

// i hate this library
#[derive(Debug, Clone)]
pub struct LabImage(LabInner);

impl LabImage
{
    pub fn width(&self) -> u32
    {
        self.0.width()
    }

    pub fn height(&self) -> u32
    {
        self.0.height()
    }

    pub fn pixels(&self) -> impl Iterator<Item=Lab> + '_
    {
        self.0.pixels()
            .copied()
            .map(|Rgb([l, a, b])| Lab{l, a, b})
    }

    pub fn subimage_pixels(
        &self,
        position: Point2<u32>,
        size: Point2<u32>
    ) -> Vec<Lab>
    {
        self.0.view(position.x, position.y, size.x, size.y)
            .pixels()
            .map(|(_x, _y, pixel)| pixel)
            .map(|Rgb([l, a, b])| Lab{l, a, b})
            .collect()
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
    fn from(mut value: Rgb32FImage) -> Self
    {
        value.pixels_mut().for_each(|pixel|
        {
            let lab = Lab::from(*pixel);

            *pixel = Rgb::from([lab.l, lab.a, lab.b]);
        });

        Self(value)
    }
}
