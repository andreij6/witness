#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(&self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(&self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub const fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let (x0, y0) = (self.x.max(other.x), self.y.max(other.y));
        let (x1, y1) = (
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        );
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }

    pub fn bounding_union(&self, other: &Rect) -> Rect {
        let (x0, y0) = (self.x.min(other.x), self.y.min(other.y));
        let (x1, y1) = (
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        );
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    pub fn clamp_to(&self, width: u32, height: u32) -> Option<Rect> {
        self.intersection(&Rect::new(0, 0, width, height))
    }

    pub fn expand(&self, dx: u32, dy: u32, width: u32, height: u32) -> Rect {
        let x0 = self.x.saturating_sub(dx);
        let y0 = self.y.saturating_sub(dy);
        let x1 = self.right().saturating_add(dx).min(width).max(x0);
        let y1 = self.bottom().saturating_add(dy).min(height).max(y0);
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Similarity {
    pub a: f64,
    pub b: f64,
    pub tx: f64,
    pub ty: f64,
    pub mirrored: bool,
}

impl Similarity {
    pub const IDENTITY: Similarity = Similarity {
        a: 1.0,
        b: 0.0,
        tx: 0.0,
        ty: 0.0,
        mirrored: false,
    };

    pub const fn new(a: f64, b: f64, tx: f64, ty: f64, mirrored: bool) -> Self {
        Self {
            a,
            b,
            tx,
            ty,
            mirrored,
        }
    }

    pub const fn scale_translate(scale: f64, tx: f64, ty: f64) -> Self {
        Self::new(scale, 0.0, tx, ty, false)
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        if self.mirrored {
            (
                self.a * x + self.b * y + self.tx,
                self.b * x - self.a * y + self.ty,
            )
        } else {
            (
                self.a * x - self.b * y + self.tx,
                self.b * x + self.a * y + self.ty,
            )
        }
    }

    pub fn scale(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    pub fn is_finite(&self) -> bool {
        self.a.is_finite() && self.b.is_finite() && self.tx.is_finite() && self.ty.is_finite()
    }

    pub fn inverse(&self) -> Option<Similarity> {
        let det = self.a * self.a + self.b * self.b;
        if !det.is_finite() || det <= 0.0 {
            return None;
        }
        let (a, b) = if self.mirrored {
            (self.a / det, self.b / det)
        } else {
            (self.a / det, -self.b / det)
        };
        let partial = Similarity::new(a, b, 0.0, 0.0, self.mirrored);
        let (tx, ty) = partial.apply(self.tx, self.ty);
        Some(Similarity::new(a, b, -tx, -ty, self.mirrored))
    }

    pub fn mirror_x(&self, width: f64) -> Similarity {
        Similarity::new(-self.a, self.b, width - self.tx, self.ty, !self.mirrored)
    }

    pub fn resize(from: (u32, u32), to: (u32, u32)) -> Option<Similarity> {
        let (fw, fh) = (f64::from(from.0), f64::from(from.1));
        let (tw, th) = (f64::from(to.0), f64::from(to.1));
        if fw == 0.0 || fh == 0.0 || tw == 0.0 || th == 0.0 {
            return None;
        }
        let s = (tw + th) / (fw + fh);
        ((s * fw - tw).abs() <= 1.0 && (s * fh - th).abs() <= 1.0)
            .then_some(Similarity::scale_translate(s, 0.0, 0.0))
    }

    pub fn max_corner_shift(&self, other: &Similarity, width: u32, height: u32) -> f64 {
        if !(self.is_finite() && other.is_finite()) {
            return f64::INFINITY;
        }
        let (w, h) = (f64::from(width), f64::from(height));
        [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)]
            .into_iter()
            .map(|(x, y)| {
                let (p, q) = (self.apply(x, y), other.apply(x, y));
                let (dx, dy) = (p.0 - q.0, p.1 - q.1);
                (dx * dx + dy * dy).sqrt()
            })
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn rect_edges_and_area() {
        let r = Rect::new(2, 3, 4, 5);
        assert_eq!((r.right(), r.bottom(), r.area()), (6, 8, 20));
        assert!(!r.is_empty());
        assert!(Rect::new(1, 1, 0, 3).is_empty());
        assert_eq!(Rect::new(u32::MAX, 0, 10, 1).right(), u32::MAX);
    }

    #[test]
    fn rect_contains_points_half_open() {
        let r = Rect::new(2, 3, 4, 5);
        assert!(r.contains(2, 3));
        assert!(r.contains(5, 7));
        assert!(!r.contains(6, 7));
        assert!(!r.contains(5, 8));
        assert!(!r.contains(1, 3));
        assert!(!Rect::new(0, 0, 0, 0).contains(0, 0));
    }

    #[test]
    fn rect_containment_and_intersection() {
        let outer = Rect::new(0, 0, 10, 10);
        let inner = Rect::new(2, 2, 3, 3);
        assert!(outer.contains_rect(&inner));
        assert!(!inner.contains_rect(&outer));
        assert_eq!(outer.intersection(&inner), Some(inner));
        assert_eq!(
            Rect::new(0, 0, 5, 5).intersection(&Rect::new(3, 4, 5, 5)),
            Some(Rect::new(3, 4, 2, 1))
        );
        assert_eq!(
            Rect::new(0, 0, 5, 5).intersection(&Rect::new(5, 0, 5, 5)),
            None
        );
        assert_eq!(
            Rect::new(0, 0, 5, 5).bounding_union(&Rect::new(7, 8, 1, 1)),
            Rect::new(0, 0, 8, 9)
        );
    }

    #[test]
    fn rect_clamp_and_expand() {
        assert_eq!(
            Rect::new(8, 8, 10, 10).clamp_to(12, 10),
            Some(Rect::new(8, 8, 4, 2))
        );
        assert_eq!(Rect::new(20, 0, 1, 1).clamp_to(12, 10), None);
        assert_eq!(
            Rect::new(1, 1, 2, 2).expand(3, 1, 10, 10),
            Rect::new(0, 0, 6, 4)
        );
        assert_eq!(
            Rect::new(8, 8, 2, 2).expand(5, 5, 10, 10),
            Rect::new(3, 3, 7, 7)
        );
        assert_eq!(
            Rect::new(20, 20, 2, 2).expand(1, 1, 10, 10),
            Rect::new(19, 19, 0, 0)
        );
    }

    #[test]
    fn identity_is_neutral() {
        assert_eq!(Similarity::IDENTITY.apply(3.5, -2.0), (3.5, -2.0));
        assert_eq!(Similarity::IDENTITY.inverse(), Some(Similarity::IDENTITY));
        assert_eq!(Similarity::IDENTITY.scale(), 1.0);
    }

    #[test]
    fn rotation_scale_applies() {
        let t = Similarity::new(0.0, 2.0, 1.0, 1.0, false);
        assert!(close(t.apply(1.0, 0.0), (1.0, 3.0)));
        assert!(close(t.apply(0.0, 1.0), (-1.0, 1.0)));
        assert_eq!(t.scale(), 2.0);
    }

    #[test]
    fn mirrored_reflects() {
        let t = Similarity::new(1.0, 0.0, 0.0, 0.0, true);
        assert_eq!(t.apply(2.0, 3.0), (2.0, -3.0));
    }

    #[test]
    fn inverse_round_trips_both_handednesses() {
        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 2001) as f64 / 100.0 - 10.0
        };
        for i in 0..200 {
            let t = Similarity::new(
                next() / 5.0 + 0.01,
                next() / 5.0,
                next() * 10.0,
                next() * 10.0,
                i % 2 == 0,
            );
            let inv = t.inverse().unwrap();
            let p = (next(), next());
            let (x, y) = t.apply(p.0, p.1);
            let back = inv.apply(x, y);
            assert!(
                (back.0 - p.0).abs() < 1e-6 && (back.1 - p.1).abs() < 1e-6,
                "{t:?}"
            );
            assert_eq!(inv.mirrored, t.mirrored);
        }
    }

    #[test]
    fn degenerate_inverse_is_none() {
        assert_eq!(Similarity::new(0.0, 0.0, 1.0, 1.0, false).inverse(), None);
        assert_eq!(
            Similarity::new(f64::NAN, 0.0, 1.0, 1.0, false).inverse(),
            None
        );
        assert_eq!(
            Similarity::new(f64::INFINITY, 0.0, 0.0, 0.0, true).inverse(),
            None
        );
        assert!(!Similarity::new(f64::NAN, 0.0, 0.0, 0.0, false).is_finite());
        assert!(Similarity::IDENTITY.is_finite());
    }

    #[test]
    fn mirror_x_composes_horizontal_flip() {
        for mirrored in [false, true] {
            let t = Similarity::new(0.8, 0.3, 5.0, -2.0, mirrored);
            let m = t.mirror_x(100.0);
            for p in [(0.0, 0.0), (10.0, 3.0), (-4.0, 7.5)] {
                let (x, y) = t.apply(p.0, p.1);
                assert!(close(m.apply(p.0, p.1), (100.0 - x, y)));
            }
            assert_eq!(m.mirrored, !mirrored);
            assert!(close(m.mirror_x(100.0).apply(1.0, 2.0), t.apply(1.0, 2.0)));
        }
    }

    #[test]
    fn scale_translate_constructor() {
        let t = Similarity::scale_translate(2.0, 1.0, -1.0);
        assert_eq!(t.apply(1.0, 1.0), (3.0, 1.0));
    }

    #[test]
    fn resize_maps_matching_aspect_only() {
        assert_eq!(
            Similarity::resize((192, 144), (192, 144)),
            Some(Similarity::IDENTITY)
        );
        let down = Similarity::resize((4000, 3000), (2048, 1536)).unwrap();
        assert!(close(down.apply(4000.0, 3000.0), (2048.0, 1536.0)));
        assert!(!down.mirrored);
        assert_eq!(down.b, 0.0);
        let rounded = Similarity::resize((1001, 750), (512, 384)).unwrap();
        let (x, y) = rounded.apply(1001.0, 750.0);
        assert!((x - 512.0).abs() <= 1.0 && (y - 384.0).abs() <= 1.0);
        assert!(Similarity::resize((192, 144), (144, 192)).is_none());
        assert!(Similarity::resize((192, 144), (192, 120)).is_none());
        assert!(Similarity::resize((1000, 1000), (1000, 990)).is_none());
    }

    #[test]
    fn resize_rejects_empty_dimensions() {
        assert!(Similarity::resize((0, 10), (10, 10)).is_none());
        assert!(Similarity::resize((10, 10), (10, 0)).is_none());
        assert!(Similarity::resize((0, 0), (0, 0)).is_none());
        assert!(Similarity::resize((u32::MAX, u32::MAX), (u32::MAX, u32::MAX)).is_some());
    }

    #[test]
    fn resize_is_deterministic() {
        for (f, t) in [
            ((97, 73), (48, 36)),
            ((2048, 1536), (96, 72)),
            ((7, 5), (7, 5)),
        ] {
            assert_eq!(Similarity::resize(f, t), Similarity::resize(f, t));
        }
    }

    #[test]
    fn corner_shift_measures_worst_corner() {
        let i = Similarity::IDENTITY;
        assert_eq!(i.max_corner_shift(&i, 100, 50), 0.0);
        let shifted = Similarity::scale_translate(1.0, 3.0, 4.0);
        assert_eq!(i.max_corner_shift(&shifted, 100, 50), 5.0);
        assert_eq!(shifted.max_corner_shift(&i, 100, 50), 5.0);
        let rotated = Similarity::new(1.0, 0.01, 0.0, 0.0, false);
        let worst = i.max_corner_shift(&rotated, 100, 50);
        assert!((worst - (100.0f64.powi(2) + 50.0f64.powi(2)).sqrt() * 0.01).abs() < 1e-9);
        assert_eq!(i.max_corner_shift(&i, 0, 0), 0.0);
    }

    #[test]
    fn corner_shift_of_non_finite_is_infinite() {
        let bad = Similarity::new(f64::NAN, 0.0, 0.0, 0.0, false);
        assert_eq!(
            bad.max_corner_shift(&Similarity::IDENTITY, 10, 10),
            f64::INFINITY
        );
        assert_eq!(
            Similarity::IDENTITY.max_corner_shift(&bad, 10, 10),
            f64::INFINITY
        );
        let huge = Similarity::new(f64::INFINITY, 0.0, 0.0, 0.0, false);
        assert_eq!(
            huge.max_corner_shift(&Similarity::IDENTITY, 1, 1),
            f64::INFINITY
        );
    }

    #[test]
    fn mirrored_transform_is_far_from_its_unmirrored_twin() {
        let m = Similarity::IDENTITY.mirror_x(100.0);
        assert!(m.max_corner_shift(&Similarity::IDENTITY, 100, 50) >= 100.0);
    }
}
