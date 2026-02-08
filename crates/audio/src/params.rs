use std::sync::atomic::{AtomicU32, Ordering};

/// Audio parameters that the callback uses.
/// Minimal, numeric only.
#[derive(Clone, Copy, Debug)]
pub struct AudioParams {
    pub master_gain: f32,
    pub base_freq_hz: f32,
    pub detune_ratio: f32,
    pub brightness: f32,
    pub motion: f32,
    pub texture: f32,
    pub sparkle_impulse: f32,
}

impl Default for AudioParams {
    fn default() -> Self {
        Self {
            master_gain: 0.1,
            base_freq_hz: 440.0,
            detune_ratio: 1.0,
            brightness: 0.5,
            motion: 0.0,
            texture: 0.0,
            sparkle_impulse: 0.0,
        }
    }
}

impl AudioParams {
    /// Derive from world state variables.
    pub fn from_world_state(
        density: f32,
        rhythm: f32,
        tension: f32,
        energy: f32,
        warmth: f32,
        sparkle_impulse: f32,
    ) -> Self {
        Self {
            master_gain: (energy * 0.2).clamp(0.0, 1.0), // energy -> gain, clamped
            base_freq_hz: (80.0 + warmth * 160.0).clamp(80.0, 240.0), // warmth -> freq range 80-240 Hz
            detune_ratio: (1.0 + tension * 0.01).clamp(0.5, 2.0), // tension -> slight detune, clamped
            brightness: (1.0 - warmth * 0.5).clamp(0.0, 1.0), // warmth inverse -> brightness, clamped
            motion: (rhythm * 0.5).clamp(0.0, 1.0),           // rhythm -> motion, clamped
            texture: (density * 0.3).clamp(0.0, 1.0),         // density -> texture, clamped
            sparkle_impulse,
        }
    }
}

/// Thread-safe shared audio parameters using atomics.
#[derive(Debug)]
pub struct SharedAudioParams {
    master_gain: AtomicU32,
    base_freq_hz: AtomicU32,
    detune_ratio: AtomicU32,
    brightness: AtomicU32,
    motion: AtomicU32,
    texture: AtomicU32,
    sparkle_impulse: AtomicU32,
}

impl SharedAudioParams {
    pub fn new(initial: AudioParams) -> Self {
        Self {
            master_gain: AtomicU32::new(initial.master_gain.to_bits()),
            base_freq_hz: AtomicU32::new(initial.base_freq_hz.to_bits()),
            detune_ratio: AtomicU32::new(initial.detune_ratio.to_bits()),
            brightness: AtomicU32::new(initial.brightness.to_bits()),
            motion: AtomicU32::new(initial.motion.to_bits()),
            texture: AtomicU32::new(initial.texture.to_bits()),
            sparkle_impulse: AtomicU32::new(initial.sparkle_impulse.to_bits()),
        }
    }

    pub fn set(&self, params: AudioParams) {
        self.master_gain
            .store(params.master_gain.to_bits(), Ordering::Relaxed);
        self.base_freq_hz
            .store(params.base_freq_hz.to_bits(), Ordering::Relaxed);
        self.detune_ratio
            .store(params.detune_ratio.to_bits(), Ordering::Relaxed);
        self.brightness
            .store(params.brightness.to_bits(), Ordering::Relaxed);
        self.motion
            .store(params.motion.to_bits(), Ordering::Relaxed);
        self.texture
            .store(params.texture.to_bits(), Ordering::Relaxed);
        self.sparkle_impulse
            .store(params.sparkle_impulse.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> AudioParams {
        AudioParams {
            master_gain: f32::from_bits(self.master_gain.load(Ordering::Relaxed)),
            base_freq_hz: f32::from_bits(self.base_freq_hz.load(Ordering::Relaxed)),
            detune_ratio: f32::from_bits(self.detune_ratio.load(Ordering::Relaxed)),
            brightness: f32::from_bits(self.brightness.load(Ordering::Relaxed)),
            motion: f32::from_bits(self.motion.load(Ordering::Relaxed)),
            texture: f32::from_bits(self.texture.load(Ordering::Relaxed)),
            sparkle_impulse: f32::from_bits(self.sparkle_impulse.load(Ordering::Relaxed)),
        }
    }
}
