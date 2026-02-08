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
/// Layers are owned by the callback closure to avoid locking.
#[allow(unused)]
pub struct AudioEngine {
    _stream: Stream, // Keep stream alive
    config: StreamConfig,
}

impl AudioEngine {
    pub fn start(shared_params: Arc<SharedAudioParams>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output device"))?;

        // Get the default output config (whatever format it supports)
        let mut supported_configs = device.supported_output_configs()?;
        let supported_config = supported_configs
            .next()
            .ok_or_else(|| anyhow::anyhow!("No supported output configs found"))?;
        let sample_format = supported_config.sample_format();
        let config = supported_config.with_max_sample_rate().config();

        let sample_rate_hz = config.sample_rate as u32;

        info!(
            "Selected device: {}, config: {} Hz, {} channels, format: {:?}",
            device.description()?,
            sample_rate_hz,
            config.channels,
            sample_format
        );

        let sample_rate = sample_rate_hz as f32;

        // Create layers directly (no Mutex needed since callback owns them)
        let drone_layer = Box::new(DroneLayer::new(sample_rate)) as Box<dyn Layer>;
        let sparkle_layer = Box::new(SparkleLayer::new(sample_rate)) as Box<dyn Layer>;
        let texture_layer = Box::new(TextureLayer::new(sample_rate)) as Box<dyn Layer>;
        let mut layers = vec![drone_layer, texture_layer, sparkle_layer];

        // Build stream based on sample format
        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        Self::process_audio_f32(data, &mut layers, &shared_params, config.channels);
                    },
                    |err| eprintln!("Stream error: {}", err),
                    None,
                )?
            }
            SampleFormat::I16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        Self::process_audio_i16(data, &mut layers, &shared_params, config.channels);
                    },
                    |err| eprintln!("Stream error: {}", err),
                    None,
                )?
            }
            SampleFormat::U16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        Self::process_audio_u16(data, &mut layers, &shared_params, config.channels);
                    },
                    |err| eprintln!("Stream error: {}", err),
                    None,
                )?
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported sample format: {:?}", sample_format));
            }
        };

        stream.play()?;

        Ok(Self {
            _stream: stream,
            config,
        })
    }

    fn process_audio_f32(
        output: &mut [f32],
        layers: &mut [Box<dyn Layer>],
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
            for (i, layer) in layers.iter_mut().enumerate() {
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

    fn process_audio_i16(
        output: &mut [i16],
        layers: &mut [Box<dyn Layer>],
        shared_params: &Arc<SharedAudioParams>,
        channels: u16,
    ) {
        // Generate f32 samples first
        let mut f32_buffer = vec![0.0f32; output.len()];
        Self::process_audio_f32(&mut f32_buffer, layers, shared_params, channels);

        // Convert f32 (-1.0..1.0) to i16 (-32768..32767)
        for (i, &sample) in f32_buffer.iter().enumerate() {
            output[i] = (sample * i16::MAX as f32) as i16;
        }
    }

    fn process_audio_u16(
        output: &mut [u16],
        layers: &mut [Box<dyn Layer>],
        shared_params: &Arc<SharedAudioParams>,
        channels: u16,
    ) {
        // Generate f32 samples first
        let mut f32_buffer = vec![0.0f32; output.len()];
        Self::process_audio_f32(&mut f32_buffer, layers, shared_params, channels);

        // Convert f32 (-1.0..1.0) to u16 (0..65535)
        for (i, &sample) in f32_buffer.iter().enumerate() {
            let normalized = (sample + 1.0) * 0.5; // Convert -1..1 to 0..1
            output[i] = (normalized * u16::MAX as f32) as u16;
        }
    }
}
