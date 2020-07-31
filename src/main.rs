use eyre::{eyre, Result};
use gilrs::{Axis, Button, Event, EventType, Gilrs};
use std::collections::HashMap;
use std::convert::TryInto;
use std::default::Default;
use wmidi::{Channel, ControlNumber, MidiMessage, Note};

#[derive(Clone, Debug)]
pub struct Config {
    output_port_name: String,
    output_midi_channel: Channel,
    keys: HashMap<Button, Note>,
    analog_button_ccs: HashMap<Button, ControlNumber>,
    axis_ccs: HashMap<Axis, ControlNumber>,
}

impl Default for Config {
    fn default() -> Config {
        let mut cfg = Config {
            output_port_name: "xbox".to_string(),
            output_midi_channel: Channel::Ch15,
            keys: HashMap::new(),
            analog_button_ccs: HashMap::new(),
            axis_ccs: HashMap::new(),
        };
        cfg.keys.extend(vec![
            (Button::North, Note::C1),
            (Button::East, Note::D1),
            (Button::South, Note::E1),
            (Button::West, Note::F1),
            (Button::LeftTrigger, Note::A2),
            (Button::RightTrigger, Note::B2),
            (Button::Start, Note::C3),
            (Button::Select, Note::D3),
            (Button::Mode, Note::E3),
            (Button::DPadUp, Note::A4),
            (Button::DPadDown, Note::B4),
            (Button::DPadLeft, Note::C4),
            (Button::DPadRight, Note::D4),
        ]);
        cfg.analog_button_ccs.extend(vec![
            (Button::LeftTrigger2, 1_u8.try_into().unwrap()),
            (Button::RightTrigger2, 2_u8.try_into().unwrap()),
        ]);
        cfg.axis_ccs.extend(vec![
            (Axis::LeftStickX, 3_u8.try_into().unwrap()),
            (Axis::LeftStickY, 4_u8.try_into().unwrap()),
            (Axis::RightStickX, 5_u8.try_into().unwrap()),
            (Axis::RightStickY, 6_u8.try_into().unwrap()),
        ]);
        cfg
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let mut gilrs = Gilrs::new().map_err(|e| eyre!("{}", e))?;
    let midi_out = midir::MidiOutput::new("gamepad2midi")?;
    let cfg = Config::default();
    log::info!("Config: {:#?}", cfg);
    let mut connection = None;

    for mop in midi_out.ports().iter() {
        let pn = midi_out.port_name(mop)?;
        log::info!("Output port: {}", pn);
        if pn == cfg.output_port_name {
            connection = Some(midi_out.connect(mop, "gamepad2midi")?);
            break;
        }
    }

    for (id, gamepad) in gilrs.gamepads() {
        log::info!("id({:?}) {}", id, gamepad.name());
    }
    let mut outbuf = Vec::new();
    loop {
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            if let Some(mm) = match event {
                EventType::ButtonChanged(btn, pos, code) => {
                    log::debug!("{:?} {} {:?} {} {}", time, id, btn, pos, code);
                    if let Some(cc) = cfg.analog_button_ccs.get(&btn) {
                        let mm = MidiMessage::ControlChange(
                            cfg.output_midi_channel,
                            *cc,
                            abs_float_to_midi(pos),
                        );
                        Some(mm)
                    } else {
                        None
                    }
                }
                EventType::ButtonPressed(btn, code) => {
                    log::debug!("{:?} {} {:?} press {}", time, id, btn, code);
                    if let Some(note) = cfg.keys.get(&btn) {
                        let mm = MidiMessage::NoteOn(
                            cfg.output_midi_channel,
                            *note,
                            80u8.try_into().unwrap(),
                        );
                        Some(mm)
                    } else {
                        None
                    }
                }
                EventType::ButtonReleased(btn, code) => {
                    log::debug!("{:?} {} {:?} press {}", time, id, btn, code);
                    if let Some(note) = cfg.keys.get(&btn) {
                        let mm = MidiMessage::NoteOff(
                            cfg.output_midi_channel,
                            *note,
                            80u8.try_into().unwrap(),
                        );
                        Some(mm)
                    } else {
                        None
                    }
                }
                EventType::AxisChanged(ax, pos, code) => {
                    log::debug!("{:?} {} {:?} {} {}", time, id, ax, pos, code);
                    if let Some(cc) = cfg.axis_ccs.get(&ax) {
                        let mm = MidiMessage::ControlChange(
                            cfg.output_midi_channel,
                            *cc,
                            centered_float_to_midi(pos),
                        );
                        Some(mm)
                    } else {
                        None
                    }
                }
                other => {
                    log::debug!("{:?} {} {:?}", time, id, other);
                    None
                }
            } {
                log::debug!("Would send: {:?}", mm);
                if let Some(ref mut mop) = connection {
                    outbuf.clear();
                    outbuf.resize(mm.bytes_size(), 0);
                    mm.copy_to_slice(&mut outbuf)?;
                    mop.send(&outbuf)?;
                }
            }
        }
    }
}

fn abs_float_to_midi(pos: f32) -> wmidi::U7 {
    let b = pos * 128.0;
    let b = b as u8;
    let b = b.max(0).min(127);
    use std::convert::TryFrom;
    wmidi::U7::try_from(b).unwrap()
}

fn centered_float_to_midi(pos: f32) -> wmidi::U7 {
    let b = 64.0 + pos * 64.0;
    let b = b as u8;
    let b = b.max(0).min(127);
    use std::convert::TryFrom;
    wmidi::U7::try_from(b).unwrap()
}
