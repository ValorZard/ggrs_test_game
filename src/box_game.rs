extern crate freetype as ft;

use macroquad::prelude::*;

use ggrs::{Frame, GGRSRequest, GameInput, GameState, GameStateCell, NULL_FRAME};
use serde::{Deserialize, Serialize};

const FPS: u64 = 60;
const NUM_PLAYERS: usize = 2;
const CHECKSUM_PERIOD: i32 = 100;


pub const PLAYER_COLORS: [Color; 2] = [BLUE, ORANGE];

pub const PLAYER_SIZE: f32 = 50.0;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

const MOVEMENT_SPEED: f32 = 15.0 / FPS as f32;
const ROTATION_SPEED: f32 = 2.5 / FPS as f32;
const MAX_SPEED: f32 = 7.0;
const FRICTION: f32 = 0.98;

/// Computes the fletcher16 checksum, copied from wikipedia: <https://en.wikipedia.org/wiki/Fletcher%27s_checksum>
fn fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;

    for index in 0..data.len() {
        sum1 = (sum1 + data[index] as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }

    (sum2 << 8) | sum1
}

pub struct BoxGame {
    game_state: BoxGameState,
    pub key_states: [bool; 4],
    //font: PathBuf,
    last_checksum: (Frame, u64),
    periodic_checksum: (Frame, u64),
}

impl BoxGame {
    pub fn new() -> Self {
        Self {
            game_state: BoxGameState::new(),
            key_states: [false; 4],
            //font,
            last_checksum: (NULL_FRAME, 0),
            periodic_checksum: (NULL_FRAME, 0),
        }
    }

    pub fn game_state(&self) -> &BoxGameState {
        &self.game_state
    }

    pub fn last_checksum(&self) -> (i32, u64){
        self.last_checksum
    }

    pub fn periodic_checksum(&self) -> (i32, u64){
        self.periodic_checksum
    }

    pub fn handle_requests(&mut self, requests: Vec<GGRSRequest>) {
        for request in requests {
            match request {
                GGRSRequest::LoadGameState { cell } => self.load_game_state(cell),
                GGRSRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                GGRSRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    fn save_game_state(&mut self, cell: GameStateCell, frame: Frame) {
        assert_eq!(self.game_state.frame, frame);
        let buffer = bincode::serialize(&self.game_state).unwrap();
        let checksum = fletcher16(&buffer) as u64;

        cell.save(GameState::new(frame, Some(buffer), Some(checksum)));
    }

    fn load_game_state(&mut self, cell: GameStateCell) {
        let state_to_load = cell.load();
        self.game_state = bincode::deserialize(&state_to_load.buffer.unwrap()).unwrap();
    }

    fn advance_frame(&mut self, inputs: Vec<GameInput>) {
        // increase the frame counter
        self.game_state.frame += 1;

        for i in 0..NUM_PLAYERS {
            // get input of that player
            let input;
            // check if the player is disconnected (disconnected players might maybe do something different)
            if inputs[i].frame == NULL_FRAME {
                input = 4; // disconnected players spin
            } else {
                input = bincode::deserialize(inputs[i].input()).unwrap();
            }

            // old values
            let (old_x, old_y) = self.game_state.positions[i];
            let (old_vel_x, old_vel_y) = self.game_state.velocities[i];
            let mut rot = self.game_state.rotations[i];

            // slow down
            let mut vel_x = old_vel_x * FRICTION;
            let mut vel_y = old_vel_y * FRICTION;

            // thrust
            if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
                vel_x += MOVEMENT_SPEED * rot.cos();
                vel_y += MOVEMENT_SPEED * rot.sin();
            }
            //break
            if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
                vel_x -= MOVEMENT_SPEED * rot.cos();
                vel_y -= MOVEMENT_SPEED * rot.sin();
            }
            // turn left
            if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
                rot = (rot - ROTATION_SPEED).rem_euclid(2.0 * std::f32::consts::PI);
            }
            // turn right
            if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
                rot = (rot + ROTATION_SPEED).rem_euclid(2.0 * std::f32::consts::PI);
            }

            // limit speed
            let magnitude = (vel_x * vel_x + vel_y * vel_y).sqrt();
            if magnitude > MAX_SPEED {
                vel_x = (vel_x * MAX_SPEED) / magnitude;
                vel_y = (vel_y * MAX_SPEED) / magnitude;
            }

            // compute new position
            let mut x = old_x + vel_x;
            let mut y = old_y + vel_y;

            //constrain boxes to canvas borders
            x = x.max(0.0);
            x = x.min(screen_width());
            y = y.max(0.0);
            y = y.min(screen_width());

            self.game_state.positions[i] = (x, y);
            self.game_state.velocities[i] = (vel_x, vel_y);
            self.game_state.rotations[i] = rot;
        }

        // TODO: inefficient to serialize the gamestate here just for the checksum
        // remember checksum to render it later
        let buffer = bincode::serialize(&self.game_state).unwrap();
        let checksum = fletcher16(&buffer) as u64;
        self.last_checksum = (self.game_state.frame, checksum);
        if self.game_state.frame % CHECKSUM_PERIOD == 0 {
            self.periodic_checksum = (self.game_state.frame, checksum);
        }
    }

    #[allow(dead_code)]
    pub fn local_input(&self) -> Vec<u8> {
        // Create a set of pressed Keys.
        let mut input: u8 = 0;

        // ugly, but it works...
        if self.key_states[0] {
            input |= INPUT_UP;
        }
        if self.key_states[1] {
            input |= INPUT_LEFT;
        }
        if self.key_states[2] {
            input |= INPUT_DOWN;
        }
        if self.key_states[3] {
            input |= INPUT_RIGHT;
        }

        bincode::serialize(&input).unwrap()
    }
}

// BoxGameState holds all relevant information about the game state
#[derive(Serialize, Deserialize)]
pub struct BoxGameState {
    pub frame: i32,
    pub positions: Vec<(f32, f32)>,
    pub velocities: Vec<(f32, f32)>,
    pub rotations: Vec<f32>,
}

impl BoxGameState {
    pub fn new() -> Self {
        let mut positions = Vec::new();
        let mut velocities = Vec::new();
        let mut rotations = Vec::new();
        for i in 0..NUM_PLAYERS as i32 {
            let x: f32 = screen_width()  / 2. + (2. * (i as f32) - 1.) * (screen_width() / 4.);
            let y: f32 = screen_height()  / 2.;
            positions.push((x, y));
            velocities.push((0.0, 0.0));
            rotations.push(0.0);
        }

        Self {
            frame: 0,
            positions,
            velocities,
            rotations,
        }
    }
}
