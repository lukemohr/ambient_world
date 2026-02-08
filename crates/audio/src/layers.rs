use crate::params::AudioParams;

/// Trait for audio layers that generate samples.
pub trait Layer {
    fn process(&mut self, params: &AudioParams) -> f32;
}

/// Drone layer that generates a continuous tone.
pub struct DroneLayer {
    phase: f32,
    smoothed_master_gain: f32,
    smoothed_base_freq_hz: f32,
    sample_rate: f32,
    smoothing_coeff: f32,
}

impl DroneLayer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            smoothed_master_gain: 0.0,
            smoothed_base_freq_hz: 440.0,
            sample_rate,
            smoothing_coeff: 0.01, // Adjust for smoothing speed
        }
    }

    fn smooth(current: &mut f32, target: f32, coeff: f32) {
        *current += (target - *current) * coeff;
    }
}

impl Layer for DroneLayer {
    fn process(&mut self, params: &AudioParams) -> f32 {
        // Smooth parameters
        Self::smooth(
            &mut self.smoothed_master_gain,
            params.master_gain,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_base_freq_hz,
            params.base_freq_hz,
            self.smoothing_coeff,
        );

        // Generate sample
        let sample = (self.phase * self.smoothed_base_freq_hz * 2.0 * std::f32::consts::PI
            / self.sample_rate)
            .sin()
            * self.smoothed_master_gain;

        // Update phase
        self.phase += 1.0;
        if self.phase >= self.sample_rate / self.smoothed_base_freq_hz {
            self.phase -= self.sample_rate / self.smoothed_base_freq_hz;
        }

        sample
    }
}
