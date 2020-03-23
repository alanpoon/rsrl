extern crate futures;
extern crate rsrl;
extern crate hardback_server;
extern crate scrabble;

#[macro_use]
extern crate slog;
use::std::collections::HashMap;
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
// use futures::channel::mpsc;
use hardback_server::codec_lib::codec::*;
use hardback_server::codec_lib::cards;
use hardback_server::draft::TheStartingDraftStruct;
use std::sync::mpsc;
use websocket::message::OwnedMessage;
use hardback_server::game_logic::game_engine::*;
use hardback_server::game_logic::board::BoardStruct;
const CONNECTION: &'static str = "127.0.0.1:8080";
#[derive(Clone)]
pub struct Connection {
    pub name: String,
    pub player_num: Option<usize>,
    pub sender: mpsc::Sender<OwnedMessage>,
}

impl GameCon for Connection {
    fn tx_send(&self, msg: ClientReceivedMsg, log: &mut Vec<ClientReceivedMsg>) {
        let ClientReceivedMsg { boardstate, request, .. } = msg.clone();
        if let Some(Some(_)) = boardstate.clone() {
            if let Some(0) = self.player_num {
                log.push(msg.clone());
            }
        } else if let Some(Some(_)) = request.clone() {
            log.push(msg.clone());
        }

        self.sender
            .clone()
            .send(OwnedMessage::Text(ClientReceivedMsg::serialize_send(msg).unwrap()))
            .unwrap();
    }
}
pub struct MountainCar {
    x: f64,
    v: f64,
    end:bool,
    tx: mpsc::Sender<(usize,GameCommand)>,
    con_rx: mpsc::Receiver<OwnedMessage>,
    from: Observation<Vec<f64>>,
}

impl MountainCar {
    pub fn new(
        x: f64,
        v: f64,
        end: bool,
        tx: mpsc::Sender<(usize,GameCommand)>,
        con_rx: mpsc::Receiver<OwnedMessage>,
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

    fn update_state(&mut self, _a: usize) {
        let k1 = GameCommand::new();
        self.tx.send((0,k1)).unwrap();
    }
}
impl Default for MountainCar {
    fn default() -> MountainCar {
        let (tx, rx) = mpsc::channel();
        let (con_tx, con_rx) = mpsc::channel();
        let p = Player::new("DefaultPlayer".to_owned());
        let connections: HashMap<usize, Connection> = [(0,
                                                    Connection {
                                                        name: "DefaultPlayer".to_owned(),
                                                        player_num: Some(0),
                                                        sender: con_tx,
                                                    })]
            .iter()
            .cloned()
            .collect();
        
        std::thread::spawn(|| {
            let mut log: Vec<ClientReceivedMsg> = vec![];
            let boardstruct = TheStartingDraftStruct{};
            hardback_server::game_logic::GameEngine::new(vec![p],connections).run(rx, boardstruct,&mut log);
        });
        
        MountainCar::new(-0.5, 0.0, false,tx, con_rx)
    }
}
fn p_game(game_state:GameState,player:Player,offer_row:Vec<usize>,tx:mpsc::Sender<(usize,GameCommand)>,cardmeta:&[cards::ListCard<BoardStruct>; 180],action:usize){
    //compute each card in offer_row with the covered cards and rank them
    /*buy [d_coin.0, d_vp.1,d_ink.2,d_remover.3,d_literacy_award.4,d_lockup.5,d_discard.6,minus_ink.7,minus_remover.8])*/
    /*(d_coin ,d_vp.9,d_ink.10,d_remover.11,d_literacy_award.12,d_lockup.13,d_discard.14,minus_ink.15,minus_remover.16])*/
    let existing_plays: Vec<scrabble::ScrabblePlay> = vec![];
    let board = scrabble::board_from_plays(&existing_plays);
    let buy_play = scrabble::best_card_to_buy(player.draft,offer_row.clone(),&board);
    let plays = scrabble::generate_plays(player.hand, &board);
    let mut new_arranged=None;
    let mut buy_card =None;
    let mut buy_lockup = None;
    if action <9{
        if let Some((word,value,arranged,buy_card_index))= &buy_play[action]{
            let card_price = cardmeta[*buy_card_index].cost;
            if let Some(p) = &plays[0]{
                if card_price>player.coin +p.1 as usize{
                    if let Some(lockup)= &plays[5]{
                        buy_lockup = offer_row.iter().position(|&r| r == *buy_card_index);
                    }
                }else{
                    buy_card = offer_row.iter().position(|&r| r == *buy_card_index);
                }
            }
        }
    }else{//9->1
        if let Some(z) = plays[action-8].clone(){
            new_arranged=Some(z.2);
        }else{
            if let Some(z) = plays[1].clone(){
                new_arranged = Some(z.2);
            }else{
                new_arranged = Some(plays[0].clone().unwrap().2);
            }
        }
    }
    let mut k1 = GameCommand::new();
    k1.arranged = new_arranged;
    tx.send((0,k1)).unwrap();
    let mut k2 = GameCommand::new();
    k2.submit_word = Some(true);
    tx.send((0, k2)).unwrap();
    if let Some(z) = buy_card{
        let mut k3 = GameCommand::new();
        k3.buy_offer = Some((true,z));
        tx.send((0,k3)).unwrap();
    }
    if let Some(z) = buy_lockup{
        let mut k3 = GameCommand::new();
        k3.buy_lockup = Some((true,z));
        tx.send((0,k3)).unwrap();
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
        let cardmeta = cards::populate::<BoardStruct>();
        let mut c=0;
        while k{
            if let Ok(msg) = self.con_rx.recv() {
                println!("c {:?}",c);
                if let OwnedMessage::Text(z) = msg {
                    if let Ok(ClientReceivedMsg { boardstate, .. }) =
                        ClientReceivedMsg::deserialize_receive(&z)
                    {
                        if let Some(Some(Ok(boardcodec))) = boardstate{
                            k=false;
                            let player = boardcodec.players.get(0).unwrap();
                            let gamestate = boardcodec.gamestates.get(0).unwrap();
                            println!("gamestate{:?} {:?}",gamestate,std::time::Instant::now());
                            match gamestate {
                                GameState::ShowDraft=>{
                                    let mut g = GameCommand::new();
                                    g.go_to_shuffle = Some(true);
                                    self.tx.send((0,g)).unwrap();
                                    println!("showdraft {:?}",std::time::Instant::now());
                                },
                                GameState::ShowResult(winner_id) => {
                                    self.end =true;
                                    let mut k1 = GameCommand::new();
                                    k1.exit_game =Some(true);
                                    self.tx.send((0,k1)).unwrap();
                                },
                                GameState::Shuffle | GameState::Spell=> {
                                    p_game(gamestate.clone(),player.clone(),boardcodec.offer_row,self.tx.clone(),&cardmeta,action);
                                },
                                _=>{

                                }
                            }
                        }
                    }
                }
            c= c+1;
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
    scrabble::load_dawg();
    //let domain = MountainCar::default();
    let n_actions =game_logic::action_space().card().into();
    let bases = Fourier::from_space(17, game_logic::state_space()).with_constant();
    
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
