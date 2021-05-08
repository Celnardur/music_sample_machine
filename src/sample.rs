use crate::Error;
//use crate::effect::Effect;

use std::error;
use std::f32::consts::PI;
use std::fs::File;
use std::iter;

use hound;
use minimp3;

const RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 32;

pub trait Sample {
    fn sample_rate(&self) -> u32;
    fn length(&self) -> usize;
    fn waveform(&self, channel: u16) -> Option<Vec<f32>>;
    fn channels(&self) -> u16;
    fn box_clone(&self) -> Box<dyn Sample>; // nesscarry for cloning
    fn export(&self, file: &str) -> Result<(), Box<dyn error::Error>> {
        // store all the channels in a 2D vec
        let mut wave_data = Vec::new();
        for channel in 0..self.channels() {
            wave_data.push(
                self.waveform(channel)
                    .ok_or(Error::new_box("Sample is missing a channel"))?,
            );
        }

        // set up hound
        let spec = hound::WavSpec {
            channels: self.channels(),
            sample_rate: self.sample_rate(),
            bits_per_sample: BITS_PER_SAMPLE,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(file, spec)?;

        // interleave channel data
        for index in 0..self.length() {
            for channel in 0..self.channels() {
                writer.write_sample(wave_data[channel as usize][index])?
            }
        }
        writer.finalize()?;
        Ok(())
    }
    fn sample(&self, start: usize, end: usize) -> Box<dyn Sample> {
        let mut sample = MultiChannel::new();
        for channel in 0..self.channels() {
            sample
                .add_channel(&WaveForm::from(
                    &self.waveform(channel).unwrap()[start..end],
                ))
                .unwrap();
        }
        Box::new(sample)
    }
    fn sample_sec(&self, start: f32, end: f32) -> Box<dyn Sample> {
        let start = (start * (self.sample_rate() as f32)) as usize;
        let end = (end * (self.sample_rate() as f32)) as usize;
        self.sample(start, end)
    }
    //fn apply(&self, effect: &dyn Effect) -> Box<dyn Sample>;
}

#[derive(Clone)]
pub struct SineWave {
    frequency: f32,
    amplitude: f32,
    sample_rate: u32,
    length: usize,
}

impl SineWave {
    pub fn new(frequency: f32, length: usize, amplitude: f32) -> Self {
        SineWave {
            frequency,
            length,
            amplitude,
            sample_rate: RATE,
        }
    }
}

impl Sample for SineWave {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn length(&self) -> usize {
        self.length
    }

    fn channels(&self) -> u16 {
        1
    }

    fn waveform(&self, channel: u16) -> Option<Vec<f32>> {
        if channel > 0 {
            return None;
        }

        let mut waveform = Vec::new();
        for step in 0..self.length {
            let t = (step as f32) * 1.0 / (self.sample_rate() as f32);
            waveform.push(self.amplitude * (2.0 * PI * self.frequency * t).sin())
        }
        Some(waveform)
    }

    fn box_clone(&self) -> Box<dyn Sample> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct WaveForm {
    sample_rate: u32,
    waveform: Vec<f32>,
}

impl WaveForm {
    pub fn from(waveform: &[f32]) -> WaveForm {
        WaveForm {
            sample_rate: RATE,
            waveform: waveform.to_vec(),
        }
    }
}

impl Sample for WaveForm {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn length(&self) -> usize {
        self.waveform.len()
    }

    fn channels(&self) -> u16 {
        1
    }

    fn waveform(&self, channel: u16) -> Option<Vec<f32>> {
        if channel > 0 {
            return None;
        }

        Some(self.waveform.clone())
    }

    fn box_clone(&self) -> Box<dyn Sample> {
        Box::new(self.clone())
    }
}

pub struct MultiChannel {
    sample_rate: u32,
    length: usize,
    channels: Vec<Box<dyn Sample>>,
}

impl MultiChannel {
    pub fn new() -> MultiChannel {
        MultiChannel {
            sample_rate: 0,
            length: 0,
            channels: Vec::new(),
        }
    }

    pub fn new_dual(left: &dyn Sample, right: &dyn Sample) -> Result<MultiChannel, Error> {
        if left.length() != right.length() {
            return Err(Error::new("Left and right sample lengths do not match"));
        }
        if left.sample_rate() != right.sample_rate() {
            return Err(Error::new("Left and right sample rates do not match"));
        }
        if left.channels() != 1 {
            return Err(Error::new("Left channel has more than one channel"));
        }
        if right.channels() != 1 {
            return Err(Error::new("Right channel has more than one channel"));
        }

        Ok(MultiChannel {
            sample_rate: left.sample_rate(),
            length: left.length(),
            channels: vec![left.box_clone(), right.box_clone()],
        })
    }

    pub fn from_mp3(filename: &str) -> Result<MultiChannel, Box<dyn error::Error>> {
        let mut waveforms: Vec<Vec<f32>> = Vec::new();
        let mut decoder = minimp3::Decoder::new(File::open(filename)?);
        let mut rate = 0;
        loop {
            match decoder.next_frame() {
                Ok(minimp3::Frame {
                    data,
                    sample_rate,
                    channels,
                    ..
                }) => {
                    if rate != 0 && sample_rate != rate {
                        return Err(Error::new_box("Sample rate changed in file"));
                    }
                    rate = sample_rate;

                    if waveforms.is_empty() {
                        waveforms = iter::repeat(Vec::new()).take(channels).collect();
                    }
                    if waveforms.len() != channels {
                        return Err(Error::new_box("Number of waveforms changed mid song"));
                    }

                    for (index, sample) in data.iter().enumerate() {
                        let sample = (*sample as f32) / (i16::MAX as f32);
                        let channel = index % waveforms.len();
                        waveforms[channel].push(sample);
                    }
                }
                Err(minimp3::Error::Eof) => break,
                Err(e) => return Err(Box::new(e)),
            }
        }

        let mut channels: Vec<Box<dyn Sample>> = Vec::new();
        for wave in waveforms {
            channels.push(Box::new(WaveForm::from(&wave)));
        }

        Ok(MultiChannel {
            sample_rate: rate as u32,
            length: channels[0].length(),
            channels,
        })
    }

    pub fn from_wav(filename: &str) -> Result<MultiChannel, Box<dyn error::Error>> {
        let mut reader = hound::WavReader::open(filename)?;
        let length = reader.duration() as usize;
        let sample_rate = reader.spec().sample_rate;
        let channels = reader.spec().channels as usize;
        let mut waveforms: Vec<Vec<f32>> = iter::repeat(Vec::new()).take(channels).collect();
        for (index, sample) in reader.samples::<f32>().enumerate() {
            waveforms[index % channels].push(sample?);
        }

        let mut channels: Vec<Box<dyn Sample>> = Vec::new();
        for wave in waveforms {
            channels.push(Box::new(WaveForm::from(&wave)));
        }

        Ok(MultiChannel {
            sample_rate,
            length,
            channels,
        })
    }

    pub fn add_channel(&mut self, track: &dyn Sample) -> Result<(), Error> {
        if track.channels() > 1 {
            return Err(Error::new(
                "Can only add single channel tracks to a multi-channel",
            ));
        }
        if self.channels.len() == 0 {
            self.sample_rate = track.sample_rate();
            self.length = track.length();
        } else {
            if self.sample_rate != track.sample_rate() {
                return Err(Error::new("Channels must have same sample rate"));
            }
            if self.length != track.length() {
                return Err(Error::new("Channels must have same length"));
            }
        }
        self.channels.push(track.box_clone());
        Ok(())
    }
}

impl Sample for MultiChannel {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn length(&self) -> usize {
        self.length
    }

    fn channels(&self) -> u16 {
        self.channels.len() as u16
    }

    fn waveform(&self, channel: u16) -> Option<Vec<f32>> {
        match self.channels.get(channel as usize) {
            Some(sample) => sample.waveform(0),
            None => None,
        }
    }

    fn box_clone(&self) -> Box<dyn Sample> {
        let mut channels = Vec::new();
        for channel in &self.channels {
            channels.push(channel.box_clone())
        }

        Box::new(MultiChannel {
            sample_rate: self.sample_rate,
            length: self.length,
            channels,
        })
    }
}

pub struct Composition {
    sample_rate: u32,
    length: usize,
    channels: u16,
    tracks: Vec<Box<dyn Sample>>,
    starts: Vec<Vec<usize>>,
}

impl Composition {
    pub fn new() -> Composition {
        Composition {
            sample_rate: 0,
            length: 0,
            channels: 0,
            tracks: Vec::new(),
            starts: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: &dyn Sample, start: usize) -> Result<usize, Error> {
        if self.tracks.is_empty() {
            self.sample_rate = track.sample_rate();
            self.length = track.length() + start;
            self.channels = track.channels();
        } else {
            if self.sample_rate != track.sample_rate() {
                return Err(Error::new(
                    "Tracks of the same composition must have the same sample rate",
                ));
            }
            if self.channels != track.channels() {
                return Err(Error::new(
                    "Tracks of the same composition must have the same number of channels",
                ));
            }
            if track.length() + start > self.length {
                self.length = track.length() + start;
            }
        }
        let id = self.tracks.len();
        self.tracks.push(track.box_clone());
        self.starts.push(vec![start]);
        Ok(id)
    }

    pub fn add_track_sec(&mut self, track: &dyn Sample, start: f32) -> Result<usize, Error> {
        let start = (start * (self.sample_rate() as f32)) as usize;
        self.add_track(track, start)
    }

    pub fn add_track_id(&mut self, id: usize, start: usize) -> Result<(), Error> {
        if id >= self.tracks.len() {
            return Err(Error::new("That track does not exist"));
        }
        if self.tracks[id].length() + start > self.length {
            self.length = self.tracks[id].length() + start;
        }
        self.starts[id].push(start);
        Ok(())
    }

    pub fn add_track_id_sec(&mut self, id: usize, start: f32) -> Result<(), Error> {
        let start = (start * (self.sample_rate() as f32)) as usize;
        self.add_track_id(id, start)
    }
}

impl Sample for Composition {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn length(&self) -> usize {
        self.length
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn waveform(&self, channel: u16) -> Option<Vec<f32>> {
        if channel >= self.channels {
            return None;
        }

        let mut waveform: Vec<f32> = iter::repeat(0.0).take(self.length).collect();
        for (id, track) in self.tracks.iter().enumerate() {
            let track = track.waveform(channel).unwrap();
            for start in &self.starts[id] {
                for (t, val) in track.iter().enumerate() {
                    waveform[t + start] += val;
                }
            }
        }
        Some(waveform)
    }

    fn box_clone(&self) -> Box<dyn Sample> {
        let mut tracks = Vec::new();
        for track in &self.tracks {
            tracks.push(track.box_clone())
        }

        Box::new(Composition {
            sample_rate: self.sample_rate,
            length: self.length,
            channels: self.channels,
            tracks,
            starts: self.starts.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_440_to_wav() -> Result<(), Box<dyn error::Error>> {
        let wave = SineWave::new(440.0, (RATE * 5) as usize, 0.5);
        wave.export("./test_files/output/sine.wav")?;
        Ok(())
    }

    #[test]
    fn left_sine() -> Result<(), Box<dyn error::Error>> {
        let wave = SineWave::new(440.0, (RATE * 5) as usize, 0.5);
        let silence = SineWave::new(440.0, (RATE * 5) as usize, 0.0);
        let left = MultiChannel::new_dual(&wave, &silence)?;
        left.export("./test_files/output/left_sine.wav")?;
        Ok(())
    }

    #[test]
    fn right_sine() -> Result<(), Box<dyn error::Error>> {
        let wave = SineWave::new(440.0, (RATE * 5) as usize, 0.5);
        let silence = SineWave::new(440.0, (RATE * 5) as usize, 0.0);
        let right = MultiChannel::new_dual(&silence, &wave)?;
        right.export("./test_files/output/right_sine.wav")?;
        Ok(())
    }

    #[test]
    fn switch_lr_sine() -> Result<(), Box<dyn error::Error>> {
        let wave = SineWave::new(440.0, (RATE * 1) as usize, 0.5);
        let silence = SineWave::new(440.0, (RATE * 1) as usize, 0.0);
        let left = MultiChannel::new_dual(&wave, &silence)?;
        let right = MultiChannel::new_dual(&silence, &wave)?;
        let mut comp = Composition::new();
        let left = comp.add_track(&left, 0)?;
        let right = comp.add_track_sec(&right, 1.0)?;
        comp.add_track_id_sec(left, 2.0)?;
        comp.add_track_id_sec(right, 3.0)?;
        comp.add_track_id_sec(left, 4.0)?;

        comp.export("./test_files/output/switch_lr_sine.wav")?;
        Ok(())
    }

    #[test]
    fn from_mp3() -> Result<(), Box<dyn error::Error>> {
        let song = MultiChannel::from_mp3("./test_files/songs/Chameleon_short.mp3")?;
        song.export("./test_files/output/from_mp3.wav")?;
        Ok(())
    }

    #[test]
    fn from_wav() -> Result<(), Box<dyn error::Error>> {
        let song = MultiChannel::from_wav("./test_files/songs/switch_lr_sine.wav")?;
        song.export("./test_files/output/from_wav.wav")?;
        Ok(())
    }

    #[test]
    fn pick_sample() -> Result<(), Box<dyn error::Error>> {
        let song =
            MultiChannel::from_mp3("./test_files/songs/Chameleon_short.mp3")?.sample_sec(10.0, 15.0);
        song.export("./test_files/output/sample.wav")?;
        Ok(())
    }
}
