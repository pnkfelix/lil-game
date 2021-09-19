use lambda_runtime::{handler_fn, Context};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::Error as _;

use std::borrow::Cow;
use std::default::Default;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Deserialize)]
struct Request {
    path: String,
}

// Note: These field names are significant.
//
// For more information, see:
// https://docs.aws.amazon.com/
//     apigateway/latest/developerguide/set-up-lambda-proxy-integrations.html
//     #api-gateway-simple-proxy-for-lambda-output-format
//
// 
// #[allow(non_snake_case)]
#[derive(Serialize)]
struct Response {
    body: DoublyEncode<ResponseBody>,
    // This should be a u32, but API Gateway actually expects a String that looking like an int for some reason.
    #[serde(rename="statusCode")]
    status_code: String,
}

struct DoublyEncode<T>(pub T);

impl<T:Serialize> Serialize for DoublyEncode<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let first_encoded = serde_json::to_string(&self.0).map_err(|err| S::Error::custom(err))?;
        serializer.serialize_str(&first_encoded)
    }
}

#[derive(Serialize)]
struct ResponseBody {
    // request: String,
    // ctx: String,
    command: String,
    parsed_game_state: String,
    player: String,
    next_game_states: Option<Vec<MoveDescription>>,
    selected_move: Option<(String, String)>,
    text: Option<String>,
    victory: Option<String>,
}

#[derive(Serialize)]
struct MoveDescription {
    move_id: String,
    next_board: String,
    next_player: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

pub(crate) async fn my_handler(event: Request, _ctx: Context) -> Result<Response, Error> {
    // a correct input path will tend to be of form `/C/GAME` where C is a
    // single character command code and GAME is a multiple-character string
    // describing the game state.
    //
    // The main exception is creating a fresh game, which just takes the form `/n/`, with no need
    // for a further string.

    // drop the leading `/`
    let (slash, input) = event.path.split_at(1);
    assert_eq!(slash, "/");
    let (cmd, slash_state) = input.split_at(1);
    let (slash, state) = slash_state.split_at(1);
    assert_eq!(slash, "/");

    #[derive(Debug)]
    struct UnknownCommand;
    impl std::fmt::Display for UnknownCommand {
        fn fmt(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(w, "unknown command")
        }
    }
    impl std::error::Error for UnknownCommand {
        
    }

    #[derive(PartialEq, Eq, Debug)]
    enum Command {
        NewGame,
        List,
        RenderToText,
        Select,
    }

    let c = match cmd {
        "n" => Command::NewGame,
        "l" => Command::List,
        "r" => Command::RenderToText,
        "s" => Command::Select,
        _ => return Err(Box::new(UnknownCommand)),
    };

    let game = if c == Command::NewGame { Default::default() } else { TicTacToeGame::parse(state)? };
    let player = game.player.to_string();
    let command;
    let parsed_game_state = game.unparse();
    let next_game_states;
    let selected_move;
    let text;
    let victory;
    
    match c {
        Command::NewGame => {
            command = "new-game".to_string();
            next_game_states = None;
            selected_move = None;
            text = None;
            victory = None;
        }
        Command::List => {
            command = "list".to_string();
            next_game_states = Some(game.moves()
                .into_iter()
                .map(|m| MoveDescription {
                    move_id: m.id.to_string(),
                    next_board: m.next_state.unparse(),
                    next_player: m.next_state.player.to_string(),
                })
                .collect());
            selected_move = None;
            victory = None;
            text = None;
        }
        Command::RenderToText => {
            command = "render-to-text".to_string();
            next_game_states = None;
            selected_move = None;
            victory = None;
            text = Some(game.render_to_text());
        }
        Command::Select => {
            command = "select".to_string();
            next_game_states = None;
            let moves = game.moves();
            let chosen = search(&moves[..]);
            let choice = &moves[chosen];
            selected_move = Some((choice.id.to_string(), choice.next_state.board.iter().collect()));
            victory = choice.victor.map(|c|c.to_string());
            text = None;
        }
    }
        
    let resp = Response {
        body: DoublyEncode(ResponseBody {
            // request: format!("{:?}", event),
            // ctx: format!("{:?}", _ctx),
            command,
            parsed_game_state,
            player,
            next_game_states,
            selected_move,
            text,
            victory,
        }),
        status_code: String::from("200")
    };

    Ok(resp)
}

// To simplify things, we will identify players by single characters, and
// moves for a given board by a positive number (we will assume 32-bits
// suffice to encode any one discrete move for a board).

type MoveId = u32;
type Player = char;

struct Move<B: Game> {
    id: MoveId,
    victor: Option<Player>,
    next_state: B,
}

trait Game: Sized {
    fn parse(input: &str) -> Result<Self, Cow<str>>;
    fn unparse(&self) -> String;
    fn moves(&self) -> Vec<Move<Self>>;
    fn render_to_text(&self) -> String;
}

type TicTacToeBoard = [char; 9];

#[derive(Clone, Debug)]
struct TicTacToeGame {
    board: TicTacToeBoard,
    player: Player,
}

impl Default for TicTacToeGame {
    fn default() -> Self {
        Self { board: ['_'; 9], player: 'X' }
    }
}


impl Game for TicTacToeGame {
    fn unparse(&self) -> String {
        self.board.iter().collect()
    }

    fn parse(input: &str) -> Result<Self, Cow<str>> {
        let mut g = TicTacToeGame { board: ['_'; 9], player: 'X' };
        if input.chars().count() != 9 { return Err("input must be length 9".into());}
        let mut num_x = 0;
        let mut num_o = 0;
        for (i, c) in input.chars().enumerate() {
            match c {
                '_' | 'X' | 'O' => g.board[i] = c,
                'x' | 'o' => return Err("only upper-case moves allowed".into()),
                _ => return Err("unexpected chacter found in board".into()),
            }
            if c == 'X' { num_x += 1; }
            if c == 'O' { num_o += 1; }
        }
        if num_o > num_x { return Err("too many O moves".into()); }
        match num_x - num_o {
            0 => g.player = 'X',
            1 => g.player = 'O',
            _ => return Err("too many X moves".into()),
        }
        return Ok(g);
    }
    
    fn moves(&self) -> Vec<Move<Self>> {
        let mut v = Vec::new();
        let next_player = if self.player == 'X' { 'O' } else { 'X' };
        for i in 1..=9 {
            if self.board[i-1] == '_' {
                let mut next_board = self.board;
                next_board[i-1] = self.player;
                let victor = victory(&next_board, self.player);
                v.push(Move {
                    id: i as u32,
                    victor,
                    next_state: TicTacToeGame { 
                        board: next_board,
                        player: next_player
                    },
                });
            }
        }
        return v;
    }
    
    fn render_to_text(&self) -> String {
        match self.board {
            [a, b, c,
             m, n, o,
             x, y, z] => {
                // converts the char for a cell state into a three character string.
                fn pad(c: char) -> String {
                    format!(" {} ", if c == '_' { ' ' } else { c })
                }
                format!("{a} | {b} | {c} \n\
                         ----|-----|-----\n\
                         {m} | {n} | {o} \n\
                         ----|-----|-----\n\
                         {x} | {y} | {z} \n",
                         a=pad(a), b=pad(b), c=pad(c),
                         m=pad(m), n=pad(n), o=pad(o), 
                         x=pad(x), y=pad(y), z=pad(z))
            }
        }
    }
}

fn victory(board: &TicTacToeBoard, player: Player) -> Option<Player> {
    match board {
        [x,y, z,
        _, _, _,
        _, _, _] |

        [_, _, _,
        x, y, z,
        _, _, _] |

        [_, _, _,
        _, _, _,
        x, y, z] |

        [x, _, _,
         y, _, _,
         z, _, _] |

        [_, x, _,
         _, y, _,
         _, z, _] |

        [_, _, x,
         _, _, y, 
         _, _, z] |

        [x, _, _, 
        _, y, _,
        _, _, z] |

        [_, _, x,
        _, y, _, 
        z, _, _]

        if all_eq(x,y,z,&player) => Some(player),

        _ => None,
    }
}

fn all_eq(x: &char, y: &char, z: &char, p: &char) -> bool {
    x == y && y == z && z == p
}

fn search(moves: &[Move<TicTacToeGame>]) -> usize {
    0
}