// what will hopefully be an FM synthesis engine in Rust

use midir::{Ignore, MidiInput, MidiInputConnection};
use rodio::{
    OutputStream,
    source::{SineWave, Source},
};
use std::error::Error;
use std::{fmt, time::Duration};

/// Midi notes, 0 = C-1 and 127 = G9
type Note = u8;

/// Convert MIDI note number to center frequency (Hz).
/// https://en.wikipedia.org/wiki/MIDI_tuning_standard
pub fn frequency(note: Note) -> f64 {
    440.0 * ((note as i16 - 69) as f64 / 12.0).exp2()
}

#[derive(Default)]
pub struct Synth {
    stream: Option<OutputStream>,
    connection: Option<MidiInputConnection<()>>,
}

impl fmt::Debug for Synth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<A synthesizer")
    }
}

impl Synth {
    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        // audio stream
        self.stream =
            Some(rodio::OutputStreamBuilder::open_default_stream().expect("open default stream"));
        // let sink = rodio::Sink::connect_new(&self.stream.mixer());

        // midi input port
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

        self.connection = Some(midi_in.connect(
            in_port,
            "midir-read-input",
            move |stamp, message, _| {
                println!("got {}: {:?} (len={})", stamp, message, message.len());
            },
            (),
        )?);

        Ok(())
    }

    pub fn play(&mut self, note: Note) {
        let source = SineWave::new(frequency(note) as f32)
            .take_duration(Duration::from_secs_f32(0.25))
            .amplify(0.20);
        self.stream
            .as_mut()
            .expect("is initialized")
            .mixer()
            .add(source);
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
