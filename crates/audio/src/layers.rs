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
    smoothed_brightness: f32,
    smoothed_motion: f32,
    smoothed_texture: f32,
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
            smoothed_brightness: 0.0,
            smoothed_motion: 0.0,
            smoothed_texture: 0.0,
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
        Self::smooth(
            &mut self.smoothed_brightness,
            params.brightness,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_motion,
            params.motion,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_texture,
            params.texture,
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
        let mixed_sample = (sample_a + sample_b) * 0.5;

        // Update phases
        self.phase_a += 1.0;
        if self.phase_a >= self.sample_rate / freq_a {
            self.phase_a -= self.sample_rate / freq_a;
        }

        mixed_sample
    }
}

/// Sparkle layer that generates short, bright impulses when sparkle_impulse > 0.
/// Sparkles are influenced by tension (detune_ratio) and rhythm (motion).
#[allow(unused)]
pub struct SparkleLayer {
    envelope_phase: f32, // 0.0 to 1.0, where 1.0 means envelope complete
    envelope_duration_samples: f32,
    sample_rate: f32,
    noise_seed: f32,
    smoothed_sparkle_impulse: f32,
    prev_smoothed_impulse: f32,
    smoothing_coeff: f32,
    // Smoothed parameters for musical influence
    smoothed_tension: f32,
    smoothed_motion: f32,
    smoothed_brightness: f32,
}

impl SparkleLayer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            envelope_phase: 1.0, // Start with envelope complete (no sound)
            envelope_duration_samples: sample_rate * 0.1, // 100ms envelope
            sample_rate,
            noise_seed: 0.0,
            smoothed_sparkle_impulse: 0.0,
            prev_smoothed_impulse: 0.0,
            smoothing_coeff: 0.2, // Very fast smoothing for sparkles to catch quick impulses
            smoothed_tension: 0.0,
            smoothed_motion: 0.0,
            smoothed_brightness: 0.0,
        }
    }

    // Simple attack/decay envelope: influenced by tension
    // Higher tension = more percussive (faster attack, shorter decay)
    // Uses smooth curves to prevent clicks
    fn envelope(&self, phase: f32, tension: f32) -> f32 {
        // Tension affects envelope shape: 0.0 = smooth/sustained, 1.0 = sharp/percussive
        let attack_portion = 0.15 + tension * 0.25; // Attack: 15-40% of envelope (longer for smoother attack)
        let decay_start = attack_portion;

        if phase < attack_portion {
            // Attack: smooth curve (sine-like) to prevent clicks
            let attack_phase = phase / attack_portion;
            (attack_phase * std::f32::consts::PI * 0.5).sin()
        } else {
            // Decay: smooth exponential decay
            let decay_phase = (phase - decay_start) / (1.0 - decay_start);
            // Exponential decay: e^(-decay_phase * 3) for smooth tail
            (-decay_phase * 3.0).exp().max(0.0)
        }
    }

    // Generate white noise sample
    fn noise(&mut self) -> f32 {
        // Simple LCG noise generator
        self.noise_seed = (self.noise_seed * 1103515245.0 + 12345.0) % (1 << 31) as f32;
        let sample = (self.noise_seed / (1 << 31) as f32) * 2.0 - 1.0;
        // Ensure no NaN or inf
        if sample.is_finite() { sample } else { 0.0 }
    }

    // Generate filtered noise burst influenced by brightness and motion
    fn filtered_noise_burst(&mut self, envelope_value: f32, brightness: f32, motion: f32) -> f32 {
        let base_noise = self.noise();

        // Motion affects the energy/pitch of the noise (higher motion = brighter/higher frequency)
        let motion_factor = 1.0 + motion * 1.5; // 1.0 to 2.5 (reduced from 3.0)

        // Brightness affects filtering (higher brightness = less filtering/more high frequencies)
        // Simplified filtering to reduce artifacts
        let brightness_factor = 0.7 + brightness * 0.6; // 0.7 to 1.3 (more conservative)
        let filtered_noise = base_noise * brightness_factor;

        // Apply gentle motion-based amplitude modulation (creates rhythmic feel)
        // Use smooth triangle wave instead of sin to avoid discontinuities
        let motion_mod = 1.0 + (motion * envelope_value * 2.0 - 1.0) * 0.2; // ±20% modulation

        // Apply envelope first, then modulation for smoother transients
        let enveloped = filtered_noise * envelope_value;
        enveloped * motion_mod * motion_factor
    }
}

impl Layer for SparkleLayer {
    fn process(&mut self, params: &AudioParams) -> f32 {
        // Smooth all parameters
        self.prev_smoothed_impulse = self.smoothed_sparkle_impulse;
        self.smoothed_sparkle_impulse +=
            (params.sparkle_impulse - self.smoothed_sparkle_impulse) * self.smoothing_coeff;

        // Smooth musical parameters
        self.smoothed_tension +=
            (params.detune_ratio - self.smoothed_tension) * self.smoothing_coeff;
        self.smoothed_motion += (params.motion - self.smoothed_motion) * self.smoothing_coeff;
        self.smoothed_brightness +=
            (params.brightness - self.smoothed_brightness) * self.smoothing_coeff;

        // Update envelope duration based on motion (higher motion = shorter, more rhythmic events)
        self.envelope_duration_samples = self.sample_rate * (0.05 + self.smoothed_motion * 0.15); // 50-200ms

        // Trigger new envelope when smoothed impulse crosses threshold and we can start a new one
        if self.smoothed_sparkle_impulse > 0.002
            && self.envelope_phase >= 1.0
            && self.prev_smoothed_impulse <= 0.002
        {
            self.envelope_phase = 0.0; // Start new envelope
        }

        // If envelope is active, generate sparkle sound
        if self.envelope_phase < 1.0 {
            let envelope_value = self.envelope(self.envelope_phase, self.smoothed_tension);

            // Generate filtered noise burst influenced by brightness and motion
            let sparkle_sample = self.filtered_noise_burst(
                envelope_value,
                self.smoothed_brightness,
                self.smoothed_motion,
            );

            // Use smoothed impulse for overall amplitude
            let final_sample = sparkle_sample * self.smoothed_sparkle_impulse;

            // Update envelope phase
            self.envelope_phase += 1.0 / self.envelope_duration_samples;

            // Ensure output is finite and apply gentle limiting
            if final_sample.is_finite() {
                // Soft limit sparkle peaks to prevent crackling
                final_sample.clamp(-0.8, 0.8)
            } else {
                0.0
            }
        } else {
            0.0 // No sound when envelope is complete
        }
    }
}

/// Texture layer that provides a subtle noise bed with slow modulation.
pub struct TextureLayer {
    noise_seed: f32,
    lfo_phase: f32,
    smoothed_density: f32,
    smoothed_warmth: f32,
    smoothed_tension: f32,
    smoothed_energy: f32,
    // Simple low-pass filter state
    filter_x1: f32,
    filter_y1: f32,
    sample_rate: f32,
    smoothing_coeff: f32,
}

impl TextureLayer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            noise_seed: 0.0,
            lfo_phase: 0.0,
            smoothed_density: 0.0,
            smoothed_warmth: 0.0,
            smoothed_tension: 0.0,
            smoothed_energy: 0.0,
            filter_x1: 0.0,
            filter_y1: 0.0,
            sample_rate,
            smoothing_coeff: 0.005, // Very slow smoothing for texture
        }
    }

    fn smooth(current: &mut f32, target: f32, coeff: f32) {
        *current += (target - *current) * coeff;
    }

    // Generate noise with tension-based roughness
    fn noise(&mut self, tension: f32) -> f32 {
        // Base LCG noise
        self.noise_seed = (self.noise_seed * 1103515245.0 + 12345.0) % (1 << 31) as f32;
        let base_noise = (self.noise_seed / (1 << 31) as f32) * 2.0 - 1.0;

        // Add roughness based on tension (slight distortion)
        let roughness = tension * 0.1;
        let rough_noise = base_noise + roughness * base_noise.powi(3);

        // Ensure finite
        if rough_noise.is_finite() {
            rough_noise
        } else {
            base_noise
        }
    }

    // Simple low-pass filter for warmth control
    fn filter(&mut self, input: f32, cutoff: f32) -> f32 {
        // Bilinear transform approximation of low-pass filter
        // cutoff is normalized (0.0 = no filtering, 1.0 = heavy filtering)
        let a = cutoff.clamp(0.001, 0.99);
        let b = 1.0 - a;

        let output = a * input + b * self.filter_x1 - b * self.filter_y1;

        // Update filter state
        self.filter_x1 = input;
        self.filter_y1 = output;

        output
    }

    // Slow LFO for amplitude modulation
    fn lfo(&mut self, energy: f32) -> f32 {
        // LFO frequency based on energy (very slow: 0.01 to 0.1 Hz)
        let lfo_freq = 0.01 + energy * 0.09;
        let lfo_inc = lfo_freq * 2.0 * std::f32::consts::PI / self.sample_rate;

        self.lfo_phase += lfo_inc;
        if self.lfo_phase >= 2.0 * std::f32::consts::PI {
            self.lfo_phase -= 2.0 * std::f32::consts::PI;
        }

        // Triangle wave LFO for smooth modulation
        let triangle = if self.lfo_phase < std::f32::consts::PI {
            self.lfo_phase / std::f32::consts::PI
        } else {
            2.0 - self.lfo_phase / std::f32::consts::PI
        };

        // Convert to bipolar (-1 to 1) and scale
        (triangle - 0.5) * 2.0
    }
}

impl Layer for TextureLayer {
    fn process(&mut self, params: &AudioParams) -> f32 {
        // Smooth parameters
        Self::smooth(
            &mut self.smoothed_density,
            params.texture,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_warmth,
            params.brightness,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_tension,
            params.detune_ratio,
            self.smoothing_coeff,
        );
        Self::smooth(
            &mut self.smoothed_energy,
            params.motion,
            self.smoothing_coeff,
        );

        // Generate base noise with tension-based roughness
        let noise = self.noise(self.smoothed_tension);

        // Apply filtering based on warmth (0.0 = bright, 1.0 = warm/dark)
        let filtered = self.filter(noise, self.smoothed_warmth);

        // Apply LFO modulation based on energy
        let lfo = self.lfo(self.smoothed_energy);
        let modulated = filtered * (1.0 + lfo * 0.3); // ±30% modulation

        // Scale by density and apply subtle gain
        let texture_sample = modulated * self.smoothed_density * 0.1;

        // Ensure finite output
        if texture_sample.is_finite() {
            texture_sample
        } else {
            0.0
        }
    }
}
