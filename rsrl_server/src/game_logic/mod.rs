pub mod game;
pub mod game_engine;
use self::game_engine::GameCon;
pub use self::game_engine::GameEngine;
use rsrl::spaces::{discrete::Ordinal, real::Interval, ProductSpace};
use rsrl_codec::*;
const X_MIN: f64 = -1.2;
const X_MAX: f64 = 0.6;

const V_MIN: f64 = -0.07;
const V_MAX: f64 = 0.07;

const FORCE_G: f64 = -0.0025;
const FORCE_CAR: f64 = 0.001;

const HILL_FREQ: f64 = 3.0;

pub const REWARD_STEP: f64 = -1.0;
pub const REWARD_GOAL: f64 = 0.0;

pub const ALL_ACTIONS: [f64; 3] = [-1.0, 0.0, 1.0];

macro_rules! clip {
    ($lb:expr, $x:expr, $ub:expr) => {{
        $lb.max($ub.min($x))
    }};
}

fn dv(x: f64, a: f64) -> f64 { FORCE_CAR * a + FORCE_G * (HILL_FREQ * x).cos() }

pub fn update_gamestate<T: GameCon>(
    a: usize,
    gamestate: &mut GameState,
    con: &T,
    log: &mut Vec<ClientReceivedMsg>,
)
{
    if let GameState::Game(x, v, step) = gamestate {
        if *x >= X_MAX {
            *gamestate = GameState::ShowResult(*step);
        } else {
            let a = ALL_ACTIONS[a];
            *v = clip!(V_MIN, *v + dv(*x, a), V_MAX);
            *x = clip!(X_MIN, *x + *v, X_MAX);
            *step = *step + 1;
        }
    }
    let mut h = ClientReceivedMsg::deserialize_receive("{}").unwrap();
    h.set_gamestate(gamestate.clone());
    con.tx_send(h,log);
    //println!("gamestate {:?}",gamestate);
}
pub fn state_space() -> ProductSpace<Interval> {
    ProductSpace::empty() + Interval::bounded(X_MIN, X_MAX) + Interval::bounded(V_MIN, V_MAX)
}
pub fn action_space() -> Ordinal { Ordinal::new(3) }
