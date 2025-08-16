use x86_64::VirtAddr;
use alloc::string::String;
use core::slice;
use crate::memory::userspace::{validate_user_buffer, USER_SPACE_MANAGER};
use crate::process::PROCESS_MANAGER;
use super::{EINVAL, EFAULT, ENOMEM, ENOSYS, EBADF};

pub fn sys_exit(status: i32) -> Result<usize, usize> {
    crate::serial_println!("Process exiting with status: {}", status);
    
    let mut pm = PROCESS_MANAGER.lock();
    if let Some(current) = pm.current_process {
        pm.terminate_process(current);
    }
    
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn sys_read(fd: usize, buf: usize, count: usize) -> Result<usize, usize> {
    let buf_addr = VirtAddr::new(buf as u64);
    
    if !validate_user_buffer(buf_addr, count) {
        return Err(EFAULT);
    }
    
    match fd {
        0 => {
            let buffer = unsafe { slice::from_raw_parts_mut(buf as *mut u8, count) };
            let mut bytes_read = 0;
            
            for i in 0..count.min(1) {
                if let Some(c) = crate::interrupts::keyboard::read_char() {
                    buffer[i] = c;
                    bytes_read += 1;
                } else {
                    break;
                }
            }
            
            Ok(bytes_read)
        }
        _ => Err(EBADF),
    }
}

pub fn sys_write(fd: usize, buf: usize, count: usize) -> Result<usize, usize> {
    let buf_addr = VirtAddr::new(buf as u64);
    
    if !validate_user_buffer(buf_addr, count) {
        return Err(EFAULT);
    }
    
    match fd {
        1 | 2 => {
            let buffer = unsafe { slice::from_raw_parts(buf as *const u8, count) };
            
            for &byte in buffer {
                if byte == b'\n' {
                    crate::println!();
                } else if byte.is_ascii() {
                    crate::print!("{}", byte as char);
                }
            }
            
            Ok(count)
        }
        _ => Err(EBADF),
    }
}

pub fn sys_open(path: usize, flags: usize) -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_close(fd: usize) -> Result<usize, usize> {
    match fd {
        0 | 1 | 2 => Ok(0),
        _ => Err(EBADF),
    }
}

pub fn sys_fork() -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_exec(path: usize, argv: usize) -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_wait(pid: usize) -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_kill(pid: usize, signal: usize) -> Result<usize, usize> {
    let pid = crate::process::ProcessId(pid as u32);
    let mut pm = PROCESS_MANAGER.lock();
    
    if pm.get_process(pid).is_some() {
        pm.terminate_process(pid);
        Ok(0)
    } else {
        Err(3)
    }
}

pub fn sys_getpid() -> Result<usize, usize> {
    let pm = PROCESS_MANAGER.lock();
    match pm.current_process {
        Some(pid) => Ok(pid.0 as usize),
        None => Ok(0),
    }
}

pub fn sys_brk(addr: usize) -> Result<usize, usize> {
    let new_brk = VirtAddr::new(addr as u64);
    let mut usm = USER_SPACE_MANAGER.lock();
    
    let pm = PROCESS_MANAGER.lock();
    if let Some(current_pid) = pm.current_process {
        if let Some(space) = usm.get_address_space_mut(current_pid.0 as u64) {
            match space.set_brk(new_brk) {
                Ok(old_brk) => Ok(old_brk.as_u64() as usize),
                Err(_) => Err(ENOMEM),
            }
        } else {
            Err(EINVAL)
        }
    } else {
        Err(EINVAL)
    }
}

pub fn sys_mmap(addr: usize, length: usize, prot: usize, flags: usize, fd: usize, offset: usize) -> Result<usize, usize> {
    use crate::memory::userspace::MemoryRegionType;
    
    let mut usm = USER_SPACE_MANAGER.lock();
    let pm = PROCESS_MANAGER.lock();
    
    if let Some(current_pid) = pm.current_process {
        if let Some(space) = usm.get_address_space_mut(current_pid.0 as u64) {
            let alignment = 0x1000;
            
            let region_addr = if addr == 0 {
                space.find_free_region(length as u64, alignment)
                    .ok_or(ENOMEM)?
            } else {
                VirtAddr::new(addr as u64)
            };
            
            let region = crate::memory::userspace::MemoryRegion::new(
                region_addr,
                region_addr + length,
                MemoryRegionType::Data,
            );
            
            space.add_region(region)
                .map_err(|_| ENOMEM)?;
            
            Ok(region_addr.as_u64() as usize)
        } else {
            Err(EINVAL)
        }
    } else {
        Err(EINVAL)
    }
}

pub fn sys_munmap(addr: usize, length: usize) -> Result<usize, usize> {
    let mut usm = USER_SPACE_MANAGER.lock();
    let pm = PROCESS_MANAGER.lock();
    
    if let Some(current_pid) = pm.current_process {
        if let Some(space) = usm.get_address_space_mut(current_pid.0 as u64) {
            let vaddr = VirtAddr::new(addr as u64);
            
            if space.remove_region(vaddr).is_some() {
                Ok(0)
            } else {
                Err(EINVAL)
            }
        } else {
            Err(EINVAL)
        }
    } else {
        Err(EINVAL)
    }
}

pub fn sys_sleep(milliseconds: usize) -> Result<usize, usize> {
    let start = crate::cpu::rdtsc();
    let cpu_freq = 2_000_000_000;
    let cycles_to_wait = (milliseconds as u64 * cpu_freq) / 1000;
    
    while crate::cpu::rdtsc() - start < cycles_to_wait {
        x86_64::instructions::hlt();
    }
    
    Ok(0)
}

pub fn sys_gettime() -> Result<usize, usize> {
    let tsc = crate::cpu::rdtsc();
    let seconds_since_boot = tsc / 2_000_000_000;
    Ok(seconds_since_boot as usize)
}

pub fn sys_create_window(x: usize, y: usize, width: usize, height: usize) -> Result<usize, usize> {
    use crate::graphics::window::{Window, WINDOW_MANAGER};
    
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(ref mut manager) = *wm {
        let window = Window::new(
            x as i32,
            y as i32,
            width as u32,
            height as u32,
            String::from("User Window"),
        );
        
        let id = manager.create_window(window);
        Ok(id.0 as usize)
    } else {
        Err(ENOSYS)
    }
}

pub fn sys_destroy_window(window_id: usize) -> Result<usize, usize> {
    use crate::graphics::window::{WindowId, WINDOW_MANAGER};
    
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(ref mut manager) = *wm {
        manager.destroy_window(WindowId(window_id as u32));
        Ok(0)
    } else {
        Err(ENOSYS)
    }
}

pub fn sys_draw_window(window_id: usize, buffer_ptr: usize) -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_handle_event(event_ptr: usize) -> Result<usize, usize> {
    Err(ENOSYS)
}

pub fn sys_get_screen_info(info_ptr: usize) -> Result<usize, usize> {
    use crate::graphics::VESA_DRIVER;
    
    let info_addr = VirtAddr::new(info_ptr as u64);
    if !validate_user_buffer(info_addr, 12) {
        return Err(EFAULT);
    }
    
    let driver = VESA_DRIVER.lock();
    if let Some(fb) = &driver.framebuffer {
        let info = unsafe { &mut *(info_ptr as *mut ScreenInfo) };
        info.width = fb.width as u32;
        info.height = fb.height as u32;
        info.bpp = fb.bpp as u32;
        Ok(0)
    } else {
        Err(ENOSYS)
    }
}

#[repr(C)]
struct ScreenInfo {
    width: u32,
    height: u32,
    bpp: u32,
}