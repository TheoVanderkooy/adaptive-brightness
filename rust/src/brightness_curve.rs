/// Brightness curve
///
/// Represents a piece-wise linear function and can be evaluated at a point.

pub struct BrightnessCurve {
    curve: Vec<(u32, u32)>,
}

impl BrightnessCurve {
    /// Initialize a brightness curve from a vector of (lux value, desired brightness) pairs
    ///
    /// Preconditions:
    ///  - There should not be duplicate lux values
    ///  - The input should not be empty
    ///  - Ideally they should be ordered but this is not necessary
    ///  - Brightness values should be in 0..=100
    pub fn from_steps(mut curve_steps: Vec<(u32, u32)>) -> Self {
        curve_steps.sort();

        BrightnessCurve { curve: curve_steps }
    }

    pub fn target_brightness(&self, lux: u32) -> u32 {
        // Interpolate based on the piece-wise linear curve
        match self.curve.iter().position(|p| p.0 > lux) {
            // boundary: lux >= all points in the curve, take last brightness value
            // this could also mean the curve is an empty list, in which case default to 100% brightness
            None => self.curve.last().unwrap_or(&(0, 100)).1,
            // boundary: lux <= all points in the curve, take first value
            Some(0) => self.curve[0].1,
            // lux is somewhere within the curve,
            Some(i) => {
                let (llux, lbr) = self.curve[i - 1];
                let (rlux, rbr) = self.curve[i];

                let b =
                    lbr as f64 + (rbr - lbr) as f64 * (lux - llux) as f64 / (rlux - llux) as f64;
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
        let curve = BrightnessCurve::from_steps(vec![]);
        assert_eq!(100, curve.target_brightness(0));
        assert_eq!(100, curve.target_brightness(100));
        assert_eq!(100, curve.target_brightness(1000));
    }

    #[test]
    fn single_value_curve() {
        let curve = BrightnessCurve::from_steps(vec![(50, 42)]);
        assert_eq!(42, curve.target_brightness(0));
        assert_eq!(42, curve.target_brightness(50));
        assert_eq!(42, curve.target_brightness(100));
    }

    #[test]
    fn double_value_curve() {
        let curve = BrightnessCurve::from_steps(vec![(50, 30), (250, 80)]);
        assert_eq!(30, curve.target_brightness(0));
        assert_eq!(30, curve.target_brightness(20));
        assert_eq!(30, curve.target_brightness(50));
        assert_eq!(42.5 as u32, curve.target_brightness(100));
        assert_eq!(55, curve.target_brightness(150));
        assert_eq!(67.5 as u32, curve.target_brightness(200));
        assert_eq!(80, curve.target_brightness(250));
        assert_eq!(80, curve.target_brightness(300));
    }

    #[test]
    fn multi_value_curve() {
        let curve = BrightnessCurve::from_steps(vec![(0, 0), (100, 20), (150, 50), (250, 100)]);
        assert_eq!(0, curve.target_brightness(0));
        assert_eq!(5, curve.target_brightness(25));
        assert_eq!(10, curve.target_brightness(50));
        assert_eq!(15, curve.target_brightness(75));
        assert_eq!(20, curve.target_brightness(100));
        assert_eq!(35, curve.target_brightness(125));
        assert_eq!(50, curve.target_brightness(150));
        assert_eq!(55, curve.target_brightness(160));
        assert_eq!(60, curve.target_brightness(170));
        assert_eq!(65, curve.target_brightness(180));
        assert_eq!(70, curve.target_brightness(190));
        assert_eq!(75, curve.target_brightness(200));
        assert_eq!(80, curve.target_brightness(210));
        assert_eq!(85, curve.target_brightness(220));
        assert_eq!(90, curve.target_brightness(230));
        assert_eq!(95, curve.target_brightness(240));
        assert_eq!(100, curve.target_brightness(250));
        assert_eq!(100, curve.target_brightness(300));
    }

    #[test]
    fn unordered_value_curve() {
        let curve = BrightnessCurve::from_steps(vec![
            (10, 10),
            (100, 100),
            (0, 0),
            (40, 40),
            (80, 80),
            (60, 60),
        ]);
        for i in 0..=10 {
            assert_eq!(10 * i, curve.target_brightness(10 * i));
        }
    }
}
