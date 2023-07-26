use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use invasion::{
    frame::{self, new_frame, Drawable},
    invaders::Invaders,
    player::Player,
    render,
};
use rusty_audio::Audio;
use std::{
    error::Error,
    sync::mpsc,
    time::{Duration, Instant},
    {io, thread},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    let mut stdout = io::stdout();

    for quak in &["come", "bye", "receive", "send", "run", "success"] {
        audio.add(quak, &format!("assets/sounds/{}.wav", quak));
    }
    audio.play("run");

    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);

        loop {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    'gameloop: loop {
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();

        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("send");
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("bye");
                        break 'gameloop;
                    }
                    _ => {}
                }
            }
        }

        player.update(delta);
        if invaders.update(delta) {
            audio.play("come");
        }
        if player.detect_hits(&mut invaders) {
            audio.play("receive");
        }

        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }

        let _ = render_tx.send(curr_frame);

        thread::sleep(Duration::from_micros(1));

        if invaders.all_killed() {
            audio.play("success");
            break 'gameloop;
        }
        if invaders.reached_bottom() {
            audio.play("bye");
            break 'gameloop;
        }
    }

    drop(render_tx);
    render_handle.join().unwrap();

    audio.wait();

    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
