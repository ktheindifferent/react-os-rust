// Transport layer abstraction for kernel debugger
use alloc::vec::Vec;

pub trait DebugTransport {
    fn initialize(&mut self) -> bool;
    fn send(&mut self, data: &[u8]) -> bool;
    fn receive(&mut self, buffer: &mut [u8]) -> Option<usize>;
    fn is_connected(&self) -> bool;
    fn flush(&mut self);
}