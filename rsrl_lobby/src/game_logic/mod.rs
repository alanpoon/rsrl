pub mod game;
use rsrl::spaces::{discrete::Ordinal, real::Interval, ProductSpace};
use hardback_server::codec_lib::*;
use hardback_server::codec_lib::codec::*;
const X_MIN: f64 = -1.2;
const X_MAX: f64 = 0.6;

const V_MIN: f64 = -0.07;
const V_MAX: f64 = 0.07;

const FORCE_G: f64 = -0.0025;
const FORCE_CAR: f64 = 0.001;

const HILL_FREQ: f64 = 3.0;

pub const REWARD_STEP: f64 = -1.0;
pub const REWARD_GOAL: f64 = 0.0;

macro_rules! clip {
    ($lb:expr, $x:expr, $ub:expr) => {{
        $lb.max($ub.min($x))
    }};
}

fn dv(x: f64, a: f64) -> f64 { FORCE_CAR * a + FORCE_G * (HILL_FREQ * x).cos() }

pub fn state_space() -> ProductSpace<Interval> {
    ProductSpace::empty() + Interval::bounded(X_MIN, X_MAX) + Interval::bounded(V_MIN, V_MAX)
}
pub fn action_space() -> Ordinal { Ordinal::new(188) }
