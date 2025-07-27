use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Curve(Vec<CurvePoint>);

impl Curve {
    pub fn new(points: Vec<CurvePoint>) -> Self {
        Self(points)
    }
    pub fn apply(&self, point: i32) -> i32 {
        let points = self.0.as_slice();
        points
            .iter()
            .enumerate()
            .map(|(i, point_i)| {
                points
                    .iter()
                    .enumerate()
                    .fold(point_i.y, |pre, (j, point_j)| {
                        if j == i {
                            pre
                        } else {
                            pre * (point - point_j.x) / (point_i.x - point_j.x)
                        }
                    })
            })
            .sum()
    }
}

#[derive(Debug, Deserialize)]
pub struct CurvePoint {
    pub x: i32,
    pub y: i32,
}

#[cfg(test)]
mod tests {
    use super::{Curve, CurvePoint};

    #[test]
    fn t() {
        let curve = Curve::new(vec![
            CurvePoint { x: 0, y: 0 },
            CurvePoint { x: 100, y: 50 },
        ]);
        for i in 0..=200 {
            println!("{i}: {}", curve.apply(i));
        }
        assert_eq!(curve.apply(0), 0);
        assert_eq!(curve.apply(100), 50);
        assert_eq!(curve.apply(50), 25);
    }
}
