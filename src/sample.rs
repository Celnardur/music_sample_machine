use std::error;
use crate::Error;
use std::f32::consts::PI;
//use crate::effect::Effect;
use hound;

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
                .ok_or(Error::new_box("Sample is missing a channel"))?
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
        if channel > 0 { return None }

        let mut waveform = Vec::new();
        for step in 0..self.length {
            let t = (step as f32) * 1.0/(RATE as f32);
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
        if channel > 0 { return None }

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
    pub fn new_dual(left: &dyn Sample, right: &dyn Sample) -> Result<MultiChannel, Error> {
        if left.length() != right.length() {
            return Err(Error::new("Left and right sample lengths do not match"))
        }
        if left.sample_rate() != right.sample_rate() {
            return Err(Error::new("Left and right sample rates do not match"))
        }
        if left.channels() != 1 {
            return Err(Error::new("Left channel has more than one channel"))
        }
        if right.channels() != 1 {
            return Err(Error::new("Right channel has more than one channel"))
        }

        Ok(MultiChannel {
            sample_rate: left.sample_rate(), 
            length: left.length(),
            channels: vec![left.box_clone(), right.box_clone()],
        })
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
            channels
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_440_to_wav() -> Result<(), Box<dyn error::Error>>{
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
}
