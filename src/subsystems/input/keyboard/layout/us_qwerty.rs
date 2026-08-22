use super::{KeyCode, KeyboardLayout, LockState, Modifiers};

#[derive(Clone, Copy, Debug, Default)]
pub struct UsQwerty;

impl KeyboardLayout for UsQwerty {
    fn translate(&self, code: KeyCode, modifiers: Modifiers, locks: LockState) -> Option<char> {
        if modifiers.control() || modifiers.alt() || modifiers.meta() {
            return None;
        }

        if let Some(letter) = letter(code) {
            return Some(if modifiers.shift() ^ locks.caps_lock {
                letter.to_ascii_uppercase()
            } else {
                letter
            });
        }

        if let Some((normal, shifted)) = symbol(code) {
            return Some(if modifiers.shift() { shifted } else { normal });
        }

        if let Some(operator) = numpad_operator(code) {
            return Some(operator);
        }

        if locks.num_lock && !modifiers.shift() {
            return numpad_digit(code);
        }

        None
    }
}

fn letter(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::A => 'a',
        KeyCode::B => 'b',
        KeyCode::C => 'c',
        KeyCode::D => 'd',
        KeyCode::E => 'e',
        KeyCode::F => 'f',
        KeyCode::G => 'g',
        KeyCode::H => 'h',
        KeyCode::I => 'i',
        KeyCode::J => 'j',
        KeyCode::K => 'k',
        KeyCode::L => 'l',
        KeyCode::M => 'm',
        KeyCode::N => 'n',
        KeyCode::O => 'o',
        KeyCode::P => 'p',
        KeyCode::Q => 'q',
        KeyCode::R => 'r',
        KeyCode::S => 's',
        KeyCode::T => 't',
        KeyCode::U => 'u',
        KeyCode::V => 'v',
        KeyCode::W => 'w',
        KeyCode::X => 'x',
        KeyCode::Y => 'y',
        KeyCode::Z => 'z',
        _ => return None,
    })
}

fn symbol(code: KeyCode) -> Option<(char, char)> {
    Some(match code {
        KeyCode::Backquote => ('`', '~'),
        KeyCode::Digit1 => ('1', '!'),
        KeyCode::Digit2 => ('2', '@'),
        KeyCode::Digit3 => ('3', '#'),
        KeyCode::Digit4 => ('4', '$'),
        KeyCode::Digit5 => ('5', '%'),
        KeyCode::Digit6 => ('6', '^'),
        KeyCode::Digit7 => ('7', '&'),
        KeyCode::Digit8 => ('8', '*'),
        KeyCode::Digit9 => ('9', '('),
        KeyCode::Digit0 => ('0', ')'),
        KeyCode::Minus => ('-', '_'),
        KeyCode::Equal => ('=', '+'),
        KeyCode::LeftBracket => ('[', '{'),
        KeyCode::RightBracket => (']', '}'),
        KeyCode::Backslash => ('\\', '|'),
        KeyCode::Semicolon => (';', ':'),
        KeyCode::Quote => ('\'', '"'),
        KeyCode::Comma => (',', '<'),
        KeyCode::Period => ('.', '>'),
        KeyCode::Slash => ('/', '?'),
        KeyCode::Space => (' ', ' '),
        _ => return None,
    })
}

fn numpad_operator(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::NumpadDivide => '/',
        KeyCode::NumpadMultiply => '*',
        KeyCode::NumpadSubtract => '-',
        KeyCode::NumpadAdd => '+',
        _ => return None,
    })
}

fn numpad_digit(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::Numpad0 => '0',
        KeyCode::Numpad1 => '1',
        KeyCode::Numpad2 => '2',
        KeyCode::Numpad3 => '3',
        KeyCode::Numpad4 => '4',
        KeyCode::Numpad5 => '5',
        KeyCode::Numpad6 => '6',
        KeyCode::Numpad7 => '7',
        KeyCode::Numpad8 => '8',
        KeyCode::Numpad9 => '9',
        KeyCode::NumpadDecimal => '.',
        _ => return None,
    })
}
