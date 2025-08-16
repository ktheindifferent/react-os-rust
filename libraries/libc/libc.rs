#![no_std]
#![no_main]

use core::mem;
use core::ptr;
use core::slice;

pub mod stdio {
    use super::*;
    
    pub struct FILE {
        fd: i32,
        buffer: [u8; 4096],
        buf_pos: usize,
        buf_len: usize,
        flags: u32,
    }
    
    pub static mut stdin: FILE = FILE {
        fd: 0,
        buffer: [0; 4096],
        buf_pos: 0,
        buf_len: 0,
        flags: 0,
    };
    
    pub static mut stdout: FILE = FILE {
        fd: 1,
        buffer: [0; 4096],
        buf_pos: 0,
        buf_len: 0,
        flags: 0,
    };
    
    pub static mut stderr: FILE = FILE {
        fd: 2,
        buffer: [0; 4096],
        buf_pos: 0,
        buf_len: 0,
        flags: 0,
    };
    
    pub fn printf(format: &str, args: &[&dyn core::fmt::Display]) -> i32 {
        let mut output = String::new();
        let mut arg_idx = 0;
        let mut chars = format.chars();
        
        while let Some(ch) = chars.next() {
            if ch == '%' {
                if let Some(next) = chars.next() {
                    match next {
                        's' | 'd' | 'x' | 'f' => {
                            if arg_idx < args.len() {
                                use core::fmt::Write;
                                let _ = write!(output, "{}", args[arg_idx]);
                                arg_idx += 1;
                            }
                        }
                        '%' => output.push('%'),
                        _ => {
                            output.push('%');
                            output.push(next);
                        }
                    }
                }
            } else {
                output.push(ch);
            }
        }
        
        fputs(&output, unsafe { &mut stdout })
    }
    
    pub fn fprintf(file: &mut FILE, format: &str, args: &[&dyn core::fmt::Display]) -> i32 {
        let mut output = String::new();
        let mut arg_idx = 0;
        let mut chars = format.chars();
        
        while let Some(ch) = chars.next() {
            if ch == '%' {
                if let Some(next) = chars.next() {
                    match next {
                        's' | 'd' | 'x' | 'f' => {
                            if arg_idx < args.len() {
                                use core::fmt::Write;
                                let _ = write!(output, "{}", args[arg_idx]);
                                arg_idx += 1;
                            }
                        }
                        '%' => output.push('%'),
                        _ => {
                            output.push('%');
                            output.push(next);
                        }
                    }
                }
            } else {
                output.push(ch);
            }
        }
        
        fputs(&output, file)
    }
    
    pub fn fopen(path: &str, mode: &str) -> Option<*mut FILE> {
        None
    }
    
    pub fn fclose(_file: *mut FILE) -> i32 {
        0
    }
    
    pub fn fread(buffer: &mut [u8], size: usize, count: usize, file: &mut FILE) -> usize {
        0
    }
    
    pub fn fwrite(buffer: &[u8], size: usize, count: usize, file: &mut FILE) -> usize {
        0
    }
    
    pub fn fgets(buffer: &mut [u8], size: i32, file: &mut FILE) -> Option<&[u8]> {
        None
    }
    
    pub fn fputs(s: &str, file: &mut FILE) -> i32 {
        0
    }
    
    pub fn fflush(_file: &mut FILE) -> i32 {
        0
    }
}

pub mod stdlib {
    use super::*;
    
    pub fn malloc(size: usize) -> *mut u8 {
        ptr::null_mut()
    }
    
    pub fn calloc(count: usize, size: usize) -> *mut u8 {
        let total = count * size;
        let ptr = malloc(total);
        if !ptr.is_null() {
            unsafe {
                ptr::write_bytes(ptr, 0, total);
            }
        }
        ptr
    }
    
    pub fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
        ptr::null_mut()
    }
    
    pub fn free(_ptr: *mut u8) {
    }
    
    pub fn abort() -> ! {
        loop {}
    }
    
    pub fn exit(status: i32) -> ! {
        loop {}
    }
    
    pub fn atexit(_func: fn()) -> i32 {
        0
    }
    
    pub fn atoi(s: &str) -> i32 {
        s.parse().unwrap_or(0)
    }
    
    pub fn atol(s: &str) -> i64 {
        s.parse().unwrap_or(0)
    }
    
    pub fn atof(s: &str) -> f64 {
        s.parse().unwrap_or(0.0)
    }
    
    pub fn strtol(s: &str, endptr: Option<&mut *const u8>, base: i32) -> i64 {
        0
    }
    
    pub fn strtoul(s: &str, endptr: Option<&mut *const u8>, base: i32) -> u64 {
        0
    }
    
    pub fn strtod(s: &str, endptr: Option<&mut *const u8>) -> f64 {
        0.0
    }
    
    pub fn rand() -> i32 {
        0
    }
    
    pub fn srand(_seed: u32) {
    }
    
    pub fn qsort<T>(base: &mut [T], compare: fn(&T, &T) -> i32) {
        base.sort_by(|a, b| {
            match compare(a, b) {
                x if x < 0 => core::cmp::Ordering::Less,
                x if x > 0 => core::cmp::Ordering::Greater,
                _ => core::cmp::Ordering::Equal,
            }
        });
    }
    
    pub fn bsearch<T>(key: &T, base: &[T], compare: fn(&T, &T) -> i32) -> Option<&T> {
        base.binary_search_by(|elem| {
            match compare(key, elem) {
                x if x < 0 => core::cmp::Ordering::Less,
                x if x > 0 => core::cmp::Ordering::Greater,
                _ => core::cmp::Ordering::Equal,
            }
        }).ok().map(|idx| &base[idx])
    }
}

pub mod string {
    use super::*;
    
    pub fn strlen(s: &str) -> usize {
        s.len()
    }
    
    pub fn strcpy(dest: &mut [u8], src: &str) -> &mut [u8] {
        let bytes = src.as_bytes();
        let len = bytes.len().min(dest.len() - 1);
        dest[..len].copy_from_slice(&bytes[..len]);
        dest[len] = 0;
        dest
    }
    
    pub fn strncpy(dest: &mut [u8], src: &str, n: usize) -> &mut [u8] {
        let bytes = src.as_bytes();
        let len = bytes.len().min(n).min(dest.len());
        dest[..len].copy_from_slice(&bytes[..len]);
        if len < n && len < dest.len() {
            dest[len] = 0;
        }
        dest
    }
    
    pub fn strcat(dest: &mut [u8], src: &str) -> &mut [u8] {
        let dest_len = strlen(core::str::from_utf8(dest).unwrap_or(""));
        let src_bytes = src.as_bytes();
        let copy_len = src_bytes.len().min(dest.len() - dest_len - 1);
        dest[dest_len..dest_len + copy_len].copy_from_slice(&src_bytes[..copy_len]);
        dest[dest_len + copy_len] = 0;
        dest
    }
    
    pub fn strcmp(s1: &str, s2: &str) -> i32 {
        match s1.cmp(s2) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Equal => 0,
        }
    }
    
    pub fn strncmp(s1: &str, s2: &str, n: usize) -> i32 {
        let s1_bytes = &s1.as_bytes()[..n.min(s1.len())];
        let s2_bytes = &s2.as_bytes()[..n.min(s2.len())];
        match s1_bytes.cmp(s2_bytes) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Equal => 0,
        }
    }
    
    pub fn strchr(s: &str, c: char) -> Option<usize> {
        s.find(c)
    }
    
    pub fn strrchr(s: &str, c: char) -> Option<usize> {
        s.rfind(c)
    }
    
    pub fn strstr(haystack: &str, needle: &str) -> Option<usize> {
        haystack.find(needle)
    }
    
    pub fn strtok(s: &mut str, delim: &str) -> Option<&str> {
        None
    }
    
    pub fn memcpy(dest: &mut [u8], src: &[u8], n: usize) -> &mut [u8] {
        let copy_len = n.min(dest.len()).min(src.len());
        dest[..copy_len].copy_from_slice(&src[..copy_len]);
        dest
    }
    
    pub fn memmove(dest: &mut [u8], src: &[u8], n: usize) -> &mut [u8] {
        let copy_len = n.min(dest.len()).min(src.len());
        unsafe {
            ptr::copy(src.as_ptr(), dest.as_mut_ptr(), copy_len);
        }
        dest
    }
    
    pub fn memset(s: &mut [u8], c: u8, n: usize) -> &mut [u8] {
        let fill_len = n.min(s.len());
        for i in 0..fill_len {
            s[i] = c;
        }
        s
    }
    
    pub fn memcmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
        let cmp_len = n.min(s1.len()).min(s2.len());
        match s1[..cmp_len].cmp(&s2[..cmp_len]) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Greater => 1,
            core::cmp::Ordering::Equal => 0,
        }
    }
}

pub mod math {
    pub fn abs(x: i32) -> i32 {
        x.abs()
    }
    
    pub fn labs(x: i64) -> i64 {
        x.abs()
    }
    
    pub fn fabs(x: f64) -> f64 {
        x.abs()
    }
    
    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }
    
    pub fn pow(base: f64, exp: f64) -> f64 {
        base.powf(exp)
    }
    
    pub fn exp(x: f64) -> f64 {
        x.exp()
    }
    
    pub fn log(x: f64) -> f64 {
        x.ln()
    }
    
    pub fn log10(x: f64) -> f64 {
        x.log10()
    }
    
    pub fn sin(x: f64) -> f64 {
        x.sin()
    }
    
    pub fn cos(x: f64) -> f64 {
        x.cos()
    }
    
    pub fn tan(x: f64) -> f64 {
        x.tan()
    }
    
    pub fn asin(x: f64) -> f64 {
        x.asin()
    }
    
    pub fn acos(x: f64) -> f64 {
        x.acos()
    }
    
    pub fn atan(x: f64) -> f64 {
        x.atan()
    }
    
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
    
    pub fn ceil(x: f64) -> f64 {
        x.ceil()
    }
    
    pub fn floor(x: f64) -> f64 {
        x.floor()
    }
    
    pub fn round(x: f64) -> f64 {
        x.round()
    }
}

pub mod time {
    #[repr(C)]
    pub struct tm {
        pub tm_sec: i32,
        pub tm_min: i32,
        pub tm_hour: i32,
        pub tm_mday: i32,
        pub tm_mon: i32,
        pub tm_year: i32,
        pub tm_wday: i32,
        pub tm_yday: i32,
        pub tm_isdst: i32,
    }
    
    pub type time_t = i64;
    pub type clock_t = i64;
    
    pub fn time(t: Option<&mut time_t>) -> time_t {
        let now = 0;
        if let Some(t_ref) = t {
            *t_ref = now;
        }
        now
    }
    
    pub fn clock() -> clock_t {
        0
    }
    
    pub fn difftime(time1: time_t, time0: time_t) -> f64 {
        (time1 - time0) as f64
    }
    
    pub fn mktime(_tm: &tm) -> time_t {
        0
    }
    
    pub fn gmtime(_t: &time_t) -> Option<tm> {
        None
    }
    
    pub fn localtime(_t: &time_t) -> Option<tm> {
        None
    }
}

pub mod errno {
    pub static mut errno: i32 = 0;
    
    pub const EPERM: i32 = 1;
    pub const ENOENT: i32 = 2;
    pub const ESRCH: i32 = 3;
    pub const EINTR: i32 = 4;
    pub const EIO: i32 = 5;
    pub const ENXIO: i32 = 6;
    pub const E2BIG: i32 = 7;
    pub const ENOEXEC: i32 = 8;
    pub const EBADF: i32 = 9;
    pub const ECHILD: i32 = 10;
    pub const EAGAIN: i32 = 11;
    pub const ENOMEM: i32 = 12;
    pub const EACCES: i32 = 13;
    pub const EFAULT: i32 = 14;
    pub const ENOTBLK: i32 = 15;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const EXDEV: i32 = 18;
    pub const ENODEV: i32 = 19;
    pub const ENOTDIR: i32 = 20;
    pub const EISDIR: i32 = 21;
    pub const EINVAL: i32 = 22;
    pub const ENFILE: i32 = 23;
    pub const EMFILE: i32 = 24;
    pub const ENOTTY: i32 = 25;
    pub const ETXTBSY: i32 = 26;
    pub const EFBIG: i32 = 27;
    pub const ENOSPC: i32 = 28;
    pub const ESPIPE: i32 = 29;
    pub const EROFS: i32 = 30;
    pub const EMLINK: i32 = 31;
    pub const EPIPE: i32 = 32;
    pub const EDOM: i32 = 33;
    pub const ERANGE: i32 = 34;
}

pub mod ctype {
    pub fn isalnum(c: char) -> bool {
        c.is_alphanumeric()
    }
    
    pub fn isalpha(c: char) -> bool {
        c.is_alphabetic()
    }
    
    pub fn isdigit(c: char) -> bool {
        c.is_ascii_digit()
    }
    
    pub fn isxdigit(c: char) -> bool {
        c.is_ascii_hexdigit()
    }
    
    pub fn islower(c: char) -> bool {
        c.is_lowercase()
    }
    
    pub fn isupper(c: char) -> bool {
        c.is_uppercase()
    }
    
    pub fn isspace(c: char) -> bool {
        c.is_whitespace()
    }
    
    pub fn ispunct(c: char) -> bool {
        c.is_ascii_punctuation()
    }
    
    pub fn isprint(c: char) -> bool {
        !c.is_control()
    }
    
    pub fn iscntrl(c: char) -> bool {
        c.is_control()
    }
    
    pub fn tolower(c: char) -> char {
        c.to_ascii_lowercase()
    }
    
    pub fn toupper(c: char) -> char {
        c.to_ascii_uppercase()
    }
}

use core::fmt::Display;
struct String {
    data: Vec<u8>,
}

impl String {
    fn new() -> Self {
        Self { data: Vec::new() }
    }
    
    fn push(&mut self, ch: char) {
        if let Some(b) = ch.as_ascii() {
            self.data.push(b as u8);
        }
    }
}

impl core::fmt::Write for String {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.data.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

struct Vec<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
}

impl<T> Vec<T> {
    fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
    
    fn push(&mut self, _value: T) {
    }
    
    fn extend_from_slice(&mut self, _slice: &[T]) where T: Clone {
    }
}

impl<T> core::ops::Deref for Vec<T> {
    type Target = [T];
    
    fn deref(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.ptr, self.len)
        }
    }
}