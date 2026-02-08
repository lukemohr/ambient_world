use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::Arc;
use tracing::info;

use crate::layers::DroneLayer;
use crate::layers::Layer;
use crate::layers::SparkleLayer;
use crate::layers::TextureLayer;
use crate::params::SharedAudioParams;

/// Audio engine that manages CPAL stream.
#[allow(unused)]
pub struct AudioEngine {
    _stream: Stream, // Keep stream alive
    config: StreamConfig,
    layers: Vec<Arc<std::sync::Mutex<dyn Layer + Send>>>,
}

impl AudioEngine {
    pub fn start(shared_params: Arc<SharedAudioParams>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output device"))?;

        // Select a config with f32 sample format
        let supported_config = device
            .supported_output_configs()?
            .find(|config| config.sample_format() == SampleFormat::F32)
            .ok_or_else(|| anyhow::anyhow!("No f32 output config found"))?;
        let config = supported_config.with_max_sample_rate().config();

        let sample_rate_hz = config.sample_rate as u32;

        info!(
            "Selected device: {}, config: {} Hz, {} channels",
            device.description()?,
            sample_rate_hz,
            config.channels
        );

        let sample_rate = sample_rate_hz as f32;
        let drone_layer: Arc<std::sync::Mutex<dyn Layer + Send>> =
            Arc::new(std::sync::Mutex::new(DroneLayer::new(sample_rate)));
        let sparkle_layer: Arc<std::sync::Mutex<dyn Layer + Send>> =
            Arc::new(std::sync::Mutex::new(SparkleLayer::new(sample_rate)));
        let texture_layer: Arc<std::sync::Mutex<dyn Layer + Send>> =
            Arc::new(std::sync::Mutex::new(TextureLayer::new(sample_rate)));
        let layers = vec![drone_layer, texture_layer, sparkle_layer];
        let layers_clone: Vec<_> = layers.iter().map(Arc::clone).collect();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                Self::process_audio(data, &layers_clone, &shared_params, config.channels);
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
            config,
            layers,
        })
    }

    fn process_audio(
        output: &mut [f32],
        layers: &[Arc<std::sync::Mutex<dyn Layer + Send>>],
        shared_params: &Arc<SharedAudioParams>,
        channels: u16,
    ) {
        // Read latest params (non-blocking, atomic)
        let params = shared_params.get();

        // Conservative per-layer gains to prevent clipping
        // These are tuned so that max combined output is around 0.8 before master gain
        const DRONE_LAYER_GAIN: f32 = 0.3; // Drone is loud, keep it moderate
        const TEXTURE_LAYER_GAIN: f32 = 0.4; // Texture needs to be audible but not overpowering
        const SPARKLE_LAYER_GAIN: f32 = 0.6; // Sparkles: balanced gain for audibility without crackling

        let mut sample_index = 0;
        while sample_index < output.len() {
            // Mix samples from all layers with individual gains
            let mut mixed_sample = 0.0;

            // Process each layer with its specific gain
            for (i, layer) in layers.iter().enumerate() {
                let mut layer = layer.lock().unwrap();
                let layer_sample = layer.process(&params);

                // Ensure layer output is finite
                if layer_sample.is_finite() {
                    let layer_gain = match i {
                        0 => DRONE_LAYER_GAIN,   // Drone layer
                        1 => TEXTURE_LAYER_GAIN, // Texture layer
                        2 => SPARKLE_LAYER_GAIN, // Sparkle layer
                        _ => 0.1,                // Default conservative gain
                    };
                    mixed_sample += layer_sample * layer_gain;
                }
            }

            // Apply master gain with cap to prevent excessive amplification
            let master_gain = params.master_gain.min(1.0); // Cap master gain at 1.0
            mixed_sample *= master_gain;

            // Soft limiter: more aggressive than tanh for better headroom
            // This provides about 6dB of limiting with smooth knee
            if mixed_sample.abs() > 0.8 {
                // Soft knee compression above 0.8
                let excess = mixed_sample.abs() - 0.8;
                let compressed = excess * 0.5; // 2:1 ratio
                mixed_sample = mixed_sample.signum() * (0.8 + compressed);
            }

            // Final hard clip at 1.0 as safety net (should rarely engage with above limiting)
            mixed_sample = mixed_sample.clamp(-1.0, 1.0);

            for _ in 0..channels {
                if sample_index < output.len() {
                    output[sample_index] = mixed_sample;
                    sample_index += 1;
                }
            }
        }
    }
}
