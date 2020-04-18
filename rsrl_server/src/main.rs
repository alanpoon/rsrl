extern crate futures;
extern crate rsrl;
extern crate rsrl_codec;
#[macro_use]
extern crate slog;

use rsrl::{
    control::{ac::A2C, td::SARSA},
    fa::linear::{
        basis::{Fourier, Projector},
        optim::SGD,
        LFA,
    },
    logging,
    make_shared,
    policies::Gibbs,
    run,
    spaces::Space,
    Evaluation,
    SerialExperiment,
};
pub mod game_logic;

use rsrl::{
    domains::{Domain, Observation, Transition},
    spaces::{discrete::Ordinal, real::Interval, ProductSpace},
};
use rsrl_codec::*;
// use futures::channel::mpsc;
use crate::game_logic::game::Connection;
use std::sync::mpsc;
use websocket::message::OwnedMessage;
const CONNECTION: &'static str = "127.0.0.1:8080";
pub struct MountainCar {
    x: f64,
    v: f64,
    end:bool,
    tx: mpsc::Sender<GameCommand>,
    con_rx: futures::channel::mpsc::Receiver<OwnedMessage>,
    from: Observation<Vec<f64>>,
}

impl MountainCar {
    pub fn new(
        x: f64,
        v: f64,
        end: bool,
        tx: mpsc::Sender<GameCommand>,
        con_rx: futures::channel::mpsc::Receiver<OwnedMessage>,
    ) -> MountainCar
    {
        MountainCar {
            x,
            v,
            end,
            tx,
            con_rx,
            from: Observation::Full(vec![x, v]),
        }
    }

    fn update_state(&mut self, a: usize) {
        let k1 = GameCommand::new(a);
        self.tx.send(k1).unwrap();
    }
}
impl Default for MountainCar {
    fn default() -> MountainCar {
        let (tx, rx) = mpsc::channel();
        let (con_tx, con_rx) = futures::channel::mpsc::channel(1);
        let player_con = Connection { sender: con_tx };
        std::thread::spawn(|| {
            let mut log: Vec<ClientReceivedMsg> = vec![];
            game_logic::GameEngine::new(player_con).run(rx, &mut log);
        });
        MountainCar::new(-0.5, 0.0, false,tx, con_rx)
    }
}

impl Domain for MountainCar {
    type StateSpace = ProductSpace<Interval>;
    type ActionSpace = Ordinal;

    fn emit(&self) -> Observation<Vec<f64>> {
        if self.end{
            Observation::Terminal(vec![self.x, self.v])
        }else{
            Observation::Full(vec![self.x, self.v])
        }
    }

    fn step(&mut self, action: usize) -> Transition<Vec<f64>, usize> {
        self.update_state(action);
        let mut k =true;
        while k{
            if let Ok(msg) = self.con_rx.try_next() {
                if let Some(OwnedMessage::Text(z)) = msg {
                    if let Ok(ClientReceivedMsg { gamestate, .. }) =
                        ClientReceivedMsg::deserialize_receive(&z)
                    {
                        k=false;
                        match gamestate.unwrap().unwrap() {
                            GameState::ShowResult(_) => {
                                self.end =true;
                                let mut k1 = GameCommand::new(0);
                                k1.a=None;
                                k1.exit_game =Some(true);
                                self.tx.send(k1).unwrap();
                            },
                            GameState::Game(x, v, step) => {
                                self.x = x;
                                self.v = v;
                                if step==1000{
                                    let mut k1 = GameCommand::new(0);
                                    k1.a=None;
                                    k1.exit_game =Some(true);
                                    self.tx.send(k1).unwrap();
                                }
                            },
                        }
                    }
                }
            }
        }
        
        let from = self.from.clone();
        let to = self.emit();
        self.from = to.clone();
        Transition {
            from,
            action,
            reward: if to.is_terminal() {
                game_logic::REWARD_GOAL
            } else {
                game_logic::REWARD_STEP
            },
            to,
        }
    }

    fn state_space(&self) -> Self::StateSpace { game_logic::state_space() }

    fn action_space(&self) -> Ordinal { game_logic::action_space() }
}
fn main() {
    let domain = MountainCar::default();
    let n_actions = domain.action_space().card().into();
    let bases = Fourier::from_space(3, domain.state_space()).with_constant();

    let policy = make_shared({
        let fa = LFA::vector(bases.clone(), SGD(1.0), n_actions);

        Gibbs::standard(fa)
    });
    let critic = {
        let q_func = LFA::vector(bases, SGD(1.0), n_actions);

        SARSA::new(q_func, policy.clone(), 0.001, 1.0)
    };

    let mut agent = A2C::new(critic, policy, 0.001);

    let logger = logging::root(logging::stdout());
    let domain_builder = Box::new(MountainCar::default);

    // Training phase:
    let _training_result = {
        // Start a serial learning experiment up to 1000 steps per episode.
        let e = SerialExperiment::new(&mut agent, domain_builder.clone(), 1000);

        // Realise 1000 episodes of the experiment generator.
        run(e, 1000, Some(logger.clone()))
    };

    // Testing phase:
    let testing_result = Evaluation::new(&mut agent, domain_builder).next().unwrap();

    info!(logger, "solution"; testing_result);
}
