use std::{fmt::Display, sync::Mutex, thread, time::Instant};

use once_cell::sync::{Lazy, OnceCell};
use rdev::{self, Key};

#[derive(Clone, Debug)]
pub struct Keypress {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
    pub key: String,
}

impl Display for Keypress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output: Vec<String> = Vec::new();
        if self.shift {
            output.push("Shift".to_string());
        }
        if self.ctrl {
            output.push("Ctrl".to_string());
        }
        if self.alt {
            output.push("Alt".to_string());
        }
        if self.meta {
            output.push("Meta".to_string());
        }
        output.push(self.key.clone());
        write!(f, "{}", output.join(" + "))
    }
}

static LISTEN_RESULT: OnceCell<()> = OnceCell::new(); //Lazy::new(|| {rdev::listen(callback)});
static LAST_KEYPRESS: Lazy<Mutex<Keypress>> = Lazy::new(|| {
    Mutex::new(Keypress {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
        key: "".to_string(),
    })
});

fn get_keyname(name: Option<String>, key: Key) -> String {
    name.unwrap_or(format!("{:?}", key))
}

fn callback(event: rdev::Event) {
    match event.event_type {
        rdev::EventType::KeyPress(key) => match key {
            Key::ShiftLeft | Key::ShiftRight => LAST_KEYPRESS.lock().unwrap().shift = true,
            Key::Alt | Key::AltGr => LAST_KEYPRESS.lock().unwrap().alt = true,
            Key::ControlLeft | Key::ControlRight => LAST_KEYPRESS.lock().unwrap().ctrl = true,
            Key::MetaLeft | Key::MetaRight => LAST_KEYPRESS.lock().unwrap().meta = true,
            _ => LAST_KEYPRESS.lock().unwrap().key = get_keyname(event.name, key),
        },
        rdev::EventType::KeyRelease(key) => match key {
            Key::ShiftLeft | Key::ShiftRight => LAST_KEYPRESS.lock().unwrap().shift = true,
            Key::Alt | Key::AltGr => LAST_KEYPRESS.lock().unwrap().alt = true,
            Key::ControlLeft | Key::ControlRight => LAST_KEYPRESS.lock().unwrap().ctrl = true,
            Key::MetaLeft | Key::MetaRight => LAST_KEYPRESS.lock().unwrap().meta = true,
            _ => {
                let guard = LAST_KEYPRESS.lock();
                let mut keypress = guard.unwrap();
                let current_keyname = get_keyname(event.name, key);
                if (keypress.key.len() > 0 && keypress.key == current_keyname) {
                    keypress.key = "".to_string();
                }
            }
        },
        _ => {}
    };
    let foo: Option<String> = None;
}

pub fn get_keypress() -> Option<Keypress> {
    LISTEN_RESULT.get_or_init(|| {
        thread::spawn(|| rdev::listen(callback));
        ()
    });
    let starttime = Instant::now();
    //wait up to 10 sec to get a real keypress, if not, just bail and return whatever we got
    while (starttime.elapsed().as_secs() < 10) {
        let kp = LAST_KEYPRESS.lock().unwrap().clone();
        if (kp.key.len() > 0) {
            return Some(kp);
        }
    }
    return None;
}
