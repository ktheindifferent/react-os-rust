// GDI (Graphics Device Interface) implementation for Win32 subsystem
use super::*;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Device Context structure
#[derive(Debug, Clone)]
pub struct DeviceContext {
    pub handle: HANDLE,
    pub window: Option<HANDLE>,
    pub pen: Option<HANDLE>,
    pub brush: Option<HANDLE>,
    pub font: Option<HANDLE>,
    pub text_color: COLORREF,
    pub background_color: COLORREF,
    pub background_mode: i32,
    pub position: Point,
    pub clip_region: Option<Region>,
}

// GDI Object types
#[derive(Debug, Clone)]
pub enum GdiObject {
    Pen(PenObject),
    Brush(BrushObject),
    Font(FontObject),
    Bitmap(BitmapObject),
    Region(Region),
    Palette(PaletteObject),
    DeviceContext(DeviceContext),
}

// Pen object
#[derive(Debug, Clone)]
pub struct PenObject {
    pub style: i32,
    pub width: i32,
    pub color: COLORREF,
}

// Brush object
#[derive(Debug, Clone)]
pub struct BrushObject {
    pub style: i32,
    pub color: COLORREF,
    pub hatch: i32,
}

// Font object
#[derive(Debug, Clone)]
pub struct FontObject {
    pub height: i32,
    pub width: i32,
    pub weight: i32,
    pub italic: bool,
    pub underline: bool,
    pub strike_out: bool,
    pub face_name: [u8; 32],
}

// Bitmap object
#[derive(Debug, Clone)]
pub struct BitmapObject {
    pub width: i32,
    pub height: i32,
    pub bits_per_pixel: i32,
    pub data: Vec<u8>,
}

// Region object
#[derive(Debug, Clone)]
pub struct Region {
    pub rects: Vec<Rect>,
}

// Palette object
#[derive(Debug, Clone)]
pub struct PaletteObject {
    pub entries: Vec<COLORREF>,
}

// Rectangle structure
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// Point structure
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

// Color reference type
pub type COLORREF = u32;

// RGB macro
pub fn RGB(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

// GDI Manager
pub struct GdiManager {
    objects: BTreeMap<u64, GdiObject>,
    next_handle: u64,
    stock_objects: BTreeMap<i32, HANDLE>,
}

lazy_static! {
    pub static ref GDI_MANAGER: Mutex<GdiManager> = Mutex::new(GdiManager::new());
}

impl GdiManager {
    pub fn new() -> Self {
        let mut manager = Self {
            objects: BTreeMap::new(),
            next_handle: 0x1000,
            stock_objects: BTreeMap::new(),
        };
        
        // Initialize stock objects
        manager.init_stock_objects();
        manager
    }
    
    fn init_stock_objects(&mut self) {
        // White brush
        let white_brush = self.create_solid_brush(RGB(255, 255, 255));
        self.stock_objects.insert(WHITE_BRUSH, white_brush);
        
        // Black brush
        let black_brush = self.create_solid_brush(RGB(0, 0, 0));
        self.stock_objects.insert(BLACK_BRUSH, black_brush);
        
        // Black pen
        let black_pen = self.create_pen(PS_SOLID, 1, RGB(0, 0, 0));
        self.stock_objects.insert(BLACK_PEN, black_pen);
        
        // White pen
        let white_pen = self.create_pen(PS_SOLID, 1, RGB(255, 255, 255));
        self.stock_objects.insert(WHITE_PEN, white_pen);
        
        // System font
        let system_font = self.create_font(12, 0, 0, 0, FW_NORMAL, false, false, false);
        self.stock_objects.insert(SYSTEM_FONT, system_font);
    }
    
    pub fn allocate_handle(&mut self) -> HANDLE {
        let handle = Handle(self.next_handle);
        self.next_handle += 1;
        handle
    }
    
    pub fn create_dc(&mut self, window: Option<HANDLE>) -> HANDLE {
        let handle = self.allocate_handle();
        let dc = DeviceContext {
            handle,
            window,
            pen: self.stock_objects.get(&BLACK_PEN).copied(),
            brush: self.stock_objects.get(&WHITE_BRUSH).copied(),
            font: self.stock_objects.get(&SYSTEM_FONT).copied(),
            text_color: RGB(0, 0, 0),
            background_color: RGB(255, 255, 255),
            background_mode: OPAQUE,
            position: Point { x: 0, y: 0 },
            clip_region: None,
        };
        self.objects.insert(handle.0, GdiObject::DeviceContext(dc.clone()));
        handle
    }
    
    pub fn delete_dc(&mut self, hdc: HANDLE) -> bool {
        self.objects.remove(&hdc.0).is_some()
    }
    
    pub fn create_pen(&mut self, style: i32, width: i32, color: COLORREF) -> HANDLE {
        let handle = self.allocate_handle();
        let pen = PenObject { style, width, color };
        self.objects.insert(handle.0, GdiObject::Pen(pen));
        handle
    }
    
    pub fn create_solid_brush(&mut self, color: COLORREF) -> HANDLE {
        let handle = self.allocate_handle();
        let brush = BrushObject {
            style: BS_SOLID,
            color,
            hatch: 0,
        };
        self.objects.insert(handle.0, GdiObject::Brush(brush));
        handle
    }
    
    pub fn create_font(
        &mut self,
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: bool,
        underline: bool,
        strike_out: bool,
    ) -> HANDLE {
        let handle = self.allocate_handle();
        let font = FontObject {
            height,
            width,
            weight,
            italic,
            underline,
            strike_out,
            face_name: [0; 32],
        };
        self.objects.insert(handle.0, GdiObject::Font(font));
        handle
    }
    
    pub fn select_object(&mut self, hdc: HANDLE, obj: HANDLE) -> Option<HANDLE> {
        // First check if the object exists and get its type
        let obj_type = if let Some(gdi_obj) = self.objects.get(&obj.0) {
            match gdi_obj {
                GdiObject::Pen(_) => Some(1),
                GdiObject::Brush(_) => Some(2),
                GdiObject::Font(_) => Some(3),
                _ => None,
            }
        } else {
            None
        };
        
        // Now update the DC based on the object type
        if let Some(obj_type) = obj_type {
            if let Some(GdiObject::DeviceContext(ref mut dc)) = self.objects.get_mut(&hdc.0) {
                match obj_type {
                    1 => { // Pen
                        let old = dc.pen;
                        dc.pen = Some(obj);
                        return old;
                    }
                    2 => { // Brush
                        let old = dc.brush;
                        dc.brush = Some(obj);
                        return old;
                    }
                    3 => { // Font
                        let old = dc.font;
                        dc.font = Some(obj);
                        return old;
                    }
                    _ => {}
                }
            }
        }
        None
    }
    
    pub fn delete_object(&mut self, obj: HANDLE) -> bool {
        // Don't delete stock objects
        for (_, &stock_handle) in &self.stock_objects {
            if stock_handle == obj {
                return false;
            }
        }
        self.objects.remove(&obj.0).is_some()
    }
    
    pub fn get_stock_object(&self, index: i32) -> Option<HANDLE> {
        self.stock_objects.get(&index).copied()
    }
}

// Stock object constants
pub const WHITE_BRUSH: i32 = 0;
pub const LTGRAY_BRUSH: i32 = 1;
pub const GRAY_BRUSH: i32 = 2;
pub const DKGRAY_BRUSH: i32 = 3;
pub const BLACK_BRUSH: i32 = 4;
pub const NULL_BRUSH: i32 = 5;
pub const WHITE_PEN: i32 = 6;
pub const BLACK_PEN: i32 = 7;
pub const NULL_PEN: i32 = 8;
pub const SYSTEM_FONT: i32 = 13;
pub const DEFAULT_PALETTE: i32 = 15;

// Pen styles
pub const PS_SOLID: i32 = 0;
pub const PS_DASH: i32 = 1;
pub const PS_DOT: i32 = 2;
pub const PS_DASHDOT: i32 = 3;
pub const PS_DASHDOTDOT: i32 = 4;
pub const PS_NULL: i32 = 5;

// Brush styles
pub const BS_SOLID: i32 = 0;
pub const BS_NULL: i32 = 1;
pub const BS_HATCHED: i32 = 2;
pub const BS_PATTERN: i32 = 3;

// Font weights
pub const FW_DONTCARE: i32 = 0;
pub const FW_THIN: i32 = 100;
pub const FW_LIGHT: i32 = 300;
pub const FW_NORMAL: i32 = 400;
pub const FW_MEDIUM: i32 = 500;
pub const FW_SEMIBOLD: i32 = 600;
pub const FW_BOLD: i32 = 700;
pub const FW_EXTRABOLD: i32 = 800;
pub const FW_HEAVY: i32 = 900;

// Background modes
pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;

// GDI API Functions

/// CreateDC - Create a device context
#[no_mangle]
pub extern "C" fn CreateDCA(
    _driver: LPCSTR,
    _device: LPCSTR,
    _output: LPCSTR,
    _init_data: *const u8,
) -> HANDLE {
    GDI_MANAGER.lock().create_dc(None)
}

/// DeleteDC - Delete a device context
#[no_mangle]
pub extern "C" fn DeleteDC(hdc: HANDLE) -> BOOL {
    if GDI_MANAGER.lock().delete_dc(hdc) {
        1
    } else {
        0
    }
}

/// GetDC - Get device context for a window
#[no_mangle]
pub extern "C" fn GetDC(hwnd: HANDLE) -> HANDLE {
    GDI_MANAGER.lock().create_dc(Some(hwnd))
}

/// ReleaseDC - Release a device context
#[no_mangle]
pub extern "C" fn ReleaseDC(_hwnd: HANDLE, hdc: HANDLE) -> i32 {
    if GDI_MANAGER.lock().delete_dc(hdc) {
        1
    } else {
        0
    }
}

/// CreatePen - Create a pen object
#[no_mangle]
pub extern "C" fn CreatePen(style: i32, width: i32, color: COLORREF) -> HANDLE {
    GDI_MANAGER.lock().create_pen(style, width, color)
}

/// CreateSolidBrush - Create a solid brush
#[no_mangle]
pub extern "C" fn CreateSolidBrush(color: COLORREF) -> HANDLE {
    GDI_MANAGER.lock().create_solid_brush(color)
}

/// CreateFontA - Create a font object
#[no_mangle]
pub extern "C" fn CreateFontA(
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: DWORD,
    underline: DWORD,
    strike_out: DWORD,
    _charset: DWORD,
    _out_precision: DWORD,
    _clip_precision: DWORD,
    _quality: DWORD,
    _pitch_family: DWORD,
    _face_name: LPCSTR,
) -> HANDLE {
    GDI_MANAGER.lock().create_font(
        height,
        width,
        escapement,
        orientation,
        weight,
        italic != 0,
        underline != 0,
        strike_out != 0,
    )
}

/// SelectObject - Select an object into a device context
#[no_mangle]
pub extern "C" fn SelectObject(hdc: HANDLE, obj: HANDLE) -> HANDLE {
    GDI_MANAGER.lock().select_object(hdc, obj).unwrap_or(Handle::NULL)
}

/// DeleteObject - Delete a GDI object
#[no_mangle]
pub extern "C" fn DeleteObject(obj: HANDLE) -> BOOL {
    if GDI_MANAGER.lock().delete_object(obj) {
        1
    } else {
        0
    }
}

/// GetStockObject - Get a stock GDI object
#[no_mangle]
pub extern "C" fn GetStockObject(index: i32) -> HANDLE {
    GDI_MANAGER.lock().get_stock_object(index).unwrap_or(Handle::NULL)
}

/// SetTextColor - Set text color for a device context
#[no_mangle]
pub extern "C" fn SetTextColor(hdc: HANDLE, color: COLORREF) -> COLORREF {
    let mut manager = GDI_MANAGER.lock();
    if let Some(GdiObject::DeviceContext(ref mut dc)) = manager.objects.get_mut(&hdc.0) {
        let old_color = dc.text_color;
        dc.text_color = color;
        old_color
    } else {
        0
    }
}

/// SetBkColor - Set background color for a device context
#[no_mangle]
pub extern "C" fn SetBkColor(hdc: HANDLE, color: COLORREF) -> COLORREF {
    let mut manager = GDI_MANAGER.lock();
    if let Some(GdiObject::DeviceContext(ref mut dc)) = manager.objects.get_mut(&hdc.0) {
        let old_color = dc.background_color;
        dc.background_color = color;
        old_color
    } else {
        0
    }
}

/// SetBkMode - Set background mode for a device context
#[no_mangle]
pub extern "C" fn SetBkMode(hdc: HANDLE, mode: i32) -> i32 {
    let mut manager = GDI_MANAGER.lock();
    if let Some(GdiObject::DeviceContext(ref mut dc)) = manager.objects.get_mut(&hdc.0) {
        let old_mode = dc.background_mode;
        dc.background_mode = mode;
        old_mode
    } else {
        0
    }
}

/// TextOutA - Output text to a device context
#[no_mangle]
pub extern "C" fn TextOutA(
    hdc: HANDLE,
    x: i32,
    y: i32,
    text: LPCSTR,
    length: i32,
) -> BOOL {
    if text.is_null() || length <= 0 {
        return 0;
    }
    
    let text_slice = unsafe { core::slice::from_raw_parts(text, length as usize) };
    if let Ok(text_str) = core::str::from_utf8(text_slice) {
        crate::println!("TextOut at ({}, {}): {}", x, y, text_str);
        1
    } else {
        0
    }
}

/// Rectangle - Draw a rectangle
#[no_mangle]
pub extern "C" fn Rectangle(
    hdc: HANDLE,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> BOOL {
    crate::println!("Rectangle: ({}, {}) to ({}, {})", left, top, right, bottom);
    1
}

/// LineTo - Draw a line to a point
#[no_mangle]
pub extern "C" fn LineTo(hdc: HANDLE, x: i32, y: i32) -> BOOL {
    let mut manager = GDI_MANAGER.lock();
    if let Some(GdiObject::DeviceContext(ref mut dc)) = manager.objects.get_mut(&hdc.0) {
        crate::println!("LineTo: ({}, {}) to ({}, {})", dc.position.x, dc.position.y, x, y);
        dc.position.x = x;
        dc.position.y = y;
        1
    } else {
        0
    }
}

/// MoveToEx - Move the current position
#[no_mangle]
pub extern "C" fn MoveToEx(
    hdc: HANDLE,
    x: i32,
    y: i32,
    point: *mut Point,
) -> BOOL {
    let mut manager = GDI_MANAGER.lock();
    if let Some(GdiObject::DeviceContext(ref mut dc)) = manager.objects.get_mut(&hdc.0) {
        if !point.is_null() {
            unsafe {
                *point = dc.position;
            }
        }
        dc.position.x = x;
        dc.position.y = y;
        1
    } else {
        0
    }
}