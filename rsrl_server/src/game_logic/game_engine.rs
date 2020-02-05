use crate::game_logic;
use rsrl_codec::*;
use std::sync::mpsc;
pub trait GameCon {
    fn tx_send(&self, msg: ClientReceivedMsg, log: &mut Vec<ClientReceivedMsg>);
}
pub struct GameEngine<T: GameCon> {
    connection: T,
    gamestate: GameState,
}
impl<T> GameEngine<T>
where T: GameCon
{
    pub fn new(connection: T) -> Self {
        GameEngine {
            connection,
            gamestate: GameState::Game(0.0, 0.0, 0),
        }
    }
    pub fn run(&mut self, rx: mpsc::Receiver<GameCommand>, log: &mut Vec<ClientReceivedMsg>) {
        let mut last_update = std::time::Instant::now();
        'game: loop {
            let sixteen_ms = std::time::Duration::new(1, 0);
            let now = std::time::Instant::now();
            let duration_since_last_update = now.duration_since(last_update);

            if duration_since_last_update < sixteen_ms {
                std::thread::sleep(sixteen_ms - duration_since_last_update);
            }

            while let Ok(game_command) = rx.try_recv() {
                match (&game_command, &self.connection, &mut self.gamestate) {
                    (&GameCommand { a: Some(ref a), .. }, ref con, ref mut _gamestate) => {
                        match _gamestate {
                            GameState::Game(_, _, _) => {
                                game_logic::update_gamestate::<T>(
                                    *a,
                                    _gamestate,
                                    con,
                                    log,
                                );
                            },
                            _ => {},
                        }
                    },
                    (
                        &GameCommand {
                            exit_game: Some(true),
                            ..
                        },
                        con,
                        _gamestate,
                    ) => {
                        println!("break");
                        break 'game;
                    },
                    _ =>{}
                }
            }
        }
    }
}
