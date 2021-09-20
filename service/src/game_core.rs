//! The `game_core` module defines the interfaces that are common to all board
//! games defined in this architecture.

use smallvec::SmallVec;
use std::borrow::Cow;
use std::convert::TryFrom;

/// To simplify things, we will identify players by single characters,
/// and build that assumption into the architecture.
///
/// E.g. in tic-tac-toe, 'X' and 'O' are the players, while in Connect Four, 'R'
/// and 'Y' stand for "red" and "yellow" players.
pub type Player = char;

/// Each move for a given game-state is identified by a positive number.
///
/// For example, the locations on a tic-tac-toe board can be labelled with
///
/// ```
///   1  |  2  |  3
/// -----|-----|-----
///   4  |  5  |  6
/// -----|-----|-----
///   7  |  8  |  9
/// ```
///
/// and any tic-tac-toe state can identify all possible moves by using these
/// labels, which will tend to be convenient for human players.
///
/// (We assume 32-bits suffice to encode any one discrete actions on a board,
/// and even that is probably overkill.)
pub type MoveId = u32;

/// A `Move` represents a transition of a game from one state to a new state. A
/// move has an identifier (for use for players to select it from a set of
/// moves), the next state of the game, and, for convenience, a `end_game`
/// field: when a move results in the end of the game, `end_game` will be set to
/// `Some(victors)`.
#[derive(Clone, Debug)]
pub struct Move<B: Game> {
    /// Uniquely identifies this move amongst its siblings for a given game
    /// state.
    pub id: MoveId,

    /// The next game state that results if this move is taken.
    pub next_state: B,

    /// If `None`, then taking this move does not end the game. If `Some`, then
    /// taking this move ends the game, and the value it carries is the list of
    /// winning players. It uses `SmallVec` with a singleton array, because in
    /// most games, the end game always results in a single winning player.
    pub end_game: Option<SmallVec<[Player; 1]>>,
}

/// A `Game` represents the state of a turn-based game. You can serialize or
/// deserialize it from a string, you can render it to a human-readable block of
/// text, or you can query it for a list of possible actions ("moves") to take
/// on the game state.
///
/// A valid initial game state can be build via `Default::default()`.
///
/// The games we use as examples will be simple enough to have human-readable
/// strings for their serialized representation. For example, tictactoe can be
/// summarized with a string of 9 characters, where each character is either
/// 'X', 'O', or '-'.
///
/// Also, since we will be embedding these strings directly into the service
/// URI, the serialized strings should be valid path segments for a URI: so, to
/// be safe, stick to non-whitespace alphanumeric characters, or '-'.
pub trait Game: Sized + Clone + Default {
    const NAME: &'static str;

    /// Deserializes an input string to an instance of the game, or returns an
    /// error with a description of why deserialization failed.
    fn parse(input: &str) -> Result<Self, Cow<str>>;

    /// Converts a game state to its corresponding serialized string.
    fn unparse(&self) -> String;

    /// Produces the set of "moves" that are available to take from the current
    /// game state.
    ///
    /// Note that while in most cases a "move" will correspond to a player's
    /// move in the game (e.g. putting an 'X' in the upper-right corner in
    /// tictactoe), more complex games may prefer breaking a player's move up
    /// into multiple actions (e.g., perhaps in Chess the first "move" of a
    /// player will be to select which piece they are moving, and the second
    /// "move" will be to select the new position for that piece on the board).
    fn moves(&self) -> Vec<Move<Self>>;

    /// Renders the game state into a human visible depiction of the globally
    /// visible board.
    fn render_to_text(&self) -> String;
}

// FIXME: The interface for `Game` does not yet carry enough info for us to
// generically make choices here.

/// Chooses the best move amongst a provided set of moves.
pub fn search<B: Game>(moves: &[Move<B>]) -> &Move<B> {
    &moves[0]
}

#[derive(Debug)]
pub struct UnknownCommand;
impl std::fmt::Display for UnknownCommand {
    fn fmt(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(w, "unknown command")
    }
}
impl std::error::Error for UnknownCommand {

}

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    NewGame,
    List,
    RenderToText,
    Select,
}

impl TryFrom<char> for Command {
    type Error = UnknownCommand;

    fn try_from(c: char) -> Result<Self, UnknownCommand> {
        Ok(match c {
            'n' => Command::NewGame,
            'l' => Command::List,
            'r' => Command::RenderToText,
            's' => Command::Select,
            _ => return Err(UnknownCommand),
        })
    }
}
