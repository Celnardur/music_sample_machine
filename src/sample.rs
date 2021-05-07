use std::error;
use std::f32::consts::PI;
//use crate::effect::Effect;
use hound;

const RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 32;

pub trait Sample {
    fn sample_rate(&self) -> u32;
    fn length(&self) -> usize;
    fn waveform(&self, channel: u16) -> Vec<f32>;
    fn channels(&self) -> u16;
    fn export(&self, file: &str) -> Result<(), Box<dyn error::Error>> {
        // store all the channels in a 2D vec
        let mut wave_data = Vec::new();
        for channel in 0..self.channels() {
            wave_data.push(self.waveform(channel))
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

    fn waveform(&self, _: u16) -> Vec<f32> {
        let mut waveform = Vec::new();
        for step in 0..self.length {
            let t = (step as f32) * 1.0/(RATE as f32);
            waveform.push(self.amplitude * (2.0 * PI * self.frequency * t).sin())
        }
        waveform
    }
}

//pub struct WaveForm {
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_440_to_wav() -> Result<(), Box<dyn error::Error>>{
        let wave = SineWave::new(440.0, (RATE * 5) as usize, 0.5);
        wave.export("./test_files/output/sine.wav")?;
        Ok(())
    }
}
