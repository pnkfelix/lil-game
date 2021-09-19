use std::io::{self, BufRead, Write};
use std::convert::TryInto;

use crate::game_core::{Command, Game};

mod game_core;
mod tictactoe;

type TheGame = crate::tictactoe::TicTacToeGame;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let mut game: TheGame = Default::default();

    println!("{}", TheGame::NAME);
    println!("{}", game.render_to_text());
    let prompt = || {
        println!("next command: [n, l, r, s] (with optional /<game>)");
        print!("? ");
        std::io::stdout().flush().unwrap();
    };

    prompt();
    while let Some(Ok(line)) = lines.next() {
        // println!("line: {}", line);
        if line == "" {
            // no command provided; retry.
            prompt();
            continue;
        }
        let (cmd, slash_state) = line.split_at(1);
        if slash_state == "" {
            // no overriding state provided; reuse the current game.
            println!("game: {:?}", game.unparse());
        } else {
            let (slash, state) = slash_state.split_at(1);
            if slash != "/" {
                println!("provide either <C> or <C>/<game> for command");
                prompt();
                continue;
            }

            game = match TheGame::parse(state) {
                Ok(game) => game,
                Err(msg) => {
                    println!("failed to parse game due to {}", msg);
                    println!("provide either <C> or <C>/<game> for command");
                    prompt();
                    continue;
                }
            }
        }

        let c: Command = cmd.chars().next().unwrap().try_into()?;

        let unparsed = game.unparse();
        match c {
            Command::NewGame => {
                game = Default::default();
                println!("new-game: {:?}", game.unparse());
            }
            Command::List => {
                let moves = game.moves();
                let moves_unparsed = moves.iter()
                    .map(|m|(m.id, m.next_state.unparse()))
                    .collect::<Vec<_>>();

                'choose: loop {
                    println!("list {:?} : {:?}", unparsed, moves_unparsed);

                    println!("choose a move from list above");
                    println!("(you will see preview of it before you commit to it.)");
                    let (num, m) = if let Some(Ok(line)) = lines.next() {
                        let num: u32 = match line.parse() {
                            Ok(num) => num,
                            Err(msg) => {
                                println!("{} is not a number, due to {}", line, msg);
                                println!("Please try again.");
                                continue 'choose;
                            }
                        };
                        match moves.iter().filter(|m| m.id == num).next() {
                            Some(m) => (num, m),
                            None => {
                                println!("The number {} is not in the list", num);
                                println!("Please try again.");
                                continue 'choose;
                            }
                        }
                    } else {
                        continue 'choose;
                    };

                    'confirm: loop {
                        println!("Move {} yields \n{}",
                                 num,
                                 m.next_state.render_to_text());
                        println!("Is this what you want (Y/n)?");
                        if let Some(Ok(line)) = lines.next() {
                            match &line.to_lowercase()[..] {
                                "n" | "no" => continue 'choose,
                                "" | "y" | "yes" => {
                                    game = m.next_state.clone();
                                    break 'choose;
                                }
                                _ => {
                                    println!("{} is not an expected y/n answer.", line);
                                    println!("Please try again");
                                    continue 'confirm;
                                }
                            }
                        }
                    }
                }
            }
            Command::RenderToText => {
                println!("render {:?} :\n{}", unparsed, game.render_to_text());
            }
            Command::Select => {
                println!("select {:?} : {:?}", unparsed, game_core::search(&game.moves()));
            }
        }

        prompt();
    }
    Ok(())
}
