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
use hardback_server::codec_lib::cards::{self,WaitForInputType};
use hardback_server::draft::TheStartingDraftStruct;
use hardback_server::drafttest::TheStartingSeedDraftStruct;
use std::sync::mpsc;
use websocket::message::OwnedMessage;
use hardback_server::game_logic::game_engine::*;
use hardback_server::game_logic::purchase;
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
            let boardstruct = TheStartingSeedDraftStruct{};
            hardback_server::game_logic::GameEngine::new(vec![p],connections).run(rx, boardstruct,&mut log);
        });
        
        MountainCar::new(-0.5, 0.0, false,tx, con_rx)
    }
}
fn p_game(game_state:GameState,player:Player,offer_row:Vec<usize>,unknown:&mut Vec<usize>,buy_card: &mut Option<usize>,tx:mpsc::Sender<(usize,GameCommand)>,cardmeta:&[cards::ListCard<BoardStruct>; 180],action:usize)->([Option<(String,i8,i8,Vec<(usize, bool, std::option::Option<std::string::String>, bool)>,[WaitForInputType;4])>;9],usize){
    //compute each card in offer_row with the covered cards and rank them
    /*buy [d_coin.0, d_vp.1,d_ink.2,d_remover.3,d_literacy_award.4,d_lockup.5,d_discard.6,minus_ink.7,minus_remover.8])*/
    /*(d_coin ,d_vp.9,d_ink.10,d_remover.11,d_literacy_award.12,d_lockup.13,d_discard.14,minus_ink.15,minus_remover.16])*/
    let existing_plays: Vec<scrabble::ScrabblePlay> = vec![];
    let board = scrabble::board_from_plays(&existing_plays);
    let player_c = player.clone();
    let mut draft = vec![];
    
    if let GameState::Shuffle= game_state{
        for u in unknown.iter(){
            if let None = player.hand.iter().position(|&r| r == *u){
                draft.push(*u);
            }
        }
    }else if draft.len()>0{
        draft = player.draft.clone();
    }else{
        draft = unknown.clone();
    }
    let buy_play = scrabble::best_card_to_buy(draft.clone(),offer_row.clone(),&board);
    let plays = scrabble::generate_plays(player.hand.clone(), &board);
    let mut new_arranged=None;
    let mut new_word=None;
    let mut buy_lockup = None;
    let mut action =2;
    let mut wait_for_input_index=0;    
    if action <9{
        println!("gamestate {:?}",game_state);
        if let None = &buy_play[action]{
            action = 0;
        }
        if let Some((word,cost,value,arranged,wait_,buy_card_index))= &buy_play[action]{
            let card_price = cardmeta[*buy_card_index].cost;
            if let Some(p) = &plays[0]{//p[d_coin,d_vp,d_ink,d_remover,d_literacy_award,d_lockup,d_discard,minus_ink,minus_remover]
                println!("----------card_price {:?}  p.1 {:?}",card_price,p.1);
                if card_price<=p.1 as usize{  //string,cost,value
                    *buy_card = offer_row.iter().position(|&r| r == *buy_card_index);
                    println!("buy_card_index mm{:?}",buy_card_index);
                }else{
                    let mut most_exp_affordable:Option<usize> = None;
                    for o in offer_row.iter(){
                        let card_price = cardmeta[*o].cost;
                        if let Some(a) = most_exp_affordable{
                            if card_price > cardmeta[a].cost && card_price<=p.1 as usize{
                                most_exp_affordable = Some(*o);
                            }
                        }else{
                            if card_price<=p.1 as usize{
                                most_exp_affordable = Some(*o);
                            }
                        }
                    }
                    if let Some(o) = most_exp_affordable{
                        *buy_card = offer_row.iter().position(|&r| r == o);
                        println!("buy_card_index mm{:?}",o);
                    }
                }
                new_word = Some((p.0.clone(),p.1));
                new_arranged=Some(p.3.clone());
                wait_for_input_index = 0;
            }
            if let (&None,Some(p)) = (&new_arranged,&plays[0]){
                if card_price>p.1 as usize{
                    if let Some(lockup)= &plays[5]{
                        buy_lockup = offer_row.iter().position(|&r| r == *buy_card_index);
                    }
                }else{
                    *buy_card = offer_row.iter().position(|&r| r == *buy_card_index);
                }
                new_word = Some((p.0.clone(),p.1.clone()));
                new_arranged = Some(p.3.clone());
                wait_for_input_index = 0;
            }
            
        }
    }else{//9->1
        if let Some(z) = &plays[action-8]{
            new_arranged=Some(z.3.clone());
            new_word=Some((z.0.clone(),z.1.clone()));
            wait_for_input_index = action-8;
        }else{
            if let Some(z) = &plays[1]{
                new_arranged = Some(z.3.clone());
                wait_for_input_index = 1;
                new_word=Some((z.0.clone(),z.1.clone()));
            }else{
                if let Some(z) = &plays[0]{
                    new_arranged = Some(z.3.clone());
                    wait_for_input_index = 0;
                    new_word=Some((z.0.clone(),z.1.clone()));
                }
            }
        }
    }
    let mut k1 = GameCommand::new();
    k1.arranged = new_arranged.clone();
    k1.submit_word = Some(true);
    tx.send((0,k1)).unwrap();
    println!("submit_word {:?}",new_arranged.clone());
    
    if let Some(z) = buy_lockup{
        let mut k3 = GameCommand::new();
        k3.buy_lockup = Some((true,z));
        tx.send((0,k3)).unwrap();
    }
    if let  Some(p) = &plays[wait_for_input_index]{
        println!("----wait_for_input_index {:?} len: {:?}",wait_for_input_index,p.4[0].len());

    }
    (plays,wait_for_input_index)
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
        let mut unknown:Vec<usize>= vec![];
        let mut buy_card = None;
        let mut wait_for_index = 0;
        let mut gen_play:[Option<(String,i8,i8,Vec<(usize, bool, std::option::Option<std::string::String>, bool)>,[WaitForInputType;4])>;9] = [None,None,None,None,None,None,None,None,None];
        while k{
            if let Ok(msg) = self.con_rx.recv() {
                if let OwnedMessage::Text(z) = msg {
                    if let Ok(ClientReceivedMsg { boardstate,notification, .. }) =
                        ClientReceivedMsg::deserialize_receive(&z)
                    {
                        
                        if let Some(Some(Ok(boardcodec))) = boardstate{
                            let player = boardcodec.players.get(0).unwrap();
                            let gamestate = boardcodec.gamestates.get(0).unwrap();
                            println!("jj gamestate {:?} player {:?}",gamestate,player.clone());
                            match gamestate {
                                GameState::ShowDraft=>{
                                    let mut g = GameCommand::new();
                                    g.go_to_shuffle = Some(true);
                                    self.tx.send((0,g)).unwrap();
                                    unknown = player.draft.clone();
                                },
                                GameState::ShowResult(winner_id) => {
                                    self.end =true;
                                    let mut k1 = GameCommand::new();
                                    k1.exit_game =Some(true);
                                    k=false;
                                    self.tx.send((0,k1)).unwrap();
                                },
                                GameState::Shuffle | GameState::Spell | GameState::TurnToSubmit=> {
                                    let (gen_playz,wait_for_indexz)= p_game(gamestate.clone(),player.clone(),boardcodec.offer_row,&mut unknown,&mut buy_card,self.tx.clone(),&cardmeta,action);
                                    gen_play = gen_playz;
                                    wait_for_index = wait_for_indexz;
                                },
                                GameState::Buy=>{
                                    let mut k3 = GameCommand::new();
                                    if let Some(z) = buy_card{
                                        println!("gamestate is buy,player {:?} offer {:?} buy_card{:?}" ,player,boardcodec.offer_row,buy_card);
                                        k3.buy_offer = Some((true,z));
                                        let mut offer_row = boardcodec.offer_row.clone();
                                        let mut _board = BoardStruct::new(vec![player.clone()],&offer_row);
                                        if let  Some(p) = &mut gen_play[wait_for_index]{
                                            purchase::buy_card_from(z,
                                                &mut offer_row,
                                                &cardmeta,
                                                &mut _board,
                                                0,
                                                &mut p.4);
                                        }
                                    }else{
                                        k3.buy_offer = Some((false,0));
                                    }
                                    self.tx.send((0,k3)).unwrap();
                                    buy_card = None;
                                },
                                GameState::TrashOther(z) =>{
                                    let mut k3 = GameCommand::new();
                                    k3.reply = Some(0);
                                    self.tx.send((0,k3)).unwrap();
                                },
                                GameState::WaitForReply=>{
                                    println!("waitfor!!");
                                    println!("wait_for_index {:?}",wait_for_index);
                                    if let  Some(p) = &mut gen_play[wait_for_index]{
                                        println!("!!! len: {:?}",p.4[0].len());
                                        if p.4[0].len()>0{
                                            p.4[0].remove(0);
                                            let mut k3 = GameCommand::new();
                                            k3.reply = Some(0);
                                            self.tx.send((0,k3)).unwrap();
                                        }
                                    }
                                },
                                _=>{
                                    println!("gamestate is others,game_state {:?} player {:?}",gamestate,player);
                                }
                            }
                        }
                        if let Some(notify) = notification{
                            println!("notify {:?}",notify);
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
