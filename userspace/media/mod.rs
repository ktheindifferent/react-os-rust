pub mod player;
pub mod transcoder;
pub mod editor;

pub use player::{MediaPlayer, PlayerState, PlayerEvent, Playlist};
pub use transcoder::{Transcoder, TranscodeOptions, QualityPreset, VideoCodec, AudioCodec, ContainerFormat};
pub use editor::{VideoEditor, Timeline, Track, Clip, ExportSettings};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_media_player() {
        let mut player = MediaPlayer::new();
        assert_eq!(player.get_state(), PlayerState::Stopped);
        
        // Test opening a file
        let _ = player.open("test.mp4");
        assert_eq!(player.get_state(), PlayerState::Stopped);
        
        // Test play/pause
        let _ = player.play();
        assert_eq!(player.get_state(), PlayerState::Playing);
        
        player.pause();
        assert_eq!(player.get_state(), PlayerState::Paused);
        
        player.stop();
        assert_eq!(player.get_state(), PlayerState::Stopped);
    }

    #[test]
    fn test_playlist() {
        let mut playlist = Playlist::new();
        
        playlist.add(player::PlaylistItem {
            url: String::from("song1.mp3"),
            title: String::from("Song 1"),
            duration: Some(Duration::from_secs(180)),
            metadata: std::collections::HashMap::new(),
        });
        
        playlist.add(player::PlaylistItem {
            url: String::from("song2.mp3"),
            title: String::from("Song 2"),
            duration: Some(Duration::from_secs(200)),
            metadata: std::collections::HashMap::new(),
        });
        
        assert!(playlist.next().is_some());
        assert!(playlist.next().is_some());
        
        playlist.set_repeat_mode(player::RepeatMode::All);
        assert!(playlist.next().is_some()); // Should loop back
    }

    #[test]
    fn test_transcoder() {
        let mut transcoder = Transcoder::new("input.mp4", "output.webm");
        
        transcoder
            .set_video_codec(VideoCodec::VP9)
            .set_audio_codec(AudioCodec::Opus)
            .set_container(ContainerFormat::WebM)
            .set_quality(QualityPreset::High)
            .set_resolution(1280, 720);
        
        // Transcoding would fail without actual files, but we can test the builder
        assert!(transcoder.transcode().is_err());
    }

    #[test]
    fn test_video_editor() {
        let mut editor = VideoEditor::new();
        
        // Add a color clip
        let clip = Clip::new(
            editor::ClipSource::Color(editor::Color { r: 255, g: 0, b: 0, a: 255 }),
            Duration::from_secs(5)
        );
        
        assert!(editor.add_clip(0, clip).is_ok());
        
        // Test split
        assert!(editor.split_clip(0, 0, Duration::from_secs(2)).is_ok());
        
        // Test export settings
        editor.set_export_settings(ExportSettings {
            output_path: String::from("output.mp4"),
            format: editor::ExportFormat::MP4,
            video_codec: String::from("h264"),
            audio_codec: String::from("aac"),
            bitrate: 5_000_000,
            quality: editor::ExportQuality::High,
        });
    }
}