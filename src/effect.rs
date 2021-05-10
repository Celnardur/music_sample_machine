use crate::sample::Sample;
use crate::sample;

use std::error;

pub trait Effect {
    fn apply(&self, sample: &dyn Sample) -> Result<Box<dyn Sample>, Box<dyn error::Error>>;
}

pub trait WaveformEffect {
    fn process(&self, waveform: &[f32]) -> Result<Box<dyn Sample>, Box<dyn error::Error>>;
}

impl Effect for dyn WaveformEffect {
    fn apply(&self, sample: &dyn Sample) -> Result<Box<dyn Sample>, Box<dyn error::Error>> {
        let mut channels = sample::MultiChannel::new();
        for channel in 0..sample.channels() {
            let wave = self.process(&sample.waveform(channel).unwrap())?;
            channels.add_channel(&*wave)?;
        }
        Ok(Box::new(channels))
    }
}

pub struct LinearFadeEcho {
    pub delay: usize,
    pub fade_slope: f32,
}

impl LinearFadeEcho {
    pub fn new(delay: usize, fade_slope: f32) -> Self {
        LinearFadeEcho {
            delay,
            fade_slope,
        }
    }
}

impl Effect for LinearFadeEcho {
    fn apply(&self, sample: &dyn Sample) -> Result<Box<dyn Sample>, Box<dyn error::Error>> {
        let mut echo = sample::Composition::new();
        let mut fade = 1.0 - self.fade_slope;
        let mut delay = self.delay;
        while fade > 0.0 {
            echo.add_track(&*sample.scale(fade)?, delay)?;
            fade -= self.fade_slope;
            delay += self.delay;
        }
        Ok(Box::new(echo))
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn echo() -> Result<(), Box<dyn error::Error>> {
        let wave = sample::SineWave::new(440.0, (44100.0 * 0.25) as usize, 0.6);
        let echo = LinearFadeEcho{
            delay: (44100.0 * 0.5) as usize,
            fade_slope: 0.2,
        };
        let wave = wave.apply(&echo)?;
        wave.export("./test_files/output/echo.wav")?;
        Ok(())
    }
}
