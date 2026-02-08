use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::Arc;
use tracing::info;

use crate::layers::DroneLayer;
use crate::layers::Layer;
use crate::params::SharedAudioParams;

/// Audio engine that manages CPAL stream.
pub struct AudioEngine {
    _stream: Stream, // Keep stream alive
    config: StreamConfig,
    layer: Arc<std::sync::Mutex<DroneLayer>>,
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
        let layer: Arc<std::sync::Mutex<DroneLayer>> =
            Arc::new(std::sync::Mutex::new(DroneLayer::new(sample_rate)));
        let layer_clone = Arc::clone(&layer);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut layer = layer_clone.lock().unwrap();
                Self::process_audio(data, &mut layer, &shared_params, config.channels);
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
            config,
            layer,
        })
    }

    fn process_audio(
        output: &mut [f32],
        layer: &mut DroneLayer,
        shared_params: &Arc<SharedAudioParams>,
        channels: u16,
    ) {
        // Read latest params (non-blocking, atomic)
        let params = shared_params.get();

        let mut sample_index = 0;
        while sample_index < output.len() {
            let sample = layer.process(&params);
            for _ in 0..channels {
                if sample_index < output.len() {
                    output[sample_index] = sample;
                    sample_index += 1;
                }
            }
        }
    }
}
