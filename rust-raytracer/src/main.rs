mod camera;
mod hittable;
mod material;
mod ray;
mod sphere;
mod vec3;

use camera::Camera;
use hittable::{Hittable, HittableList};
use material::{Dielectric, Lambertian, Metal};
use ray::Ray;
use rayon::prelude::*;
use sphere::Sphere;
use std::time::Instant;
use vec3::{color_to_rgb, Color, Vec3};

fn ray_color(ray: &Ray, world: &dyn Hittable, depth: i32) -> Color {
    if depth <= 0 {
        return Color::zero();
    }

    if let Some(rec) = world.hit(ray, 0.001, f64::INFINITY) {
        if let Some((attenuation, scattered)) = rec.material.scatter(ray, &rec) {
            return attenuation * ray_color(&scattered, world, depth - 1);
        }
        return Color::zero();
    }

    // Sky gradient
    let unit_direction = ray.direction.unit();
    let t = 0.5 * (unit_direction.y + 1.0);
    Color::new(1.0, 1.0, 1.0) * (1.0 - t) + Color::new(0.5, 0.7, 1.0) * t
}

fn random_scene() -> HittableList {
    let mut world = HittableList::new();
    // Ground
    world.add(Box::new(Sphere::new(
        Vec3::new(0.0, -1000.0, 0.0),
        1000.0,
        Box::new(Lambertian::new(Color::new(0.5, 0.5, 0.5))),
    )));

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat: f64 = rand::random();
            let center = Vec3::new(
                a as f64 + 0.9 * rand::random::<f64>(),
                0.2,
                b as f64 + 0.9 * rand::random::<f64>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                let material: Box<dyn material::Material> = if choose_mat < 0.8 {
                    // Diffuse
                    let albedo = Color::random(0.0, 1.0) * Color::random(0.0, 1.0);
                    Box::new(Lambertian::new(albedo))
                } else if choose_mat < 0.95 {
                    // Metal
                    let albedo = Color::random(0.5, 1.0);
                    let fuzz = rand::random::<f64>() * 0.5;
                    Box::new(Metal::new(albedo, fuzz))
                } else {
                    // Glass
                    Box::new(Dielectric::new(1.5))
                };

                world.add(Box::new(Sphere::new(center, 0.2, material)));
            }
        }
    }

    // Three big spheres
    world.add(Box::new(Sphere::new(
        Vec3::new(0.0, 1.0, 0.0),
        1.0,
        Box::new(Dielectric::new(1.5)),
    )));

    world.add(Box::new(Sphere::new(
        Vec3::new(-4.0, 1.0, 0.0),
        1.0,
        Box::new(Lambertian::new(Color::new(0.4, 0.2, 0.1))),
    )));

    world.add(Box::new(Sphere::new(
        Vec3::new(4.0, 1.0, 0.0),
        1.0,
        Box::new(Metal::new(Color::new(0.7, 0.6, 0.5), 0.0)),
    )));

    world
}

fn main() {
    // Image settings
    let aspect_ratio = 16.0 / 9.0;
    let image_width: u32 = 800;
    let image_height: u32 = (image_width as f64 / aspect_ratio) as u32;
    let samples_per_pixel: u32 = 100;
    let max_depth: i32 = 50;

    // Scene
    let world = random_scene();

    // Camera
    let lookfrom = Vec3::new(13.0, 2.0, 3.0);
    let lookat = Vec3::new(0.0, 0.0, 0.0);
    let vup = Vec3::new(0.0, 1.0, 0.0);
    let aperture = 0.1;
    let focus_dist = 10.0;

    let camera = Camera::new(lookfrom, lookat, vup, 20.0, aspect_ratio, aperture, focus_dist);

    println!(
        "Rendering {}x{} image with {} samples per pixel...",
        image_width, image_height, samples_per_pixel
    );
    let start = Instant::now();

    // Render rows in parallel with rayon
    let pixels: Vec<Vec<[u8; 3]>> = (0..image_height)
        .into_par_iter()
        .rev()
        .map(|j| {
            let mut row = Vec::with_capacity(image_width as usize);
            for i in 0..image_width {
                let mut pixel_color = Color::zero();
                for _ in 0..samples_per_pixel {
                    let u = (i as f64 + rand::random::<f64>()) / (image_width - 1) as f64;
                    let v = (j as f64 + rand::random::<f64>()) / (image_height - 1) as f64;
                    let ray = camera.get_ray(u, v);
                    pixel_color = pixel_color + ray_color(&ray, &world, max_depth);
                }
                row.push(color_to_rgb(pixel_color, samples_per_pixel));
            }

            // Progress (approximate since parallel)
            if j % 50 == 0 {
                eprintln!("  scanlines remaining: {}", j);
            }

            row
        })
        .collect();

    let elapsed = start.elapsed();
    println!("Render completed in {:.2}s", elapsed.as_secs_f64());

    // Write to PNG
    let mut img = image::RgbImage::new(image_width, image_height);
    for (y, row) in pixels.iter().enumerate() {
        for (x, pixel) in row.iter().enumerate() {
            img.put_pixel(x as u32, y as u32, image::Rgb(*pixel));
        }
    }

    let output_path = "output.png";
    img.save(output_path).expect("Failed to save image");
    println!("Saved to {}", output_path);
}