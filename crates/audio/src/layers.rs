use crate::params::AudioParams;

/// Trait for audio layers that generate samples.
pub trait Layer {
    fn process(&mut self, params: &AudioParams) -> f32;
}

/// Drone layer that generates a continuous tone with two oscillators for richness.
pub struct DroneLayer {
    phase_a: f32,
    phase_b: f32,
    smoothed_master_gain: f32,
    smoothed_base_freq_hz: f32,
    smoothed_detune_ratio: f32,
    sample_rate: f32,
    smoothing_coeff: f32,
}

impl DroneLayer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase_a: 0.0,
            phase_b: 0.0,
            smoothed_master_gain: 0.0,
            smoothed_base_freq_hz: 440.0,
            smoothed_detune_ratio: 1.0,
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
        Self::smooth(
            &mut self.smoothed_detune_ratio,
            params.detune_ratio,
            self.smoothing_coeff,
        );

        // Generate samples from two oscillators
        let freq_a = self.smoothed_base_freq_hz;
        let freq_b = self.smoothed_base_freq_hz * self.smoothed_detune_ratio;

        let sample_a =
            (self.phase_a * freq_a * 2.0 * std::f32::consts::PI / self.sample_rate).sin();
        let sample_b =
            (self.phase_b * freq_b * 2.0 * std::f32::consts::PI / self.sample_rate).sin();

        // Mix the two oscillators (equal volume)
        let mixed_sample = (sample_a + sample_b) * 0.5 * self.smoothed_master_gain;

        // Update phases
        self.phase_a += 1.0;
        if self.phase_a >= self.sample_rate / freq_a {
            self.phase_a -= self.sample_rate / freq_a;
        }

        self.phase_b += 1.0;
        if self.phase_b >= self.sample_rate / freq_b {
            self.phase_b -= self.sample_rate / freq_b;
        }

        mixed_sample
    }
}
