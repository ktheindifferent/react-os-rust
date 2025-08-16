use super::{VideoDecoder, VideoEncoder, VideoFrame};
use crate::multimedia::{MediaError, PixelFormat};
use alloc::vec::Vec;

pub struct H265Decoder {
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
}

impl H265Decoder {
    pub fn new() -> Self {
        Self {
            width: 3840,
            height: 2160,
            pixel_format: PixelFormat::YUV420P,
        }
    }
}

impl VideoDecoder for H265Decoder {
    fn decode_frame(&mut self, _data: &[u8]) -> Result<VideoFrame, MediaError> {
        let y_size = (self.width * self.height) as usize;
        let uv_size = y_size / 4;
        
        Ok(VideoFrame {
            data: vec![vec![0; y_size], vec![128; uv_size], vec![128; uv_size]],
            linesize: vec![self.width as usize, self.width as usize / 2, self.width as usize / 2],
            width: self.width,
            height: self.height,
            pixel_format: self.pixel_format,
            pts: 0,
            key_frame: false,
        })
    }

    fn get_width(&self) -> u32 { self.width }
    fn get_height(&self) -> u32 { self.height }
    fn get_pixel_format(&self) -> PixelFormat { self.pixel_format }
    fn reset(&mut self) {}
}

pub struct H265Encoder {
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
    bitrate: u32,
}

impl H265Encoder {
    pub fn new() -> Self {
        Self {
            width: 3840,
            height: 2160,
            pixel_format: PixelFormat::YUV420P,
            bitrate: 4_000_000,
        }
    }
}

impl VideoEncoder for H265Encoder {
    fn encode_frame(&mut self, _frame: &VideoFrame) -> Result<Vec<u8>, MediaError> {
        Ok(vec![0x00, 0x00, 0x00, 0x01, 0x40, 0x01])
    }

    fn set_resolution(&mut self, width: u32, height: u32) -> Result<(), MediaError> {
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn set_pixel_format(&mut self, format: PixelFormat) -> Result<(), MediaError> {
        self.pixel_format = format;
        Ok(())
    }

    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError> {
        self.bitrate = bitrate;
        Ok(())
    }

    fn set_framerate(&mut self, _fps: f32) -> Result<(), MediaError> { Ok(()) }
    fn request_keyframe(&mut self) {}
    fn flush(&mut self) -> Result<Vec<u8>, MediaError> { Ok(Vec::new()) }
}