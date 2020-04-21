#![feature(arbitrary_enum_discriminant)]
extern crate inheritance;

inheritance::inheritance!(
  #[derive(Debug)]
  pub struct Shape {
    pub area: f32,
    pub circumference: f32
  }

  #[derive(Debug)]
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

  #[derive(Debug)]
  pub struct CircleA: Circle {
    data: u32,
  }

  pub struct CircleB: Circle {
    data: u64,
  }

  pub struct RectX: Rect {}

  pub struct RectY: RectX {}
);

fn test1(s: &GenericShape) {
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

fn test2(s: &mut GenericShape) {
  // do some arbitrary mutation, disrespecting the contract of `Shape`
  if let Some(c) = s.downcast_mut::<GenericCircle>() {
    c.radius = c.circumference;
  }
  if let Some(t) = s.downcast_mut::<TriangleShape>() {
    t.b = t.c * 2.0;
    t.a = t.b * 2.0;
  }
  if let Some(r) = s.downcast_mut::<RectYRectX>() {
    r.height *= 2.0;
  }
}

fn main() {
  use std::f32::consts::{PI, SQRT_2};
  let mut ca = CircleACircle::new(Circle::new(Shape { area: PI, circumference: 2.0 * PI }, 1.0), 123);
  let mut t = TriangleShape::new(Shape { area: 0.2, circumference: 2.0 + SQRT_2 }, 1.0, 1.0, SQRT_2);
  let mut r = RectYRectX::new(RectX::new(Rect::new(Shape { area: 1.0, circumference: 4.0 }, 1.0, 1.0)));
  test1(ca.upcast().upcast());
  test1(t.upcast());
  test1(r.upcast().upcast().upcast());
  // `upcast_mut` is inevitably unsafe, the user of this mut reference must ensure that it will not assign other variants to it
  unsafe {
    test2(ca.upcast_mut().upcast_mut());
    println!("{}", ca.circumference);
    test2(t.upcast_mut());
    println!("{} {} {}", t.a, t.b, t.c);
    test2(r.upcast_mut().upcast_mut().upcast_mut());
    println!("{}", r.height);
  }
  println!("{:?}", &*ca);
}