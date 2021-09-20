use lambda_runtime::{handler_fn, Context};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::Error as _;

use std::convert::TryInto;
use std::default::Default;

use crate::game_core::{Game, Command};

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
    victory: Option<Vec<String>>,
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


    let c: Command = cmd.chars().next().unwrap().try_into()?;

    let game = if c == Command::NewGame {
        Default::default()
    } else {
        tictactoe::TicTacToeGame::parse(state)?
    };
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
            let choice = game_core::search(&moves[..]).await;
            selected_move = Some((choice.id.to_string(), choice.next_state.board.iter().collect()));
            victory = choice.end_game.as_ref().map(|v| {
                v.iter().map(|c|c.to_string()).collect()
            });
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

mod game_core;
mod tictactoe;
