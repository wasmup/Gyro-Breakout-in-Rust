//! Fractals for the 320 × 240 startup screens.
const MAX_ITERATIONS: u8 = 40;

/// A Mandelbrot viewport. Each touch halves its width around the selected pixel.
pub struct Mandelbrot {
    real: f32,
    imaginary: f32,
    step: f32,
    zooms: u8,
}

impl Default for Mandelbrot {
    fn default() -> Self {
        Self {
            real: -2.2,
            imaginary: -1.125,
            step: 3.0 / 320.0,
            zooms: 0,
        }
    }
}

impl Mandelbrot {
    pub fn zoom(&mut self, x: i32, y: i32) -> bool {
        // Single-precision coordinates cannot reliably resolve pixels beyond this level.
        if self.zooms == 14 {
            return false;
        }
        let x = x.clamp(0, 319) as f32;
        let y = y.clamp(0, 239) as f32;
        self.real += (x - 80.0) * self.step;
        self.imaginary += (y - 60.0) * self.step;
        self.step *= 0.5;
        self.zooms += 1;
        true
    }

    pub fn pixel(&self, x: i32, y: i32) -> u16 {
        color(
            self.real + x as f32 * self.step,
            self.imaginary + y as f32 * self.step,
        )
    }
}

/// Julia set for c = -0.8 + 0.156i, using the same touch-centered zoom.
pub struct Julia {
    view: Mandelbrot,
}

impl Default for Julia {
    fn default() -> Self {
        Self {
            view: Mandelbrot {
                real: -1.6,
                imaginary: -1.2,
                step: 0.01,
                zooms: 0,
            },
        }
    }
}

impl Julia {
    pub fn zoom(&mut self, x: i32, y: i32) -> bool {
        self.view.zoom(x, y)
    }

    pub fn pixel(&self, x: i32, y: i32) -> u16 {
        let mut zx = self.view.real + x as f32 * self.view.step;
        let mut zy = self.view.imaginary + y as f32 * self.view.step;
        let mut count = 0u16;
        while zx * zx + zy * zy <= 4.0 && count < 96 {
            let next_x = zx * zx - zy * zy - 0.8;
            zy = 2.0 * zx * zy + 0.156;
            zx = next_x;
            count += 1;
        }
        if count == 96 {
            return 0;
        }
        // Bands of violet, cyan, and gold show successive escape times.
        let phase = count % 48;
        let red = if phase < 24 { phase } else { 47 - phase };
        let green = (count * 5) % 64;
        let blue = 31 - ((count * 3) % 32);
        (red << 11) | (green << 5) | blue
    }
}

/// Return the RGB565 color for a pixel of the startup image.
pub fn pixel(x: i32, y: i32) -> u16 {
    Mandelbrot::default().pixel(x, y)
}

fn color(real: f32, imaginary: f32) -> u16 {
    let (mut zx, mut zy) = (0.0f32, 0.0f32);
    let mut count = 0u8;
    while zx * zx + zy * zy <= 4.0 && count < MAX_ITERATIONS {
        let next_x = zx * zx - zy * zy + real;
        zy = 2.0 * zx * zy + imaginary;
        zx = next_x;
        count += 1;
    }
    if count == MAX_ITERATIONS {
        return 0;
    }
    // A repeating blue-to-gold palette in the display's RGB565 format.
    let red = (u16::from(count) * 3).min(31);
    let green = (u16::from(count) * 5).min(63);
    let blue = 31 - (u16::from(count) * 2).min(31);
    (red << 11) | (green << 5) | blue
}

/// Screen-space view of a plotted fractal, with up to three 2× zooms.
#[derive(Default)]
pub struct Viewport {
    left: f32,
    top: f32,
    zooms: u8,
}

impl Viewport {
    fn scale(&self) -> u32 {
        1 << self.zooms
    }

    pub fn zoom(&mut self, x: i32, y: i32) -> bool {
        // Bound fern sampling cost and snowflake recursion on the microcontroller.
        if self.zooms == 3 {
            return false;
        }
        let scale = self.scale() as f32;
        self.left += (x.clamp(0, 319) as f32 - 80.0) / scale;
        self.top += (y.clamp(0, 239) as f32 - 60.0) / scale;
        self.zooms += 1;
        true
    }

    fn project(&self, x: f32, y: f32) -> (f32, f32) {
        let scale = self.scale() as f32;
        ((x - self.left) * scale, (y - self.top) * scale)
    }
}

/// Plot a Koch snowflake, adding finer levels as the viewport is magnified.
pub fn snowflake(view: &Viewport, mut plot: impl FnMut(i32, i32)) {
    fn line(start: (f32, f32), end: (f32, f32), plot: &mut impl FnMut(i32, i32)) {
        let (mut x, mut y) = ((start.0 + 0.5) as i32, (start.1 + 0.5) as i32);
        let (x1, y1) = ((end.0 + 0.5) as i32, (end.1 + 0.5) as i32);
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            if (0..320).contains(&x) && (0..240).contains(&y) {
                plot(x, y);
            }
            if x == x1 && y == y1 {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x += sx;
            }
            if twice <= dx {
                error += dx;
                y += sy;
            }
        }
    }
    fn edge(start: (f32, f32), end: (f32, f32), depth: u8, plot: &mut impl FnMut(i32, i32)) {
        if depth == 0 {
            line(start, end, plot);
            return;
        }
        let dx = (end.0 - start.0) / 3.0;
        let dy = (end.1 - start.1) / 3.0;
        let first = (start.0 + dx, start.1 + dy);
        let last = (end.0 - dx, end.1 - dy);
        let tip = (
            first.0 + 0.5 * dx - 0.866_025_4 * dy,
            first.1 + 0.866_025_4 * dx + 0.5 * dy,
        );
        edge(start, first, depth - 1, plot);
        edge(first, tip, depth - 1, plot);
        edge(tip, last, depth - 1, plot);
        edge(last, end, depth - 1, plot);
    }
    let vertices = [(160.0, 22.0), (77.0, 165.0), (243.0, 165.0)].map(|(x, y)| view.project(x, y));
    for i in 0..3 {
        edge(
            vertices[i],
            vertices[(i + 1) % 3],
            4 + view.zooms,
            &mut plot,
        );
    }
}

/// Plot the counterclockwise-rotated fern in the selected viewport.
pub fn fern(view: &Viewport, mut plot: impl FnMut(i32, i32)) {
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let mut random = 0x1357_9bdfu32;
    let scale = view.scale();
    for iteration in 0..(50_000 * scale * scale + 20) {
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let choice = (random >> 16) % 100;
        let (next_x, next_y) = if choice == 0 {
            (0.0, 0.16 * y)
        } else if choice < 86 {
            (0.85 * x + 0.04 * y, -0.04 * x + 0.85 * y + 1.6)
        } else if choice < 93 {
            (0.2 * x - 0.26 * y, 0.23 * x + 0.22 * y + 1.6)
        } else {
            (-0.15 * x + 0.28 * y, 0.26 * x + 0.24 * y + 0.44)
        };
        (x, y) = (next_x, next_y);
        if iteration >= 20 {
            let original_x = 160.0 + x * 48.0;
            let original_y = 225.0 - y * 20.0;
            let (rotated_x, rotated_y) = if scale == 1 {
                let (rx, ry) = rotate_fern(original_x as i32, original_y as i32);
                (rx as f32, ry as f32)
            } else {
                // Scale before rounding so the zoom reveals subpixel detail.
                let rotated_x = original_y + 35.0;
                let rotated_y = 291.0 - original_x;
                view.project(rotated_x, rotated_y)
            };
            if (0.0..320.0).contains(&rotated_x) && (0.0..240.0).contains(&rotated_y) {
                plot(rotated_x as i32, rotated_y as i32);
            }
        }
    }
}

fn rotate_fern(x: i32, y: i32) -> (i32, i32) {
    // Rotate around the fern's center and translate to the display center.
    (160 + (y - 125), 120 - (x - 171))
}

#[cfg(test)]
mod tests {
    use super::{fern, pixel, rotate_fern, snowflake, Julia, Mandelbrot, Viewport};

    #[test]
    fn julia_has_detail_and_touch_zoom_centers_the_selected_point() {
        let mut julia = Julia::default();
        assert_ne!(julia.pixel(0, 0), julia.pixel(160, 120));
        let selected = julia.pixel(240, 80);
        assert!(julia.zoom(240, 80));
        assert_eq!(julia.pixel(160, 120), selected);
        for _ in 1..14 {
            assert!(julia.zoom(160, 120));
        }
        assert!(!julia.zoom(160, 120));
    }

    #[test]
    fn zoom_centers_selected_point_and_limits_depth() {
        let mut view = Mandelbrot::default();
        let selected = view.pixel(240, 80);
        assert!(view.zoom(240, 80));
        assert_eq!(view.pixel(160, 120), selected);
        for _ in 1..14 {
            assert!(view.zoom(160, 120));
        }
        assert!(!view.zoom(160, 120));
    }

    #[test]
    fn image_has_interior_and_colored_exterior() {
        assert_eq!(pixel(160, 120), 0);
        assert_ne!(pixel(0, 0), 0);
        assert_ne!(pixel(319, 0), 0);
        assert_eq!(pixel(200, 80), pixel(200, 160));
    }

    #[test]
    fn snowflake_stays_within_screen_and_plots_all_three_tips() {
        let mut count = 0;
        let mut tips = [false; 3];
        snowflake(&Viewport::default(), |x, y| {
            assert!((0..320).contains(&x) && (0..240).contains(&y));
            count += 1;
            for (found, vertex) in tips.iter_mut().zip([(160, 22), (77, 165), (243, 165)]) {
                *found |= (x, y) == vertex;
            }
        });
        assert!(count > 500);
        assert!(tips.iter().all(|found| *found));
    }

    #[test]
    fn fern_points_cover_the_display_without_leaving_it() {
        let (mut count, mut min_y, mut max_y) = (0, 240, 0);
        fern(&Viewport::default(), |x, y| {
            assert!((0..320).contains(&x) && (0..240).contains(&y));
            count += 1;
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        });
        assert_eq!(count, 50_000);
        assert!(min_y < 40 && max_y > 200);
    }

    #[test]
    fn fern_zoom_levels_stay_in_bounds_and_have_detail() {
        let mut view = Viewport::default();
        for zoom in [2, 4] {
            assert!(view.zoom(160, 120));
            let mut count = 0;
            fern(&view, |x, y| {
                assert!((0..320).contains(&x) && (0..240).contains(&y));
                count += 1;
            });
            assert!(count > 5_000, "zoom {zoom} has only {count} points");
        }
    }

    #[test]
    fn viewport_centers_taps_and_bounds_snowflake_points() {
        let mut view = Viewport::default();
        assert!(view.zoom(120, 80));
        assert_eq!(view.project(120.0, 80.0), (160.0, 120.0));
        assert!(view.zoom(200, 100));
        assert_eq!(view.project(140.0, 70.0), (160.0, 120.0));
        assert!(view.zoom(160, 120));
        assert!(!view.zoom(160, 120));
        snowflake(&view, |x, y| {
            assert!((0..320).contains(&x) && (0..240).contains(&y));
        });
    }

    #[test]
    fn fern_rotation_moves_top_to_left_and_base_to_right() {
        assert_eq!(rotate_fern(171, 25), (60, 120));
        assert_eq!(rotate_fern(171, 225), (260, 120));
        assert_eq!(rotate_fern(55, 125), (160, 236));
    }
}
