use crate::sample::Sample;

pub trait Effect {
    fn apply(&self, sample: &dyn Sample) -> Box<dyn Sample>;
}
