use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

pub struct Keys {
    keys: [bool; 16],
}

impl Keys {
    pub fn new() -> Keys {
        Keys { keys: [false; 16] }
    }

    pub fn key_pressed(&self, keycode: usize) -> Option<bool> {
        let key_map = match keycode {
            0x1 => Some(0),
            0x2 => Some(1),
            0x3 => Some(2),
            0xC => Some(3),
            0x4 => Some(4),
            0x5 => Some(5),
            0x6 => Some(6),
            0xD => Some(7),
            0x7 => Some(8),
            0x8 => Some(9),
            0x9 => Some(10),
            0xE => Some(11),
            0xA => Some(12),
            0x0 => Some(13),
            0xB => Some(14),
            0xF => Some(15),
            _ => None,
        };
        if let Some(index) = key_map {
            return Some(self.keys[index]);
        }
        None
    }
}

pub struct Input {
    event_pump: sdl2::EventPump,
}

impl Input {
    pub fn new(context: &sdl2::Sdl) -> Input {
        Input {
            event_pump: context.event_pump().unwrap(),
        }
    }

    pub fn poll_for_input(&mut self) -> Option<Keys> {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return None,
                _ => {}
            }
        }

        let key_events: Vec<Keycode> = self
            .event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        let mut pressed: Keys = Keys::new();
        for event in key_events {
            let index: Option<usize> = match event {
                Keycode::Num1 => Some(0),
                Keycode::Num2 => Some(1),
                Keycode::Num3 => Some(2),
                Keycode::Num4 => Some(3),
                Keycode::Q => Some(4),
                Keycode::W => Some(5),
                Keycode::E => Some(6),
                Keycode::R => Some(7),
                Keycode::A => Some(8),
                Keycode::S => Some(9),
                Keycode::D => Some(10),
                Keycode::F => Some(11),
                Keycode::Z => Some(12),
                Keycode::X => Some(13),
                Keycode::C => Some(14),
                Keycode::V => Some(15),
                _ => None,
            };
            match index {
                Some(i) => pressed.keys[i] = true,
                None => {}
            }
        }
        Some(pressed)
    }
}
