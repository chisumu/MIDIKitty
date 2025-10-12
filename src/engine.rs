// what will hopefully be an FM synthesis engine in Rust

use midir::{Ignore, MidiInput, MidiInputConnection};
use rodio::{
    OutputStream,
    source::{SineWave, Skippable, Source},
};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

/// Midi notes, 0 = C-1 and 127 = G9
type Note = u8;

/// Midi velocity, 0 = off, 1 = ppp, 64 = mf, 127 = fff
type Velocity = u8;
const MAX_VELOCITY: Velocity = 127;

/// Convert MIDI note number to center frequency (Hz).
/// https://en.wikipedia.org/wiki/MIDI_tuning_standard
pub fn frequency(note: Note) -> f64 {
    440.0 * ((note as i16 - 69) as f64 / 12.0).exp2()
}

#[derive(Default)]
pub struct Synth {
    connection: Option<MidiInputConnection<()>>,
}

impl fmt::Debug for Synth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<A synthesizer")
    }
}

impl Synth {
    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        // midi input port
        // TODO refactor out into separate method
        let mut midi_in = MidiInput::new("midir reading input")?;
        midi_in.ignore(Ignore::None);

        let in_ports = midi_in.ports();
        let in_port = match in_ports.len() {
            0 => return Err("no input port found".into()),
            1 => {
                println!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                &in_ports[0]
            }
            _ => {
                println!(
                    "Choosing the first input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                &in_ports[0]
            }
        };

        let (tx, rx) = mpsc::channel();

        self.connection = Some(midi_in.connect(
            in_port,
            "midir-read-input",
            // move |stamp, message, _| {
            //     const NOTE_ON_MSG: u8 = 0x90;
            //     const NOTE_OFF_MSG: u8 = 0x80;
            //     println!("got {}: {:?} (len={})", stamp, message, message.len());
            //     match message {
            //         [NOTE_ON_MSG, note, velocity] => {
            //             // inner.play(*note);
            //             println!("note");
            //         }
            //         _ => println!("something else!"),
            //     }
            // },
            move |stamp, message, _| {
                tx.send((stamp, message.to_vec())).unwrap();
            },
            (),
        )?);

        thread::spawn(move || {
            // audio stream
            let mut inner = Inner::default();
            inner.stream = Some(
                rodio::OutputStreamBuilder::open_default_stream().expect("open default stream"),
            );
            // let sink = rodio::Sink::connect_new(&self.stream.mixer());

            for (stamp, message) in rx {
                const NOTE_ON_MSG: u8 = 0x90;
                const NOTE_OFF_MSG: u8 = 0x80;
                // println!("got {}: {:?} (len={})", stamp, message, message.len());
                match message[..] {
                    [NOTE_ON_MSG, note, 0] => inner.stop(note),
                    [NOTE_ON_MSG, note, velocity] => inner.play(note, velocity),
                    [NOTE_OFF_MSG, note, _velocity] => inner.stop(note),
                    // _ => println!("something else!"),
                    _ => {}
                }
            }
        });

        Ok(())
    }
}

#[derive(Default)]
struct Inner {
    stream: Option<OutputStream>,
    sources: HashMap<Note, Sender<()>>,
}

impl Inner {
    fn play(&mut self, note: Note, velocity: Velocity) {
        let (tx, rx) = mpsc::channel();

        if let Some(old_tx) = self.sources.insert(note, tx) {
            // I don't know if we need this?
            old_tx.send(()).unwrap_or_default();
        };

        let source = SineWave::new(frequency(note) as f32)
            // .take_duration(Duration::from_secs_f32(0.25))
            .amplify_normalized(velocity as f32 / MAX_VELOCITY as f32)
            .skippable()
            .periodic_access(Duration::from_micros(100), move |s| {
                if let Ok(_) = rx.try_recv() {
                    Skippable::skip(s);
                }
            });
        self.stream
            .as_mut()
            .expect("is initialized")
            .mixer()
            .add(source);
    }

    fn stop(&mut self, note: Note) {
        if let Some(tx) = self.sources.get(&note) {
            tx.send(()).unwrap();
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use float_cmp::*;

    #[test]
    fn test_frequency() {
        assert!(approx_eq!(f64, frequency(0), 8.1758, epsilon = 0.00001));
        assert!(approx_eq!(f64, frequency(127), 12543.854, epsilon = 0.0001));
        assert!(approx_eq!(f64, frequency(69), 440.0, epsilon = 0.0001));
    }
}
