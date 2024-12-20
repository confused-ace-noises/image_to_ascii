use colored::CustomColor;
use image::{GenericImageView, ImageReader, Pixel, Rgba};
use rayon::prelude::*;
use std::error::Error;

use crate::core::{algo::algo_parallel, chars::{ColorChar, DensityChar}};

#[derive(Debug, Clone)]
pub struct AsciiImg {
    pub height: Option<usize>,
    pub width: Option<usize>,
    pub pixels: Vec<Vec<ColorChar>>,
}
impl AsciiImg {
    pub fn new(
        path: String,
        target_height: Option<usize>,
        target_width: Option<usize>,
        invert: bool,
        grayscale: bool,
        uniform: bool,
    ) -> Result<AsciiImg, Box<dyn Error>> {
        let target_image = path;

        let img: image::DynamicImage;

        if grayscale {
            img = ImageReader::open(target_image)?.decode()?.grayscale();
        } else {
            img = ImageReader::open(target_image)?.decode()?;
        }

        let (width, height) = img.dimensions();

        let mut pixels: Vec<Vec<Rgba<u8>>> = Vec::new();

        for y in 0..height {
            let mut inner_vec = Vec::new();
            for x in 0..width {
                // Get the pixel as an RGBA tuple
                let pixel = img.get_pixel(x, y);

                inner_vec.push(pixel);
            }
            pixels.push(inner_vec);
        }

        println!("height: {}, width: {}", height, width);
        let (final_height, final_width) = match (target_height, target_width) {
            (None, None) => (height as usize, width as usize),
            (Some(v), None) => {
                let ratio = height as f32 / width as f32;
                println!("{}", ratio);
                (
                    v,
                    (((v as f32 / ratio) * 2.0).floor() as usize).clamp(1, width as usize),
                )
            }
            (None, Some(h)) => {
                let ratio = height as f32 / width as f32;

                (
                    (((h as f32 * ratio) / 2.0).ceil() as usize).clamp(1, height as usize),
                    h,
                )
            }
            (Some(v), Some(h)) => (v.clamp(1, height as usize), h.clamp(1, width as usize)),
        };
        println!("height: {}, width: {}", final_height, final_width);

        let source_height = pixels.len();
        let source_width = pixels[0].len();

        assert!(
            source_height > 0 && source_width > 0,
            "Input grid must not be empty."
        );
        assert!(
            final_width > 0 && final_height > 0,
            "Target dimensions must be greater than zero."
        );

        let mut output = Vec::new();

        let scale_x = ((source_width as f32 / final_width as f32).ceil() as usize).max(1);
        let scale_y = ((source_height as f32 / final_height as f32).ceil() as usize).max(1);

        for h in 0..final_height {
            let mut out_row = Vec::new();
            for w in 0..final_width {
                let mut buffer = Vec::new();
                for y in 0..scale_y {
                    for x in 0..scale_x {
                        let indx_v = ((w * scale_x) + x) as usize;
                        let indx_h = ((h * scale_y) + y) as usize;

                        if indx_v >= source_width || indx_h >= source_height {
                            continue;
                        }
                        buffer.push(pixels[indx_h][indx_v]);
                        // buffer.push(pixels[indx_v][indx_h]);
                        // buffer.push(pixels[((y * scale_y) + h) as Option<usize>][((x * scale_x) + w) as Option<usize>]);
                    }
                }
                out_row.push(
                    buffer
                        .into_iter()
                        .map(|pair| {
                            if grayscale {
                                (pair.calc_penalty(), CustomColor::new(255, 255, 255))
                            } else {
                                let x = pair.channels();
                                (pair.calc_penalty(), CustomColor::new(x[0], x[1], x[2]))
                            }
                        })
                        .collect::<Vec<(u8, CustomColor)>>()
                        .average(),
                );
            }
            output.push(out_row);
        }

        // println!("{:?}", output);
        // todo!();

        Ok(AsciiImg {
            height: target_height,
            width: target_width,
            pixels: output
                .iter()
                .map(|vec| {
                    vec.iter()
                        .map(|px| DensityChar::get_char_from_u8(px.0, invert, px.1, uniform))
                        .collect::<Vec<ColorChar>>()
                })
                .collect::<Vec<Vec<ColorChar>>>(),
        })
    }

    #[rustfmt::skip]
    pub fn new_parallel(
        path: String,
        target_height: Option<u32>,
        target_width: Option<u32>,
        invert: bool,
        grayscale: bool,
        uniform: bool,
    ) -> Result<AsciiImg, Box<dyn Error>> {
        // gets image and converts it into grayscale if needed
        let img = {
            let img = ImageReader::open(path)?.decode()?;
            if grayscale {
                img.grayscale()
            } else {
                img
            }
        };

        let (src_width, src_height) = img.dimensions();

        let (final_width, final_height) =
            (target_width, target_height).demure_unwrap(src_width, src_height);
            // (target_height, target_width).demure_unwrap(src_width, src_height);
        let (final_width, final_height) = (final_width as usize, final_height as usize);

        // ------

        // creates and populates the Vec<Vec<_>> that holds the pixels
        let mut pixels = vec![vec![Rgba([0_u8; 4]); src_width as usize]; src_height as usize];

        // Parallelize over rows (outer Vec)
        pixels
            .par_iter_mut()
            .enumerate() // Get both the row index and the mutable reference
            .for_each(|(row_idx, row)| {
                for col_idx in 0..src_width {
                    // Calculate the corresponding (x, y) in the original image
                    // let x = col_idx.min(img.width() as usize - 1);
                    // let y = row_idx.min(img.height() as usize - 1);

                    // Assign the pixel value to the corresponding cell
                    row[col_idx as usize] = img.get_pixel(col_idx as u32, row_idx as u32);
                }
            });


        // ------

        let output = algo_parallel(pixels, src_height, src_width, final_height, final_width, grayscale, invert, uniform);
        
        Ok(AsciiImg {
            height: target_height.and_then(|u| Some(u as usize)), 
            width: target_width.and_then(|u| Some(u as usize)), 
            pixels: output 
        })
    }
}

pub trait DemureUnwrap<T> {
    fn demure_unwrap(&self, src_width: T, src_height: T) -> (T, T);
}

impl DemureUnwrap<u32> for (Option<u32>, Option<u32>) {
    fn demure_unwrap(&self, src_width: u32, src_height: u32) -> (u32, u32) {
        match self {
            (None, None) => return (src_width, src_height),
            (None, Some(height)) => {
                let ratio = src_height as f32 / src_width as f32;
                let height = *height;

                (
                    (((height as f32 / ratio) * 2.0).ceil() as u32).clamp(1, src_height),
                    height,
                )
            }
            (Some(width), None) => {
                let ratio = src_height as f32 / src_width as f32;
                let width = *width;

                (
                    width,
                    (((width as f32 * ratio) / 2.0).ceil() as u32).clamp(1, src_height),
                )
            }
            (Some(width), Some(height)) => (*width, *height),
        }
    }
}

pub trait Average {
    type Output;
    fn average(&self) -> Self::Output;
}

impl Average for Vec<u8> {
    type Output = u8;
    fn average(&self) -> Self::Output {
        let len = self.len() as f32;
        ((self.iter().map(|l| Into::<u32>::into(*l)).sum::<u32>() as f32) / len) as u8
    }
}

impl Average for Vec<CustomColor> {
    type Output = CustomColor;

    fn average(&self) -> Self::Output {
        let len = self.len() as f32;
        let mut r: usize = 0;
        let mut g: usize = 0;
        let mut b: usize = 0;

        self.iter().for_each(|k| {
            r += k.r as usize;
            g += k.g as usize;
            b += k.b as usize;
        });

        CustomColor::new(
            (r as f32 / len).round_ties_even() as u8,
            (g as f32 / len).round_ties_even() as u8,
            (b as f32 / len).round_ties_even() as u8,
        )
    }
}

impl Average for Vec<(u8, CustomColor)> {
    type Output = (u8, CustomColor);

    fn average(&self) -> Self::Output {
        let (x, y): (Vec<u8>, Vec<CustomColor>) = self.clone().into_iter().unzip();

        let x = x.average();
        let y = y.average();

        (x, y)
    }
}

pub trait GenAscii {
    fn gen_ascii(&self) -> String;
}

impl GenAscii for Vec<Vec<char>> {
    fn gen_ascii(&self) -> String {
        self.into_par_iter()
            .map(|vec| {
                vec.into_par_iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl GenAscii for Vec<Vec<ColorChar>> {
    fn gen_ascii(&self) -> String {
        self.iter()
            .map(|vec| {
                vec.iter()
                    .map(|c| c.to_string())
                    .filter(|t| !t.contains("[38;2;0;0;0m "))
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

pub trait Penalty {
    fn calc_penalty(&self) -> u8;
}

impl Penalty for (u8, u8) {
    fn calc_penalty(&self) -> u8 {
        let jkl = 255;
        let x = self.0 as i32 - (jkl as i32 - self.1 as i32);
        x.max(0) as u8
    }
}

impl Penalty for Rgba<u8> {
    fn calc_penalty(&self) -> u8 {
        let things = self.channels();

        (things[0], things[3]).calc_penalty()
    }
}

pub fn convert(
    path: String,
    target_height: Option<u32>,
    target_width: Option<u32>,
    invert: bool,
    grayscale: bool,
    uniform: bool,
    paralleled: bool
) -> Result<String, Box<dyn Error>> {
    let ascii: AsciiImg;
    
    if paralleled {
        ascii = AsciiImg::new_parallel(
            path,
            target_height,
            target_width,
            invert,
            grayscale,
            uniform,
        )?;
    } else {
        ascii = AsciiImg::new(
            path,
            target_height.and_then(|x| Some(x as usize)),
            target_width.and_then(|x| Some(x as usize)),
            invert,
            grayscale,
            uniform,
        )?;
    }

    Ok(ascii.pixels.gen_ascii())
}
