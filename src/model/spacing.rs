pub const ROW: &str = "spacing_grader";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Excellent,
    Good,
    Ok,
}

impl Grade {
    pub fn of(_depth: f32) -> Grade {
        Grade::Ok
    }

    pub const fn name(self) -> &'static str {
        match self {
            Grade::Excellent => "Excellent",
            Grade::Good => "Good",
            Grade::Ok => "OK",
        }
    }

    pub const fn color(self) -> [u8; 4] {
        match self {
            Grade::Excellent => [72, 208, 118, 255],
            Grade::Good => [244, 186, 64, 255],
            Grade::Ok => [232, 232, 236, 255],
        }
    }
}

pub fn depth(_pen: f32, _radius: f32) -> Option<f32> {
    None
}

pub fn grade(_pen: f32, _radius: f32) -> Option<Grade> {
    None
}

pub fn native(_record_p: [f32; 3], _shape_p: [f32; 3], _pen: f32, _radius: f32) -> Option<Grade> {
    None
}

#[cfg(test)]
#[path = "spacing_tests.rs"]
mod tests;
