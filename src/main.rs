use rand::{self, Rng};
use sdl2::audio::{AudioCVT, AudioCallback, AudioSpecDesired, AudioSpecWAV};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::{fs, time::Duration};

fn render(
    canvas: &mut WindowCanvas,
    bg_color: Color,
    object_textures: &[Rect],
    rect_color: Color,
) -> () {
    canvas.set_draw_color(bg_color);
    canvas.clear();
    canvas.set_draw_color(rect_color);
    canvas.fill_rects(object_textures).unwrap();
    canvas.present();
}
struct Sound {
    data: Vec<u8>,
    volume: f32,
    pos: usize,
}

impl AudioCallback for Sound {
    type Channel = u8;

    fn callback(&mut self, out: &mut [u8]) {
        for dst in out.iter_mut() {
            let pre_scale = *self.data.get(self.pos).unwrap_or(&128);
            let scaled_signed_float = (pre_scale as f32 - 128.0) * self.volume;
            let scaled = (scaled_signed_float + 128.0) as u8;
            *dst = scaled;
            self.pos += 1;
        }
    }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

struct Orb {
    texture: Rect,
    position: (i32, i32),
}

struct Player<'a> {
    window: &'a (u32, u32),
    texture: Rect,
    position: (i32, i32),
    speed: i32,
}

impl<'a> Player<'a> {
    fn move_player(&mut self, x: i32, y: i32) {
        // Level bounds
        if y.is_negative() && !self.at_ceiling() {
            self.texture.offset(0, y * self.speed);
        } else if y.is_positive() && !self.at_floor() {
            self.texture.offset(0, y * self.speed);
        }
        if x.is_negative() && !self.at_left() {
            self.texture.offset(x * self.speed, 0);
        } else if x.is_positive() && !self.at_right() {
            self.texture.offset(x * self.speed, 0);
        }
        self.position = (self.texture.x, self.texture.y);
    }

    fn at_ceiling(&mut self) -> bool {
        self.position.1 <= 0
    }

    fn at_floor(&mut self) -> bool {
        self.position.1 as u32 + 128_u32 >= self.window.1
    }

    fn at_left(&mut self) -> bool {
        self.position.0 <= 0
    }

    fn at_right(&mut self) -> bool {
        self.position.0 as u32 + 128_u32 >= self.window.0
    }
}

const HIGH_SCORE: &str = include_str!("..\\score.txt");
fn check_score(score: u32) -> () {
    let high_score = HIGH_SCORE.parse().expect("Failed to parse high score!");
    if score > high_score {
        fs::write("score.txt", score.to_string()).expect("Failed to write to high score!");
    }
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("GAME", 800, 600)
        .position_centered()
        .borderless()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .software()
        .build()
        .map_err(|e| e.to_string())?;

    let (center_x, center_y): (u32, u32) = (
        canvas.window().size().0 / 2 as u32,
        canvas.window().size().1 / 2 as u32,
    );

    let mut event_pump = sdl_context.event_pump()?;
    let audio = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    let wav_device = audio.open_playback(None, &desired_spec, |spec| {
        let wav = AudioSpecWAV::load_wav(std::path::Path::new("intro.wav")).unwrap();
        let cvt = AudioCVT::new(
            wav.format,
            wav.channels,
            wav.freq,
            spec.format,
            spec.channels,
            spec.freq,
        )
        .unwrap();
        let data = cvt.convert(wav.buffer().to_vec());

        Sound {
            data,
            volume: 0.25,
            pos: 0,
        }
    })?;
    let sqr_device = audio.open_playback(None, &desired_spec, |spec| SquareWave {
        phase_inc: 400.0 / spec.freq as f32,
        phase: 0.0,
        volume: 0.03,
    })?;
    wav_device.resume();

    let mut player: Player = Player {
        window: &canvas.window().size(),
        texture: Rect::new(center_x as i32, center_y as i32, 128, 128),
        position: (center_x as i32, center_y as i32),
        speed: 10,
    };
    let mut orb: Orb = Orb {
        texture: Rect::new(200, 200, 32, 32),
        position: (200, 200),
    };

    canvas.fill_rect(player.texture)?;
    let mut color: Color = Color::RED;
    let (mut fade, mut sound_countdown, mut game_countdown): (f32, f32, f32) = (0.0, 1.0, 60.0);

    let mut score: u32 = 0;

    // GAME LOOP
    'running: loop {
        // handle events
        for keypress in event_pump.keyboard_state().pressed_scancodes() {
            match keypress {
                Scancode::W => player.move_player(0, -1),
                Scancode::A => player.move_player(-1, 0),
                Scancode::S => player.move_player(0, 1),
                Scancode::D => player.move_player(1, 0),
                _ => (),
            }
        }
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    check_score(score);
                    break 'running;
                }
                _ => (),
            }
        }

        if player.texture.has_intersection(orb.texture) {
            color = Color {
                r: rand::thread_rng().gen_range(1..=255),
                g: rand::thread_rng().gen_range(1..=255),
                b: rand::thread_rng().gen_range(1..=255),
                a: 0,
            };
            (orb.texture.x, orb.texture.y) = (
                rand::thread_rng().gen_range(32..canvas.window().size().0 as i32 - 32),
                (rand::thread_rng().gen_range(32..canvas.window().size().1) as i32 - 32),
            );
            orb.position = (orb.texture.x as i32, orb.texture.y as i32);

            score += 1;
            if score == u32::MAX {
                score = 0;
            }

            sqr_device.resume();
            sound_countdown = 1.0;
        }

        // update
        if fade < 1.0 {
            color = Color {
                r: (255.0 * fade) as u8,
                g: 0,
                b: 0,
                a: 0,
            };
            fade += 0.01;
        }
        if sound_countdown > 0.0 {
            sound_countdown -= 0.2;
        } else if sound_countdown <= 0.0 {
            sqr_device.pause();
        }

        if game_countdown > 0.0 {
            game_countdown -= 0.016667;
        } else {
            check_score(score);
            if score > HIGH_SCORE.parse().expect("Failed to read high score!") {
                println!("New high score! Score: {score}");
            } else {
                println!("Game over! Score: {score} | High Score: {HIGH_SCORE}");
            }
            if score == u32::MAX - 1 {
                println!("https://www.youtube.com/watch?v=6e6RK8o1fcs&t=1s");
            }
            break;
        }
        let rounded_time: f32 = format!("{:.2}", game_countdown).trim().parse().unwrap();
        if rounded_time == rounded_time as u32 as f32 {
            println!("{:.0}", rounded_time);
        }

        let to_be_rendered: &[Rect] = &[player.texture, orb.texture];

        // render
        render(&mut canvas, Color::BLACK, to_be_rendered, color);

        // time managment
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}
