#![feature(arbitrary_enum_discriminant)]
extern crate inheritance;

inheritance::inheritance!(
  pub struct Shape {
    pub area: f32,
    pub circumference: f32
  }

  pub struct Circle : Shape {
    pub(crate) radius: f32
  }

  pub struct Rect : Shape {
    width: f32,
    height: f32
  }

  pub struct Triangle : Shape {
    a: f32,
    b: f32,
    c: f32
  }

  pub struct CircleA : Circle {
    data: u32,
  }

  pub struct CircleB : Circle {
    data: u64,
  }

  pub struct RectX : Rect {}

  pub struct RectY : RectX {}
);


fn main() {}