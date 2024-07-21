#![no_std]

// These math functions come from https://github.com/tarcieri/micromath/blob/main/src/float/floor.rs
#[inline]
pub fn floor(x: f32) -> f32 {
    let res = (x as i32) as f32;

    if x < res {
        res - 1.0
    } else {
        res
    }
}
