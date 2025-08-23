/// Represents a piece-wise linear function and can be evaluated at a point.

#[derive(Debug)]
pub struct PiecewiseLinear {
    curve: Vec<(u32, u32)>,
}

/*
TODO:
 - make generic over input/output type?
*/

impl PiecewiseLinear {
    /// Initialize a piecewise linear function from a vector of (input, output) pairs
    ///
    /// Preconditions:
    ///  - There should not be duplicate input values
    ///  - The input should not be empty
    ///  - Ideally they should be ordered but this is not necessary
    pub fn from_steps(mut curve_steps: Vec<(u32, u32)>) -> Option<Self> {
        curve_steps.sort();

        // Invalid inputs:
        if curve_steps.is_empty() {
            return None;
        }

        Some(PiecewiseLinear { curve: curve_steps })
    }

    /// Evaluate the piecewise linear function at a given point.
    pub fn eval(&self, x: u32) -> u32 {
        // Interpolate based on the piece-wise linear curve
        match self.curve.iter().position(|p| p.0 > x) {
            // boundary: x >= all points in the curve, take last y value
            // we require the list to be non-empty on construction, so panic if it is
            None => self.curve.last().unwrap().1,
            // boundary: x <= all points in the curve, take first value
            Some(0) => self.curve[0].1,
            // x is somewhere within the curve,
            Some(i) => {
                let (lx, ly) = self.curve[i - 1];
                let (rd, ry) = self.curve[i];

                let b = ly as f64 + (ry - ly) as f64 * (x - lx) as f64 / (rd - lx) as f64;
                b as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_curve() {
        let curve = PiecewiseLinear::from_steps(vec![]);
        assert!(curve.is_none());
    }

    #[test]
    fn single_value_curve() {
        let curve = PiecewiseLinear::from_steps(vec![(50, 42)]).unwrap();
        assert_eq!(42, curve.eval(0));
        assert_eq!(42, curve.eval(50));
        assert_eq!(42, curve.eval(100));
    }

    #[test]
    fn double_value_curve() {
        let curve = PiecewiseLinear::from_steps(vec![(50, 30), (250, 80)]).unwrap();
        assert_eq!(30, curve.eval(0));
        assert_eq!(30, curve.eval(20));
        assert_eq!(30, curve.eval(50));
        assert_eq!(42.5 as u32, curve.eval(100));
        assert_eq!(55, curve.eval(150));
        assert_eq!(67.5 as u32, curve.eval(200));
        assert_eq!(80, curve.eval(250));
        assert_eq!(80, curve.eval(300));
    }

    #[test]
    fn multi_value_curve() {
        let curve =
            PiecewiseLinear::from_steps(vec![(0, 0), (100, 20), (150, 50), (250, 100)]).unwrap();
        assert_eq!(0, curve.eval(0));
        assert_eq!(5, curve.eval(25));
        assert_eq!(10, curve.eval(50));
        assert_eq!(15, curve.eval(75));
        assert_eq!(20, curve.eval(100));
        assert_eq!(35, curve.eval(125));
        assert_eq!(50, curve.eval(150));
        assert_eq!(55, curve.eval(160));
        assert_eq!(60, curve.eval(170));
        assert_eq!(65, curve.eval(180));
        assert_eq!(70, curve.eval(190));
        assert_eq!(75, curve.eval(200));
        assert_eq!(80, curve.eval(210));
        assert_eq!(85, curve.eval(220));
        assert_eq!(90, curve.eval(230));
        assert_eq!(95, curve.eval(240));
        assert_eq!(100, curve.eval(250));
        assert_eq!(100, curve.eval(300));
    }

    #[test]
    fn unordered_value_curve() {
        let curve = PiecewiseLinear::from_steps(vec![
            (10, 10),
            (100, 100),
            (0, 0),
            (40, 40),
            (80, 80),
            (60, 60),
        ])
        .unwrap();
        for i in 0..=10 {
            assert_eq!(10 * i, curve.eval(10 * i));
        }
    }
}
