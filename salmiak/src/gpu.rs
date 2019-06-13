pub mod mailbox;
use self::mailbox::{FrameBuffer, MailboxPropertyBufferBuilder, Point, Size};
use crate::memory::{Allocator, Layout, MB};
use crate::prelude::*;

pub struct Color {
    alpha: u8,
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    pub const BLACK: Color = Color {
        alpha: 0,
        red: 0,
        green: 0,
        blue: 0,
    };
    pub const RED: Color = Color {
        alpha: 0,
        red: 255,
        green: 0,
        blue: 0,
    };
    pub const GREEN: Color = Color {
        alpha: 0,
        red: 0,
        green: 255,
        blue: 0,
    };
    pub const BLUE: Color = Color {
        alpha: 0,
        red: 0,
        green: 0,
        blue: 255,
    };

    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    fn interpolate(&self, color_b: &Color, percent: f64) -> Color {
        Color {
            red: ((1.0 - percent) * f64::from(color_b.red) + percent * f64::from(self.red)) as u8,
            green: ((1.0 - percent) * f64::from(color_b.green) + percent * f64::from(self.green))
                as u8,
            blue: ((1.0 - percent) * f64::from(color_b.blue) + percent * f64::from(self.blue))
                as u8,
            alpha: ((1.0 - percent) * f64::from(color_b.alpha) + percent * f64::from(self.alpha))
                as u8,
        }
    }
}

impl From<u32> for Color {
    fn from(n: u32) -> Self {
        Color {
            alpha: (n >> 24) as u8,
            red: (n >> 16) as u8,
            green: (n >> 8) as u8,
            blue: n as u8,
        }
    }
}

impl Into<u32> for &Color {
    fn into(self) -> u32 {
        (u32::from(self.alpha) << 24)
            | (u32::from(self.red) << 16)
            | (u32::from(self.green) << 8)
            | (u32::from(self.blue))
    }
}

pub struct Gpu {
    frame_buffer: FrameBuffer,
    pitch: u32,
    resolution: Size,
    mem_buffer: u32,
}

impl Gpu {
    pub fn new(
        frame_buffer: FrameBuffer,
        resolution: Size,
        pitch: u32,
        allocator: &dyn Allocator,
    ) -> Gpu {
        unsafe {
            let sz = frame_buffer.size as usize;
            let mem = allocator.alloc(Layout::from_size_align_unchecked(sz, 16));
            assert!(!mem.is_null(), "Failed to allocate GPU buffer.");
            Gpu {
                frame_buffer,
                resolution,
                pitch,
                mem_buffer: mem as u32,
            }
        }
    }

    pub fn clear_screen(&self, color: &Color) {
        for x in 0..self.resolution.width {
            for y in 0..self.resolution.height {
                let idx = x * 4 + y * self.pitch; // there are `pitch` bytes in each row, not `width * 4`
                let wr: *mut u32 = (self.mem_buffer + idx) as *mut u32;
                unsafe {
                    wr.write_volatile(color.into());
                }
            }
        }
    }

    pub fn draw_rectangle(&self, ox: u32, oy: u32, width: u32, height: u32, color: &Color) {
        for x in ox..(ox + width) {
            for y in oy..(oy + height) {
                let idx = x * 4 + y * self.pitch;
                let wr: *mut u32 = (self.mem_buffer + idx) as *mut u32;
                unsafe {
                    wr.write_volatile(color.into());
                }
            }
        }
    }

    pub fn draw_circle(&self, ox: u32, oy: u32, rad: u32, color: &Color) {
        let diam = rad * 2;
        for x in 0..diam {
            for y in 0..diam {
                let xdiff = rad as i32 - x as i32;
                let ydiff = rad as i32 - y as i32;
                let dist = (xdiff * xdiff + ydiff * ydiff) as u32;
                if dist < rad * rad {
                    let idx = (x + ox) * 4 + (y + oy) * self.pitch;
                    let wr: *mut u32 = (self.mem_buffer + idx) as *mut u32;
                    unsafe {
                        wr.write_volatile(color.into());
                    }
                }
            }
        }
    }

    pub fn draw_circle_shaded(&self, ox: u32, oy: u32, rad: u32, color_a: &Color, color_b: &Color) {
        let diam = rad * 2;
        let pow_rad = rad * rad;
        for x in 0..diam {
            for y in 0..diam {
                let xdiff = rad as i32 - x as i32;
                let ydiff = rad as i32 - y as i32;
                let dist = (xdiff * xdiff + ydiff * ydiff) as u32;
                if dist < pow_rad {
                    let per: f64 = f64::from(dist) / f64::from(pow_rad);
                    let color = &color_a.interpolate(&color_b, per);
                    let idx = (x + ox) * 4 + (y + oy) * self.pitch;

                    let wr: *mut u32 = (self.mem_buffer + idx) as *mut u32;
                    unsafe {
                        wr.write_volatile(color.into());
                    }
                }
            }
        }
    }

    pub fn swap(&self) {
        let mut dst: *mut u32 = self.frame_buffer.pointer as *mut u32;
        let mut src: *mut u32 = self.mem_buffer as *mut u32;
        let end: *mut u32 = (self.mem_buffer + self.frame_buffer.size) as *mut u32;
        while src < end {
            unsafe {
                dst.write_volatile(src.read_volatile());
                dst = dst.add(1);
                src = src.add(1);
            }
        }
    }
}

pub fn init(width: u32, height: u32, allocator: &dyn Allocator) -> Result<Gpu, SalmiakError> {
    sprintln!("initializing GPU...");
    sprintln!("* creating {}x{} framebuffer...", width, height);

    let mut pitch = 0;
    let mut frame_buffer: FrameBuffer = Default::default();

    let mut buffer_depth = 0;
    let mut get_buffer_depth = 0;

    let mut pixel_order = 0;
    let mut get_pixel_order = 0;

    let mut physical_size: Size = Default::default();
    let mut get_physical_size: Size = Default::default();

    let mut virtual_size: Size = Default::default();
    let mut get_virtual_size: Size = Default::default();

    let mut virtual_offset: Point = Default::default();
    let mut get_virtual_offset: Point = Default::default();

    let res = MailboxPropertyBufferBuilder::new()
        .set_physical_size(width, height, Some(&mut physical_size))
        .set_virtual_size(width, height, Some(&mut virtual_size))
        .set_virtual_offset(0, 0, Some(&mut virtual_offset))
        .set_buffer_depth(32, Some(&mut buffer_depth))
        .set_pixel_order(0, Some(&mut pixel_order))
        .allocate_framebuffer(Some(&mut frame_buffer))
        .get_pitch(&mut pitch)
        .get_virtual_size(&mut get_virtual_size)
        .get_physical_size(&mut get_physical_size)
        .get_virtual_offset(&mut get_virtual_offset)
        .get_buffer_depth(&mut get_buffer_depth)
        .get_pixel_order(&mut get_pixel_order)
        .submit();

    if !res {
        return Err(SalmiakErrorKind::InitGPUError(format!(
            "Failed to allocate frame buffer of size w:{} h:{}",
            width, height
        ))
        .into());
    }

    sprintln!(
        "    SetPhys {}x{}",
        physical_size.width,
        physical_size.height
    );
    sprintln!("    SetVirt {}x{}", virtual_size.width, virtual_size.height);
    sprintln!(
        "    SetVirtOffset {}x{}",
        virtual_offset.x,
        virtual_offset.y
    );
    sprintln!("    SetBufferDepth {}", buffer_depth);
    sprintln!("    SetPixelOrder {}", pixel_order);

    sprintln!(
        "    GetPhys {}x{}",
        get_physical_size.width,
        get_physical_size.height
    );
    sprintln!(
        "    GetVirt {}x{}",
        get_virtual_size.width,
        get_virtual_size.height
    );
    sprintln!(
        "    GetVirtOffset {}x{}",
        get_virtual_offset.x,
        get_virtual_offset.y
    );
    sprintln!("    GetBufferDepth {}", get_buffer_depth);
    sprintln!("    GetPixelOrder {}", get_pixel_order);
    sprintln!("    Frame Buffer Pointer {:x}", frame_buffer.pointer);
    sprintln!(
        "    Frame Buffer Size {} MB",
        frame_buffer.size as usize / MB
    );

    sprintln!("done!");
    Ok(Gpu::new(frame_buffer, physical_size, pitch, allocator))
}
