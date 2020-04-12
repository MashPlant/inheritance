#![feature(arbitrary_enum_discriminant)]
extern crate inheritance;

inheritance::inheritance!(
  // note: keep privacy notation
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

  pub(crate) struct Triangle : Shape {
    a: f32,
    b: f32,
    c: f32
  }

  pub struct SpecialCircleA : Circle {
    data: u32,
  }

  pub struct SpecialCircleB : Circle {
    data: u64,
  }

  pub struct SpecialRect : Rect {}
);


fn main() {}