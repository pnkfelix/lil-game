use tracing::debug;

use serde::Deserialize;

type Res<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync >>;

use futures::{future::FutureExt, StreamExt};

use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    style::Print,
    terminal,
    ExecutableCommand,
};

#[tokio::main]
async fn main() -> Res<()> {
    tracing_subscriber::fmt::init();

    let url_core = std::env::args().skip(1).next();
    let url_core = url_core.unwrap_or_else(|| {
        panic!("need to provide single argument with base URL for game service.")
    });

    let service = GameService::new(url_core);


    let mut session = service.fresh_game().await?;

    // stdout.execute(terminal::Clear(terminal::ClearType::All))?;

    loop {
        match handle_human_player(&mut session).await? {
            Ok(move_description) => {
                session.game_state = move_description.next_board;
                session.player = move_description.next_player;
            }
            Err(QuitGame) => break,
        }
    }
    Ok(())
}

struct QuitGame;

async fn handle_human_player(session: &mut Session) -> Res<Result<MoveDescription, QuitGame>>
{
    let rendered = session.rendered_board().await?;
    let num_lines = rendered.matches('\n').count();

    render_board(&mut session.stdout, 1, &rendered)?;

    session.stdout
        .execute(cursor::MoveTo(1, num_lines as u16 + 1))?
        .execute(terminal::Clear(terminal::ClearType::CurrentLine))?
        ;

    session.stdout
        .execute(Print(&session.player))?
        .execute(Print(" moves: "))?
        ;

    let moves = session.move_list().await?;
    for desc in &moves {
        session.stdout
            .execute(Print(&desc.move_id))?
            .execute(Print(" "))?
            ;
    }

    let mut reader = EventStream::new();

    let query_line = num_lines as u16 + 2;

    // Big idea: when we handle user input, we want to simultaneously:
    // 1. be able to receive new user input, and also
    // 2. render a preview of what their selected move, if any, looks like.
    //
    // My first naive version of this just always attempted a render after
    // building any prefix that matches an entry in `moves`. However, the render
    // system (*by design*) is potentially slow, because it relies on the
    // Lambda-provided service to *do* the rendering, rather than building it
    // into the text here (or including rendered forms in the move list).

    loop {
        // Tracks the user's selected move choice as user types its characters.
        let mut input_choice = String::new();

        // Tracks the currently previewed move, if any.
        let mut preview: Option<String> = None;
        // Tracks number of lines of output from previous preview, if any, below the query line.
        let mut preview_length = 0;
        // Tracks maximum length of any previous preview. This is used as a
        // guess as to where a good place is to emit messages to the user.
        let mut max_preview_length = 0;

        // The player hasn't committed to a move by hitting enter yet.
        // However, we can *show* them what their move looks like, if what
        // they have typed so far happens to match our list of next moves.

        async fn preview_board(session: &mut Session,
                               query_line: u16,
                               preview_length: usize,
                               preview: Option<String>)
                               -> Res<Option<String>>
        {
            if let Some(b) = preview {
                let render_cmd = session.url_core.r(&b);
                let rendered = ask::<RenderResponse>(&render_cmd).await?;

                // delete any past preview.
                session.stdout.execute(cursor::SavePosition)?;
                clear_lines(&mut session.stdout, query_line + 1, preview_length)?;
                render_board(&mut session.stdout, query_line + 1, &rendered.text)?;
                session.stdout.execute(cursor::RestorePosition)?;
                Ok(Some(rendered.text))
            } else {
                Ok(None)
            }
        }

        loop {
            session.stdout
                .execute(cursor::MoveTo(1, query_line))?
                .execute(terminal::Clear(terminal::ClearType::CurrentLine))?
                .execute(Print("?"))?
                .execute(Print(" "))?

            // don't put a space after this; its the prefix that the user might
            // add on to, if there are moves whose identifier is more than one
            // character long.
                .execute(Print(&input_choice))?
                ;





            terminal::enable_raw_mode()?;
            let event = reader.next().fuse();
            let maybe_event = tokio::select! {
                Ok(Some(rendered)) = preview_board(session, query_line, preview_length, preview.take()) => {
                    preview_length = rendered.lines().count();
                    max_preview_length = std::cmp::max(max_preview_length, preview_length);
                    reader.next().await
                }
                maybe_event = event => {
                    preview = None;
                    preview_length = 0;
                    maybe_event
                }
            };
            match maybe_event {
                Some(Ok(Event::Key(event))) => {
                    terminal::disable_raw_mode()?;
                    debug!("{:?}", event);

                    let event::KeyEvent { code, modifiers: _ } = event;
                    match code {
                        event::KeyCode::Char('q') => { return Ok(Err(QuitGame)) }
                        event::KeyCode::Enter => break,
                        event::KeyCode::Backspace => {
                            input_choice.pop();
                        }
                        event::KeyCode::Char(c @ '0'..='9') => {
                            input_choice.push(c);
                        }
                        _ => {}
                    }
                }
                other => debug!("{:?}", other),
            }

            clear_lines(&mut session.stdout, query_line + 1, preview_length)?;

            for desc in &moves {
                if &desc.move_id == &input_choice {
                    preview = Some(desc.next_board.clone());
                    /*
                    let rendered = preview_board(session, query_line + 1, &desc.next_board).await?;
                    preview_length = rendered.lines().count();
                    max_preview_length = std::cmp::max(max_preview_length, preview_length);
                     */
                    break;
                }
            }
        }

        for desc in &moves {
            if &desc.move_id == &input_choice {
                // delete any past preview, then return the selected choice
                clear_lines(&mut session.stdout, query_line + 1, preview_length)?;
                return Ok(Ok(desc.clone()));
            }
        }

        let msg_line = query_line + max_preview_length as u16 + 1;
        session.stdout
            .execute(cursor::MoveTo(1, msg_line))?
            .execute(Print(&format!("You typed `{}`; but you need to select \
                                     one of the {} moves listed above",
                                    input_choice, moves.len())))?
            .execute(cursor::MoveTo(1, query_line + max_preview_length as u16 + 1))?
            .execute(cursor::MoveTo(1, msg_line + 1))?
            .execute(terminal::Clear(terminal::ClearType::CurrentLine))?
            ;
    }
}

fn clear_lines(stdout: &mut impl crossterm::ExecutableCommand,
               start_line: u16,
               num_lines: usize) -> Res<()>
{
    for j in 0..num_lines {
        stdout
            .execute(cursor::MoveTo(1, start_line + j as u16))?
            .execute(terminal::Clear(terminal::ClearType::CurrentLine))?;
    }
    Ok(())
}

fn render_board(stdout: &mut impl crossterm::ExecutableCommand,
                start_line: u16,
                rendered: &str)
                -> Res<()>
{
    clear_lines(stdout, start_line, rendered.lines().count())?;

    for (j, line) in rendered.lines().enumerate() {
        stdout
            .execute(cursor::MoveTo(1, start_line + j as u16))?
            .execute(Print(line))?;
    }

    Ok(())
}

struct GameService {
    url_core: String,
}

trait CommandCore: Sized {
    fn with_char(&self, c: char) -> Self;
    fn pushing(self, s: &str) -> Self;
    fn n(&self) -> Self { self.with_char('n') }
    fn r(&self, board: &str) -> Self { self.with_char('r').pushing(board) }
    fn l(&self, board: &str) -> Self { self.with_char('l').pushing(board) }
}
impl CommandCore for String {
    fn pushing(mut self, s: &str) -> Self {
        self.push_str(s);
        self
    }
    fn with_char(&self, c: char) -> Self {
        let mut new_url = self.clone();
        if !new_url.ends_with("/") {
            new_url.push('/');
        }
        new_url.push(c);
        new_url.push('/');
        new_url
    }
}

async fn ask<T: std::fmt::Debug + for<'a> Deserialize<'a>>(url_cmd: &str) -> Res<T> {
    let resp = reqwest::get(url_cmd).await?;
    let json = resp.json::<T>().await?;
    debug!("{:#?}", json);
    // let value: String = json.remove(key).unwrap().unwrap();
    Ok(json)
}

#[derive(Debug, Deserialize)]
struct FreshResponse {
    command: String,
    parsed_game_state: String,
    player: String,
}

#[derive(Debug, Deserialize)]
struct RenderResponse {
    command: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ListMovesResponse {
    command: String,
    next_game_states: Vec<MoveDescription>,
}

#[derive(Clone, Debug, Deserialize)]
struct MoveDescription {
    move_id: String,
    next_board: String,
    next_player: String,
}

impl GameService {
    fn new(url_core: String) -> Self { GameService { url_core } }

    async fn fresh_game(&self) -> Res<Session> {
        let game_state = ask::<FreshResponse>(&self.url_core.n()).await?;
        let player = game_state.player;
        let game_state = game_state.parsed_game_state;
        let stdout = std::io::stdout();
        Ok(Session { url_core: self.url_core.clone(), game_state, player, stdout })
    }
}

struct Session {
    url_core: String,
    stdout: std::io::Stdout,
    game_state: String,
    player: String,
}

impl Session {
    async fn rendered_board(&self) -> Res<String> {
        let rendered = ask::<RenderResponse>(&self.url_core.r(&self.game_state)).await?;
        Ok(rendered.text)
    }

    async fn move_list(&self) -> Res<Vec<MoveDescription>> {
        let moves = ask::<ListMovesResponse>(&self.url_core.l(&self.game_state)).await?;
        Ok(moves.next_game_states)
    }
}
