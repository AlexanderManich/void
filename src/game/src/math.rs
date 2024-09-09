use std::ops;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, op: Vec2) -> f32 {
        self.x * op.x + self.y * op.y
    }
}

impl ops::Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ops::AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        let result = self;

        *result = *result + rhs
    }
}

impl ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl ops::Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl ops::Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

// This might cause issues idk
impl ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        let result = self;

        *result = *result * rhs;
    }
}

impl ops::Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

pub fn square(a: f32) -> f32 {
    a * a
}

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn add_assign() {
        let mut result = Vec2::new(1.0, 2.0);
        result *= 2.0;

        assert_eq!(result, Vec2::new(2.0, 4.0))
    }

    #[test]
    fn add() {
        let lhs = Vec2::new(1.0, 1.0);
        let rhs = Vec2::new(1.0, 1.0);

        assert_eq!(lhs + rhs, Vec2::new(2.0, 2.0));
        assert_eq!(lhs, Vec2::new(1.0, 1.0));
        assert_eq!(rhs, Vec2::new(1.0, 1.0));
    }

    #[test]
    fn mul() {
        let lhs = Vec2::new(1.0, 1.0);

        assert_eq!(lhs * 2.0, Vec2::new(2.0, 2.0));
        assert_eq!(lhs, Vec2::new(1.0, 1.0));
    }

    #[test]
    fn dot() {
        let vec = Vec2::new(2.0, 3.0);
        let op = Vec2::new(-4.0, -5.0);

        let result = vec.dot(op);

        assert_eq!(result, -23.0);
        assert_eq!(vec, Vec2::new(2.0, 3.0));
        assert_eq!(op, Vec2::new(-4.0, -5.0));
    }

    #[test]
    fn default() {
        let result = Vec2::default();

        assert_eq!(result, Vec2::new(0.0, 0.0));
    }
}
