use p3_field::Algebra;

pub fn mul5<Expr: Algebra<Expr>>(x: Expr) -> Expr {
    x.double().double() + x
}


// TODO: delete it
use std::time::{Instant, Duration};

pub struct Timer {
    start: Instant,
    last: Duration,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            start: Instant::now(),
            last: Duration::new(0, 0),
        }
    }

    pub fn count(&mut self) -> Duration {
        let e = self.start.elapsed();
        let res = e - self.last;
        self.last = e;
        res
    }
}