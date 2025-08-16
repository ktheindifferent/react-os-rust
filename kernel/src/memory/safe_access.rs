use core::ptr;
use core::mem;
use x86_64::{VirtAddr, PhysAddr};

pub struct SafeMemoryAccess;

impl SafeMemoryAccess {
    pub fn read_u8(addr: VirtAddr) -> Result<u8, MemoryAccessError> {
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            match Self::try_read_byte(addr.as_u64() as *const u8) {
                Some(val) => Ok(val),
                None => Err(MemoryAccessError::PageFault),
            }
        }
    }

    pub fn read_u16(addr: VirtAddr) -> Result<u16, MemoryAccessError> {
        if !Self::is_aligned(addr, 2) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            match Self::try_read_word(addr.as_u64() as *const u16) {
                Some(val) => Ok(val),
                None => Err(MemoryAccessError::PageFault),
            }
        }
    }

    pub fn read_u32(addr: VirtAddr) -> Result<u32, MemoryAccessError> {
        if !Self::is_aligned(addr, 4) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            match Self::try_read_dword(addr.as_u64() as *const u32) {
                Some(val) => Ok(val),
                None => Err(MemoryAccessError::PageFault),
            }
        }
    }

    pub fn read_u64(addr: VirtAddr) -> Result<u64, MemoryAccessError> {
        if !Self::is_aligned(addr, 8) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            match Self::try_read_qword(addr.as_u64() as *const u64) {
                Some(val) => Ok(val),
                None => Err(MemoryAccessError::PageFault),
            }
        }
    }

    pub fn write_u8(addr: VirtAddr, value: u8) -> Result<(), MemoryAccessError> {
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            if Self::try_write_byte(addr.as_u64() as *mut u8, value) {
                Ok(())
            } else {
                Err(MemoryAccessError::PageFault)
            }
        }
    }

    pub fn write_u16(addr: VirtAddr, value: u16) -> Result<(), MemoryAccessError> {
        if !Self::is_aligned(addr, 2) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            if Self::try_write_word(addr.as_u64() as *mut u16, value) {
                Ok(())
            } else {
                Err(MemoryAccessError::PageFault)
            }
        }
    }

    pub fn write_u32(addr: VirtAddr, value: u32) -> Result<(), MemoryAccessError> {
        if !Self::is_aligned(addr, 4) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            if Self::try_write_dword(addr.as_u64() as *mut u32, value) {
                Ok(())
            } else {
                Err(MemoryAccessError::PageFault)
            }
        }
    }

    pub fn write_u64(addr: VirtAddr, value: u64) -> Result<(), MemoryAccessError> {
        if !Self::is_aligned(addr, 8) {
            return Err(MemoryAccessError::UnalignedAccess);
        }
        if !Self::is_valid_address(addr) {
            return Err(MemoryAccessError::InvalidAddress);
        }
        
        unsafe {
            if Self::try_write_qword(addr.as_u64() as *mut u64, value) {
                Ok(())
            } else {
                Err(MemoryAccessError::PageFault)
            }
        }
    }

    pub fn copy_from_user(dest: &mut [u8], src: VirtAddr) -> Result<(), MemoryAccessError> {
        let src_ptr = src.as_u64() as *const u8;
        
        for i in 0..dest.len() {
            unsafe {
                match Self::try_read_byte(src_ptr.add(i)) {
                    Some(val) => dest[i] = val,
                    None => return Err(MemoryAccessError::PageFault),
                }
            }
        }
        
        Ok(())
    }

    pub fn copy_to_user(dest: VirtAddr, src: &[u8]) -> Result<(), MemoryAccessError> {
        let dest_ptr = dest.as_u64() as *mut u8;
        
        for i in 0..src.len() {
            unsafe {
                if !Self::try_write_byte(dest_ptr.add(i), src[i]) {
                    return Err(MemoryAccessError::PageFault);
                }
            }
        }
        
        Ok(())
    }

    fn is_valid_address(addr: VirtAddr) -> bool {
        // Check if address is in valid range
        // Kernel space: 0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF
        // User space: 0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF
        let raw = addr.as_u64();
        raw < 0x0000_8000_0000_0000 || raw >= 0xFFFF_8000_0000_0000
    }

    fn is_aligned(addr: VirtAddr, alignment: usize) -> bool {
        addr.as_u64() as usize % alignment == 0
    }

    unsafe fn try_read_byte(ptr: *const u8) -> Option<u8> {
        // Use assembly to catch page faults
        let mut result: u8;
        let mut success: u8;
        
        core::arch::asm!(
            "mov {success}, 1",
            "2:",
            "mov {result}, byte ptr [{ptr}]",
            "3:",
            ".pushsection .fixup,\"ax\"",
            "4:",
            "mov {success}, 0",
            "jmp 3b",
            ".popsection",
            ptr = in(reg) ptr,
            result = out(reg_byte) result,
            success = out(reg_byte) success,
            options(nostack, preserves_flags)
        );
        
        if success != 0 {
            Some(result)
        } else {
            None
        }
    }

    unsafe fn try_read_word(ptr: *const u16) -> Option<u16> {
        let mut result: u16;
        let mut success: u8 = 1;
        
        match core::ptr::read_volatile(&success) {
            _ => {
                result = ptr.read_volatile();
                Some(result)
            }
        }
    }

    unsafe fn try_read_dword(ptr: *const u32) -> Option<u32> {
        Some(ptr.read_volatile())
    }

    unsafe fn try_read_qword(ptr: *const u64) -> Option<u64> {
        Some(ptr.read_volatile())
    }

    unsafe fn try_write_byte(ptr: *mut u8, value: u8) -> bool {
        ptr.write_volatile(value);
        true
    }

    unsafe fn try_write_word(ptr: *mut u16, value: u16) -> bool {
        ptr.write_volatile(value);
        true
    }

    unsafe fn try_write_dword(ptr: *mut u32, value: u32) -> bool {
        ptr.write_volatile(value);
        true
    }

    unsafe fn try_write_qword(ptr: *mut u64, value: u64) -> bool {
        ptr.write_volatile(value);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessError {
    InvalidAddress,
    UnalignedAccess,
    PageFault,
    AccessViolation,
}

impl core::fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::InvalidAddress => write!(f, "Invalid memory address"),
            Self::UnalignedAccess => write!(f, "Unaligned memory access"),
            Self::PageFault => write!(f, "Page fault"),
            Self::AccessViolation => write!(f, "Access violation"),
        }
    }
}

pub struct BoundsChecker {
    base: VirtAddr,
    size: usize,
}

impl BoundsChecker {
    pub fn new(base: VirtAddr, size: usize) -> Self {
        Self { base, size }
    }

    pub fn check(&self, addr: VirtAddr, access_size: usize) -> Result<(), MemoryAccessError> {
        let start = self.base.as_u64();
        let end = start + self.size as u64;
        let access_start = addr.as_u64();
        let access_end = access_start + access_size as u64;
        
        if access_start < start || access_end > end {
            return Err(MemoryAccessError::AccessViolation);
        }
        
        Ok(())
    }

    pub fn check_slice(&self, addr: VirtAddr, len: usize, elem_size: usize) -> Result<(), MemoryAccessError> {
        let total_size = len.checked_mul(elem_size)
            .ok_or(MemoryAccessError::InvalidAddress)?;
        self.check(addr, total_size)
    }
}