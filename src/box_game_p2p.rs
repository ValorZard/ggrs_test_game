extern crate freetype as ft;

use ggrs::{GGRSEvent, PlayerHandle, PlayerType, SessionState};
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
    assert!(args.len() >= 4);

    let port: u16 = args[1].parse().unwrap();
    let local_handle: PlayerHandle = args[2].parse().unwrap();
    let remote_handle: PlayerHandle = 1 - local_handle;
    let remote_addr: SocketAddr = args[3].parse().unwrap();

    // create a GGRS session with two players
    let mut sess = ggrs::start_p2p_session(NUM_PLAYERS as u32, INPUT_SIZE, port).unwrap();

    // add players
    sess.add_player(PlayerType::Local, local_handle).unwrap();
    sess.add_player(PlayerType::Remote(remote_addr), remote_handle)
        .unwrap();

    // optionally, add a spectator
    if args.len() > 4 {
        let spec_addr: SocketAddr = args[4].parse().unwrap();
        sess.add_player(PlayerType::Spectator(spec_addr), 2)
            .unwrap();
    }

    // set input delay for the local player
    sess.set_frame_delay(2, local_handle).unwrap();

    // start the GGRS session
    sess.start_session().unwrap();

    // Create a new box game
    let mut game = box_game::BoxGame::new();

    // set render settings
    let font = load_ttf_font("src/assets/FiraSans-Regular.ttf")
        .await
        .unwrap();

    // event loop
    let mut remaining_time = 0.;
    loop {
        remaining_time += get_frame_time();
        while remaining_time >= FPS_INV {
            if sess.current_state() == SessionState::Running {
                // tell GGRS it is time to advance the frame and handle the requests
                let local_input = game.local_input();

                match sess.advance_frame(local_handle, &local_input) {
                    Ok(requests) => game.handle_requests(requests),
                    Err(ggrs::GGRSError::PredictionThreshold) => {
                        println!("Skipping a frame: PredictionThreshold")
                    }
                    Err(e) => panic!("{}", e),
                }
            }

            // handle GGRS events
            for event in sess.events() {
                if let GGRSEvent::WaitRecommendation { skip_frames } = event {
                    // frames_to_skip += skip_frames
                }
                println!("Event: {:?}", event);
            }
            remaining_time -= FPS_INV;
        }

        // idle
        sess.poll_remote_clients();

        // update key state
        game.key_states[0] = is_key_down(KeyCode::W);
        game.key_states[1] = is_key_down(KeyCode::A);
        game.key_states[2] = is_key_down(KeyCode::S);
        game.key_states[3] = is_key_down(KeyCode::D);

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
