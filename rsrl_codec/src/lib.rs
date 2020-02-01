extern crate serde;
extern crate serde_json;
use serde::{Deserialize, Deserializer};
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate cardgame_macros;
fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableInfo {
    pub player: String,
}
impl TableInfo {
    pub fn new(player: String) -> TableInfo { TableInfo { player } }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Player {
    pub name: String,
    pub x: f64,
    pub v: f64,
}
impl Player {
    pub fn new(name: String, x: f64, v: f64) -> Player { Player { name, x, v } }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameCommand {
    pub a: Option<usize>,
    pub exit_game: Option<bool>,
}
impl GameCommand {
    pub fn new(a: usize) -> Self {
        GameCommand {
            a: Some(a),
            exit_game: None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameState {
    Game(f64, f64, u64),
    ShowResult(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConnectionError {
    NotConnectedToInternet,
    CannotFindServer,
    InvalidDestination,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "connection_status", content = "c")]
pub enum ConnectionStatus {
    None,
    Try,
    Error(ConnectionError),
    Ok,
}

CGM_codec! {
    structname:ServerReceivedMsg,
    rename:{
    },optional:{
    (gamecommand,set_gamecommand,GameCommand),
    (message,set_message,String),
    },rename_optional:{},else:{}
}
CGM_codec! {
    structname:ClientReceivedMsg,
   rename:{
    },optional:{
    (gamestate,set_gamestate,GameState),
    },rename_optional:{ (type_name,set_type_name,String,"type"),},else:{}
}
