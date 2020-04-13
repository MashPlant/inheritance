#![feature(arbitrary_enum_discriminant)]
extern crate inheritance;

inheritance::inheritance!(
  pub struct Shape {
    pub area: f32,
    pub circumference: f32
  }

  pub struct Circle: Shape {
    pub(crate) radius: f32
  }

  pub struct Rect: Shape {
    width: f32,
    height: f32
  }

  pub struct Triangle: Shape {
    a: f32,
    b: f32,
    c: f32
  }

  pub struct CircleA: Circle {
    data: u32,
  }

  pub struct CircleB: Circle {
    data: u64,
  }

  pub struct RectX: Rect {}

  pub struct RectY: RectX {}
);

fn work(s: &GenericShape) {
  if let Some(ca) = s.downcast::<CircleACircle>() {
    println!("CircleA, data = {}", ca.data);
  }
  if let Some(c) = s.downcast::<GenericCircle>() {
    println!("Circle, radius = {}", c.radius);
  }
  if let Some(t) = s.downcast::<TriangleShape>() {
    println!("Triangle, b = {}", t.b);
  }
  if let Some(_) = s.downcast::<GenericRectX>() {
    println!("RectX");
  }
  if let Some(_) = s.downcast::<RectYRectX>() {
    println!("RectY");
  }
}

fn main() {
  use std::f32::consts::{PI, SQRT_2};
  let ca = CircleACircle::new(Circle::new(Shape { area: PI, circumference: 2.0 * PI }, 1.0), 123);
  work(ca.upcast().upcast());
  let t = TriangleShape::new(Shape { area: 0.2, circumference: 2.0 + SQRT_2 }, 1.0, 1.0, SQRT_2);
  work(t.upcast());
  let r = RectYRectX::new(RectX::new(Rect::new(Shape { area: 1.0, circumference: 4.0 }, 1.0, 1.0)));
  work(r.upcast().upcast().upcast());
}