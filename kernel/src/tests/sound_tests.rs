// Sound Subsystem Tests
#[cfg(test)]
mod tests {
    use crate::sound::*;
    use crate::sound::mixer::*;
    use crate::sound::pcm::*;
    use crate::sound::codec::*;
    use alloc::vec::Vec;
    use alloc::vec;
    
    #[test]
    fn test_audio_format_creation() {
        let format = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        assert_eq!(format.sample_rate, 44100);
        assert_eq!(format.channels, 2);
        assert_eq!(format.format.bytes_per_sample(), 2);
    }
    
    #[test]
    fn test_audio_buffer_creation() {
        let format = AudioFormat::default();
        let buffer = AudioBuffer::new(format.clone());
        
        assert_eq!(buffer.frames, format.buffer_size);
        assert_eq!(buffer.bytes_per_frame(), format.channels as usize * format.format.bytes_per_sample());
    }
    
    #[test]
    fn test_pcm_ring_buffer() {
        let mut ring_buffer = PcmRingBuffer::new(1024);
        
        // Test write
        let data = vec![1, 2, 3, 4, 5];
        let written = ring_buffer.write(&data);
        assert_eq!(written, 5);
        assert_eq!(ring_buffer.available(), 5);
        
        // Test read
        let mut read_data = vec![0; 3];
        let read = ring_buffer.read(&mut read_data);
        assert_eq!(read, 3);
        assert_eq!(read_data, vec![1, 2, 3]);
        assert_eq!(ring_buffer.available(), 2);
        
        // Test wrap-around
        let large_data = vec![0xFF; 1020];
        ring_buffer.write(&large_data);
        assert_eq!(ring_buffer.available(), 1022); // 2 + 1020
        
        // Test buffer full
        let overflow_data = vec![0xAA; 10];
        let written = ring_buffer.write(&overflow_data);
        assert_eq!(written, 2); // Only 2 bytes free
    }
    
    #[test]
    fn test_mixer_controls() {
        let mut mixer = AudioMixer::new(48000, 2);
        
        // Test volume control
        mixer.set_volume(MixerChannel::Master, 0.5, 0.5);
        let control = mixer.get_control(MixerChannel::Master).unwrap();
        assert_eq!(control.volume_left, 0.5);
        assert_eq!(control.volume_right, 0.5);
        
        // Test mute
        if let Some(control) = mixer.get_control_mut(MixerChannel::Master) {
            control.set_mute(true);
            assert!(control.muted);
            control.toggle_mute();
            assert!(!control.muted);
        }
    }
    
    #[test]
    fn test_sample_format_conversion() {
        let mixer = AudioMixer::new(48000, 2);
        
        // Test U8 to float conversion
        let u8_data = vec![0, 128, 255];
        let float_samples = mixer.decode_samples(&u8_data, SampleFormat::U8);
        assert!(float_samples[0] < -0.9); // 0 -> ~-1.0
        assert!(float_samples[1].abs() < 0.1); // 128 -> ~0.0
        assert!(float_samples[2] > 0.9); // 255 -> ~1.0
        
        // Test S16LE to float conversion
        let s16_data = vec![0x00, 0x80, 0xFF, 0x7F]; // -32768, 32767
        let float_samples = mixer.decode_samples(&s16_data, SampleFormat::S16LE);
        assert!(float_samples[0] < -0.9); // -32768 -> ~-1.0
        assert!(float_samples[1] > 0.9); // 32767 -> ~1.0
    }
    
    #[test]
    fn test_wave_header_parsing() {
        use crate::sound::wave::WaveHeader;
        
        let format = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        let header = WaveHeader::new(&format, 88200); // 1 second of audio
        
        assert_eq!(&header.riff, b"RIFF");
        assert_eq!(&header.wave, b"WAVE");
        assert_eq!(header.channels, 2);
        assert_eq!(header.sample_rate, 44100);
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.byte_rate, 44100 * 2 * 2); // sample_rate * channels * bytes_per_sample
    }
    
    #[test]
    fn test_midi_note_to_frequency() {
        use crate::sound::midi::note_to_frequency;
        
        // A4 (MIDI note 69) should be 440 Hz
        let a4_freq = note_to_frequency(69);
        assert!((a4_freq - 440.0).abs() < 1.0);
        
        // C4 (MIDI note 60) should be ~261.63 Hz
        let c4_freq = note_to_frequency(60);
        assert!((c4_freq - 261.63).abs() < 5.0);
        
        // A5 (MIDI note 81) should be 880 Hz
        let a5_freq = note_to_frequency(81);
        assert!((a5_freq - 880.0).abs() < 5.0);
    }
    
    #[test]
    fn test_pcm_channel() {
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 256,
        };
        
        let mut channel = PcmChannel::new(format.clone(), 4096);
        
        // Create stereo S16LE frame (L: 0x1234, R: 0x5678)
        let frame = vec![0x34, 0x12, 0x78, 0x56];
        let written = channel.write_frames(&frame).unwrap();
        assert_eq!(written, 1); // 1 frame written
        
        // Read it back
        let mut read_frame = vec![0; 4];
        let read = channel.read_frames(&mut read_frame).unwrap();
        assert_eq!(read, 1);
        assert_eq!(read_frame, frame);
    }
    
    #[test]
    fn test_codec_manager() {
        let mut manager = CodecManager::new();
        let codecs = manager.list_codecs();
        
        // Should have at least PCM and ADPCM codecs
        assert!(codecs.len() >= 2);
        
        // Find PCM codec
        let pcm_codec = manager.find_codec(CodecType::Pcm);
        assert!(pcm_codec.is_some());
    }
    
    #[test]
    fn test_math_approximations() {
        // Test sine approximation
        let sin_0 = sine_approx(0.0);
        assert!(sin_0.abs() < 0.01);
        
        let sin_pi_2 = sine_approx(core::f32::consts::FRAC_PI_2);
        assert!((sin_pi_2 - 1.0).abs() < 0.01);
        
        let sin_pi = sine_approx(core::f32::consts::PI);
        assert!(sin_pi.abs() < 0.01);
        
        // Test power of 2 approximation
        let pow2_0 = pow2_approx(0.0);
        assert!((pow2_0 - 1.0).abs() < 0.01);
        
        let pow2_1 = pow2_approx(1.0);
        assert!((pow2_1 - 2.0).abs() < 0.1);
        
        let pow2_neg1 = pow2_approx(-1.0);
        assert!((pow2_neg1 - 0.5).abs() < 0.1);
    }
    
    #[test]
    fn test_ac97_controller_creation() {
        use crate::sound::ac97::Ac97Controller;
        
        let controller = Ac97Controller::new(0x200, 0x300);
        assert_eq!(controller.mixer_base, 0x200);
        assert_eq!(controller.bus_master_base, 0x300);
        
        let caps = controller.get_capabilities();
        assert_eq!(caps.name, "AC'97 Audio Codec");
        assert!(caps.has_input);
        assert!(caps.has_output);
    }
    
    #[test]
    fn test_hda_controller_creation() {
        use crate::sound::hda::HdaController;
        
        let controller = HdaController::new(0xFEB00000);
        assert_eq!(controller.base_addr, 0xFEB00000);
        
        let caps = controller.get_capabilities();
        assert_eq!(caps.name, "Intel HD Audio");
        assert!(caps.sample_rates.contains(&48000));
        assert!(caps.formats.contains(&SampleFormat::S16LE));
    }
    
    #[test]
    #[should_panic]
    fn test_invalid_frame_size() {
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 256,
        };
        
        let mut channel = PcmChannel::new(format, 4096);
        
        // Try to write invalid frame size (3 bytes instead of 4)
        let invalid_frame = vec![0x00, 0x01, 0x02];
        channel.write_frames(&invalid_frame).unwrap();
    }
}