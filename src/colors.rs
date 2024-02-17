use image::{Rgb, Rgba};


#[derive(Debug, Clone, Copy)]
pub struct Laba
{
    pub l: f32,
    pub a: f32,
    pub b: f32,
    pub alpha: f32
}

impl Laba
{
    pub fn no_alpha(self) -> Lab
    {
        Lab{
            l: self.l,
            a: self.a,
            b: self.b
        }
    }
}

impl From<Rgba<f32>> for Laba
{
    fn from(value: Rgba<f32>) -> Self
    {
        let [r, g, b, _a] = value.0;
        let rgb = Rgb::from([r, g, b]);

        let lab = Lab::from(rgb);

        Self{
            l: lab.l,
            a: lab.a,
            b: lab.b,
            alpha: value.0[3]
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Lab
{
    pub l: f32,
    pub a: f32,
    pub b: f32
}

impl Lab
{
    pub fn distance(&self, other: Lab) -> f32
    {
        let d_l = other.l - self.l;
        let d_a = other.a - self.a;
        let d_b = other.b - self.b;

        d_l.powi(2) + d_a.powi(2) + d_b.powi(2)
    }
}

impl From<Xyz> for Lab
{
    fn from(value: Xyz) -> Self
    {
        let delta = 6.0_f32 / 29.0;
        let delta_cube = delta.powi(3);
        let lower_scale = 1.0 / (delta.powi(2) * 3.0);

        let f = |value: f32| -> f32
        {
            if value > delta_cube
            {
                value.cbrt()
            } else
            {
                value * lower_scale + (4.0 / 29.0)
            }
        };

        let x = f(value.x / 95.047);
        let y = f(value.y / 100.0);
        let z = f(value.z / 108.883);

        let l = 116.0 * y - 16.0;
        let a = 500.0 * (x - y);
        let b = 200.0 * (y - z);

        Self{l, a, b}
    }
}

impl From<Rgb<f32>> for Lab
{
    fn from(value: Rgb<f32>) -> Self
    {
        Xyz::from(value).into()
    }
}

#[derive(Debug, Clone, Copy)]
struct Xyz
{
    x: f32,
    y: f32,
    z: f32
}

impl From<Rgb<f32>> for Xyz
{
    fn from(value: Rgb<f32>) -> Self
    {
        let f = |value: f32| -> f32
        {
            let value = if value <= 0.04045
            {
                value / 12.92
            } else
            {
                ((value + 0.055) / 1.055).powf(2.4)
            };

            value * 100.0
        };

        let r = f(value.0[0]);
        let g = f(value.0[1]);
        let b = f(value.0[2]);

        let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
        let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
        let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;

        Self{x, y, z}
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn close_enough(a: f32, b: f32)
    {
        assert!((a - b).abs() < 0.001, "a: {}, b: {}", a, b);
    }

    #[test]
    fn xyz_to_lab()
    {
        let xyz = Xyz{x: 0.5, y: 0.0, z: 0.0};

        let lab = Lab::from(xyz);

        close_enough(lab.l, 0.0);
        close_enough(lab.a, 20.482);
        close_enough(lab.b, 0.0);

        let xyz = Xyz{x: 0.1, y: 0.5, z: 0.9};

        let lab = Lab::from(xyz);

        close_enough(lab.l, 4.516);
        close_enough(lab.a, -15.371);
        close_enough(lab.b, -5.086);

        let rgb = Rgb::from([0.5, 0.2, 0.8]);

        let xyz = Xyz::from(rgb);

        close_enough(xyz.x, 20.907);
        close_enough(xyz.y, 11.278);
        close_enough(xyz.z, 58.190);
    }
}
