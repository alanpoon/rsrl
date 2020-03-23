use hardback_server::codec_lib::codec::*;
use hardback_server::codec_lib::cards;
use hardback_server::codec_lib::cards::*;
use hardback_server::game_logic::game_engine::GameCon;
use std::{self, collections::HashMap, fmt};
use websocket::message::OwnedMessage;
use std::sync::mpsc;
pub enum GameRxType {
    Sender(String, mpsc::Sender<OwnedMessage>),
    Message(String, OwnedMessage),
    Close(String),
}
#[derive(Clone)]
pub struct Connection {
    pub sender: mpsc::Sender<OwnedMessage>,
}
impl GameCon for Connection {
    fn tx_send(&self, msg: ClientReceivedMsg, log: &mut Vec<ClientReceivedMsg>) {
        self.sender
            .clone()
            .send(OwnedMessage::Text(
                ClientReceivedMsg::serialize_send(msg).unwrap(),
            ))
            .unwrap();
    }
}
impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "") }
}
impl Connection {
    pub fn new(sender: mpsc::Sender<OwnedMessage>) -> Connection { Connection { sender: sender } }
}
