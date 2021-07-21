extern crate freetype as ft;

use ggrs::{GGRSError, GGRSEvent, SessionState};
use macroquad::prelude::*;
use std::env;
use std::net::SocketAddr;

//const FPS: u64 = 60;
const FPS_INV: f32 = 1. / 60.;
const NUM_PLAYERS: usize = 2;
const INPUT_SIZE: usize = std::mem::size_of::<u8>();

mod box_game;

#[macroquad::main("Controllable box")]
async fn main() {
    // read cmd line arguments very clumsily
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 3);

    let port: u16 = args[1].parse().unwrap();
    let host_addr: SocketAddr = args[2].parse().unwrap();

    // create a GGRS session for a spectator
    let mut sess =
        ggrs::start_p2p_spectator_session(NUM_PLAYERS as u32, INPUT_SIZE, port, host_addr).unwrap();

    // start the GGRS session
    sess.start_session().unwrap();

    // Create a new box game
    let mut game = box_game::BoxGame::new();

    let mut remaining_time = 0.;
    loop {
        remaining_time += get_frame_time();
        while remaining_time >= FPS_INV {
            if sess.current_state() == SessionState::Running {
                // tell GGRS it is time to advance the frame and handle the requests
                match sess.advance_frame() {
                    Ok(requests) => game.handle_requests(requests),
                    Err(GGRSError::PredictionThreshold) => {
                        println!("Skipping a frame: Waiting for input from host.");
                    }
                    Err(e) => panic!("{}", e),
                }
            }

            // handle GGRS events
            for event in sess.events() {
                println!("Event: {:?}", event);
                if let GGRSEvent::Disconnected { .. } = event {
                    println!("Disconnected from host.");
                }
            }

            remaining_time -= FPS_INV;
        }

        // idle
        sess.poll_remote_clients();

        render(&game);

        next_frame().await
    }

    fn render(game: &box_game::BoxGame)
    {
        clear_background(BLACK);

        let checksum_string = format!(
            "Frame {}: Checksum {}",
            game.last_checksum().0, game.last_checksum().1
        );
        let periodic_string = format!(
            "Frame {}: Checksum {}",
            game.periodic_checksum().0, game.periodic_checksum().1
        );

        draw_text_ex(&checksum_string, 20.0, 20.0, TextParams::default());
        draw_text_ex(&periodic_string, 20.0, 40.0, TextParams::default());

        // draw the player rectangles
        for i in 0..NUM_PLAYERS {
            let (x, y) = game.game_state().positions[i];
            let rotation = game.game_state().rotations[i];

            draw_rectangle(x, y, box_game::PLAYER_SIZE, box_game::PLAYER_SIZE, box_game::PLAYER_COLORS[i]);
        }
    }
}
