use std::ops::*;
use derive_is_enum_variant::is_enum_variant;

#[derive(PartialEq, Debug, is_enum_variant, Copy, Clone)]
pub enum VerticalAlign {
    Top(f32),
    Middle(f32),
    Bottom(f32)
}

impl VerticalAlign {
    pub fn same_with_value(self, value: f32) -> Self {
        match self {
            VerticalAlign::Top(_) => VerticalAlign::Top(value),
            VerticalAlign::Middle(_) => VerticalAlign::Middle(value),
            VerticalAlign::Bottom(_) => VerticalAlign::Bottom(value)
        }
    }

    pub fn inner(self) -> f32 {
        match self {
            VerticalAlign::Top(top) => top,
            VerticalAlign::Middle(middle) => middle,
            VerticalAlign::Bottom(bottom) => bottom
        }
    }

    pub fn absolute(self, height: f32) -> f32 {
        match self {
            VerticalAlign::Top(top) => top,
            VerticalAlign::Middle(middle) => height / 2.0 + middle,
            VerticalAlign::Bottom(bottom) => height - bottom
        }
    }
}

impl From<f32> for VerticalAlign {
    fn from(top: f32) -> Self {
        VerticalAlign::Top(top)
    }
}

impl Add<f32> for VerticalAlign {
    type Output = VerticalAlign;

    fn add(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() + value)
    }
}

impl Sub<f32> for VerticalAlign {
    type Output = VerticalAlign;

    fn sub(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() - value)
    }
}

impl Mul<f32> for VerticalAlign {
    type Output = VerticalAlign;

    fn mul(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() * value)
    }
}

impl Div<f32> for VerticalAlign {
    type Output = VerticalAlign;

    fn div(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() / value)
    }
}

#[derive(PartialEq, Debug, is_enum_variant, Copy, Clone)]
pub enum HorizontalAlign {
    Left(f32),
    Middle(f32),
    Right(f32)
}

impl HorizontalAlign {
    pub fn same_with_value(self, value: f32) -> Self {
        match self {
            HorizontalAlign::Left(_) => HorizontalAlign::Left(value),
            HorizontalAlign::Middle(_) => HorizontalAlign::Middle(value),
            HorizontalAlign::Right(_) => HorizontalAlign::Right(value)
        }
    }

    pub fn inner(self) -> f32 {
        match self {
            HorizontalAlign::Left(left) => left,
            HorizontalAlign::Middle(middle) => middle,
            HorizontalAlign::Right(right) => right
        }
    }

    pub fn absolute(self, width: f32) -> f32 {
        match self {
            HorizontalAlign::Left(left) => left,
            HorizontalAlign::Middle(middle) => width / 2.0 + middle,
            HorizontalAlign::Right(right) => width - right
        }
    }
}

impl From<f32> for HorizontalAlign {
    fn from(left: f32) -> Self {
        HorizontalAlign::Left(left)
    }
}

impl Add<f32> for HorizontalAlign {
    type Output = HorizontalAlign;

    fn add(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() + value)
    }
}

impl Sub<f32> for HorizontalAlign {
    type Output = HorizontalAlign;

    fn sub(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() - value)
    }
}

impl Mul<f32> for HorizontalAlign {
    type Output = HorizontalAlign;

    fn mul(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() * value)
    }
}

impl Div<f32> for HorizontalAlign {
    type Output = HorizontalAlign;

    fn div(self, value: f32) -> Self::Output {
        self.same_with_value(self.inner() / value)
    }
}
