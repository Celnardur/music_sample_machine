use std::error;
use crate::effect::Effect;

pub trait Sample {
    fn sample_rate(&self) -> u32;
    fn length(&self) -> usize;
    fn waveform(&self, channel: u32) -> Vec<f32>;
    fn channels(&self) -> u32;
    fn export(&self, file: &str) -> Result<(), Box<dyn error::Error>>;
    fn apply_effect(&self, effect: &dyn Effect) -> Box<dyn Sample>;
}
