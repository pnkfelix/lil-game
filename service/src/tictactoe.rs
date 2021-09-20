use crate::game_core::{Game, Move, Player};
use std::borrow::Cow;

pub type TicTacToeBoard = [char; 9];

#[derive(Clone, Debug)]
pub struct TicTacToeGame {
    pub board: TicTacToeBoard,
    pub player: Player,
}

impl Default for TicTacToeGame {
    fn default() -> Self {
        Self { board: ['-'; 9], player: 'X' }
    }
}

impl Game for TicTacToeGame {
    const NAME: &'static str = "TicTacToe";

    fn unparse(&self) -> String {
        self.board.iter().collect()
    }

    fn parse(input: &str) -> Result<Self, Cow<str>> {
        let mut g = TicTacToeGame { board: ['-'; 9], player: 'X' };
        if input.chars().count() != 9 { return Err("input must be length 9".into());}
        let mut num_x = 0;
        let mut num_o = 0;
        for (i, c) in input.chars().enumerate() {
            match c {
                '-' | 'X' | 'O' => g.board[i] = c,
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
            if self.board[i-1] == '-' {
                let mut next_board = self.board;
                next_board[i-1] = self.player;
                let victor = victory(&next_board, self.player);
                let avail = space_available(&next_board);
                let end_game = if let Some(p) = victor {
                    Some(Some(p).into_iter().collect())
                } else if !avail {
                    Some(None.into_iter().collect())
                } else {
                    None
                };
                v.push(Move {
                    id: i as u32,
                    end_game,
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
                    format!(" {} ", if c == '-' { ' ' } else { c })
                }
                format!(" {a} | {b} | {c} \n\
                         -----|-----|-----\n \
                          {m} | {n} | {o} \n\
                         -----|-----|-----\n \
                          {x} | {y} | {z} \n",
                         a=pad(a), b=pad(b), c=pad(c),
                         m=pad(m), n=pad(n), o=pad(o),
                         x=pad(x), y=pad(y), z=pad(z))
            }
        }
    }

    fn value_for(&self, p: Player) -> i64 {
        let other = if p == 'X' { 'O' } else { 'X' };
        if victory(&self.board, p) == Some(p) {
            100000
        } else if victory(&self.board, other) == Some(other) {
            -100000
        } else {
            0
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

fn space_available(board: &TicTacToeBoard) -> bool {
    board.iter().any(|c| *c == '-')
}
