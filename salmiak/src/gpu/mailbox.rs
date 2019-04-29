use core::sync::atomic::{compiler_fence, Ordering};

//////////////////////////////////////////////////////
//                Mailbox Registers                 //
//////////////////////////////////////////////////////
// Mailbox  Read/Write  Peek  Sender  Status  Config//
// 0        0x00        0x10  0x14    0x18    0x1c  //
// 1        0x20        0x30  0x34    0x38    0x3c  //
//////////////////////////////////////////////////////

mod offset {
    pub const MAILBOX: u32 = 0x3F00_B880;
    pub const READ: u32 = 0x0000_0000;
    pub const WRITE: u32 = 0x0000_0020;
    pub const STATUS: u32 = 0x0000_0018;
}

mod status {
    pub const FULL: u32 = 0x8000_0000;
    pub const EMPTY: u32 = 0x4000_0000;
    pub const SUCCESS: u32 = 0x8000_0000;
}

mod tags {
    pub const SET_CLOCK_RATE: u32 = 0x0003_8002;
    pub const GET_CLOCK_RATE: u32 = 0x0003_0002;
    pub const SET_PHYSICAL_SIZE: u32 = 0x0004_8003;
    pub const GET_PHYSICAL_SIZE: u32 = 0x0004_0003;
    pub const SET_VIRTUAL_SIZE: u32 = 0x0004_8004;
    pub const GET_VIRTUAL_SIZE: u32 = 0x0004_0004;
    pub const SET_VIRTUAL_OFFSET: u32 = 0x0004_8009;
    pub const GET_VIRTUAL_OFFSET: u32 = 0x0004_0009;
    pub const SET_BUFFER_DEPTH: u32 = 0x0004_8005;
    pub const GET_BUFFER_DEPTH: u32 = 0x0004_0005;
    pub const SET_PIXEL_ORDER: u32 = 0x0004_8006;
    pub const GET_PIXEL_ORDER: u32 = 0x0004_0006;
    pub const ALLOCATE_FRAME_BUFFER: u32 = 0x0004_0001;
    pub const GET_PITCH: u32 = 0x0004_0008;
    pub const GET_ARM_MEMORY: u32 = 0x0001_0005;
}

mod property_buffer {
    pub const FIELD_COUNT_OFFSET: usize = 2;
    pub const SIZE: usize = 128;
}

// different clock constants
pub mod clock {
    pub const UART: u32 = 0x2;
}

// These structs mainly exist to make sense to whoever is using the interface
#[derive(Debug, Default)]
pub struct FrameBuffer {
    pub pointer: u32,
    pub size: u32,
}

#[derive(Debug, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Default)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Point {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Default)]
pub struct ClockRate {
    pub id: u32,
    pub hz: u32,
}

#[derive(Debug, Default)]
pub struct ARMMemory {
    pub base_address: usize,
    pub size: usize,
}

impl ARMMemory {
    pub fn new(base_address: usize, size: usize) -> Self {
        Self { base_address, size }
    }
}

#[derive(Debug)]
struct ResultReader<'a> {
    tp: MailboxResult<'a>,
    offset: u32,
}

#[derive(Debug)]
pub enum MailboxResult<'a> {
    SingleU32 {
        first: Option<&'a mut u32>,
    },
    Size {
        size: Option<(&'a mut Size)>,
    },
    Point {
        point: Option<(&'a mut Point)>,
    },
    ClockRate {
        clock_rate: Option<(&'a mut ClockRate)>,
    },
    FrameBuffer {
        frame_buffer: Option<&'a mut FrameBuffer>,
    },
    ARMMemory {
        arm_memory: Option<&'a mut ARMMemory>,
    },
    Nothing,
}

impl<'a> Default for ResultReader<'a> {
    fn default() -> Self {
        ResultReader {
            tp: MailboxResult::Nothing,
            offset: 0,
        }
    }
}

#[repr(align(16))]
pub struct MailboxPropertyBufferBuilder<'a> {
    mbox: [u32; property_buffer::SIZE],
    field_count: usize,
    results: [ResultReader<'a>; 16],
    result_count: usize,
}

impl<'a> Default for MailboxPropertyBufferBuilder<'a> {
    fn default() -> MailboxPropertyBufferBuilder<'a> {
        let mut boxbuffer = MailboxPropertyBufferBuilder {
            result_count: 0,
            field_count: property_buffer::FIELD_COUNT_OFFSET,
            results: Default::default(),
            mbox: [0; property_buffer::SIZE],
        };

        boxbuffer.mbox[1] = 0; // this is a request
        boxbuffer
    }
}

impl<'a> MailboxPropertyBufferBuilder<'a> {
    const M_READ: *mut u32 = (offset::MAILBOX + offset::READ) as *mut u32;
    const M_WRITE: *mut u32 = (offset::MAILBOX + offset::WRITE) as *mut u32;
    const M_STATUS: *mut u32 = (offset::MAILBOX + offset::STATUS) as *mut u32;

    fn add_result_reader(&mut self, tp: MailboxResult<'a>, offset: u32) {
        self.results[self.result_count] = ResultReader { tp, offset };
        self.result_count += 1;
    }

    pub fn new() -> MailboxPropertyBufferBuilder<'a> {
        Default::default()
    }

    pub fn set_clock_rate(
        &mut self,
        clock_id: u32,
        rate: u32,
        skip_turbo: u32,
        result_clock_rate: Option<&'a mut ClockRate>,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_CLOCK_RATE;
        self.mbox[self.field_count + 1] = 12;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = clock_id;
        self.mbox[self.field_count + 4] = rate;
        self.mbox[self.field_count + 5] = skip_turbo;

        if result_clock_rate.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(
                MailboxResult::ClockRate {
                    clock_rate: result_clock_rate,
                },
                offset as u32,
            );
        }

        self.field_count += 6;
        self
    }

    pub fn get_clock_rate(
        &mut self,
        clock_id: u32,
        result_clock_rate: &'a mut ClockRate,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_CLOCK_RATE;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = clock_id;
        self.mbox[self.field_count + 4] = 0; // hz will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::ClockRate {
                clock_rate: Some(result_clock_rate),
            },
            offset as u32,
        );

        self.field_count += 5;
        self
    }

    pub fn set_physical_size(
        &mut self,
        width: u32,
        height: u32,
        result_size: Option<&'a mut Size>,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_PHYSICAL_SIZE;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = width; // width -> width result
        self.mbox[self.field_count + 4] = height; // height -> height result

        if result_size.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(MailboxResult::Size { size: result_size }, offset as u32);
        }

        self.field_count += 5;
        self
    }

    pub fn get_physical_size(&mut self, result_size: &'a mut Size) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_PHYSICAL_SIZE;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // width will be written here
        self.mbox[self.field_count + 4] = 0; // height will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::Size {
                size: Some(result_size),
            },
            offset as u32,
        );

        self.field_count += 5;
        self
    }

    pub fn set_virtual_size(
        &mut self,
        width: u32,
        height: u32,
        result_size: Option<&'a mut Size>,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_VIRTUAL_SIZE;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = width; // width -> width result
        self.mbox[self.field_count + 4] = height; // height -> height result

        if result_size.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(MailboxResult::Size { size: result_size }, offset as u32);
        }

        self.field_count += 5;
        self
    }

    pub fn get_virtual_size(&mut self, result_size: &'a mut Size) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_VIRTUAL_SIZE;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // width will be written here
        self.mbox[self.field_count + 4] = 0; // height will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::Size {
                size: Some(result_size),
            },
            offset as u32,
        );

        self.field_count += 5;
        self
    }

    pub fn set_virtual_offset(
        &mut self,
        x: u32,
        y: u32,
        result_point: Option<&'a mut Point>,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_VIRTUAL_OFFSET;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = x; // x offset -> x offset result
        self.mbox[self.field_count + 4] = y; // y offset -> y offset result

        if result_point.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(
                MailboxResult::Point {
                    point: result_point,
                },
                offset as u32,
            );
        }

        self.field_count += 5;
        self
    }

    pub fn get_virtual_offset(&mut self, result_point: &'a mut Point) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_VIRTUAL_OFFSET;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // x offset will be written here
        self.mbox[self.field_count + 4] = 0; // y offset will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::Point {
                point: Some(result_point),
            },
            offset as u32,
        );

        self.field_count += 5;
        self
    }

    pub fn set_buffer_depth(&mut self, depth: u32, result_depth: Option<&'a mut u32>) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_BUFFER_DEPTH;
        self.mbox[self.field_count + 1] = 4;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = depth;

        if result_depth.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(
                MailboxResult::SingleU32 {
                    first: result_depth,
                },
                offset as u32,
            );
        }

        self.field_count += 4;
        self
    }

    pub fn get_buffer_depth(&mut self, result_depth: &'a mut u32) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_BUFFER_DEPTH;
        self.mbox[self.field_count + 1] = 4;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // depth will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::SingleU32 {
                first: Some(result_depth),
            },
            offset as u32,
        );

        self.field_count += 4;
        self
    }

    pub fn set_pixel_order(
        &mut self,
        pixel_order: u32,
        res_p_order: Option<&'a mut u32>,
    ) -> &mut Self {
        self.mbox[self.field_count] = tags::SET_PIXEL_ORDER;
        self.mbox[self.field_count + 1] = 4;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = pixel_order;

        if res_p_order.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(
                MailboxResult::SingleU32 { first: res_p_order },
                offset as u32,
            );
        }

        self.field_count += 4;
        self
    }

    pub fn get_pixel_order(&mut self, res_p_order: &'a mut u32) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_PIXEL_ORDER;
        self.mbox[self.field_count + 1] = 4;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // pixel order will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::SingleU32 {
                first: Some(res_p_order),
            },
            offset as u32,
        );

        self.field_count += 4;
        self
    }

    pub fn allocate_framebuffer(&mut self, frame_buffer: Option<&'a mut FrameBuffer>) -> &mut Self {
        self.mbox[self.field_count] = tags::ALLOCATE_FRAME_BUFFER;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 4096; // alignment -> pointer to framebuffer
        self.mbox[self.field_count + 4] = 0; // FrameBufferInfo.size

        if frame_buffer.is_some() {
            let offset = self.field_count + 3;
            self.add_result_reader(MailboxResult::FrameBuffer { frame_buffer }, offset as u32);
        }

        self.field_count += 5;
        self
    }

    pub fn get_pitch(&mut self, pitch: &'a mut u32) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_PITCH;
        self.mbox[self.field_count + 1] = 4;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // pitch will be written here

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::SingleU32 { first: Some(pitch) },
            offset as u32,
        );

        self.field_count += 4;
        self
    }

    pub fn get_arm_memory(&mut self, result_memory: &'a mut ARMMemory) -> &mut Self {
        self.mbox[self.field_count] = tags::GET_ARM_MEMORY;
        self.mbox[self.field_count + 1] = 8;
        self.mbox[self.field_count + 2] = 0;
        self.mbox[self.field_count + 3] = 0; // base address
        self.mbox[self.field_count + 4] = 0; // memory size bytes

        let offset = self.field_count + 3;
        self.add_result_reader(
            MailboxResult::ARMMemory {
                arm_memory: Some(result_memory),
            },
            offset as u32,
        );

        self.field_count += 5;
        self
    }

    pub fn submit(&mut self) -> bool {
        const MAILBOX_PROPERTY_CHANNEL: u32 = 8;
        self.mbox[self.field_count] = 0x0; // end of tags
        self.mbox[0] = ((self.field_count + 1) * 4) as u32;

        // make sure all data is written to buffer
        compiler_fence(Ordering::Release);

        let mbox_ptr = (&self.mbox as *const u32) as u32;
        assert!(mbox_ptr.trailing_zeros() >= 4);
        self.mailbox_write(mbox_ptr, MAILBOX_PROPERTY_CHANNEL);
        while self.mailbox_read(MAILBOX_PROPERTY_CHANNEL) != mbox_ptr {}

        // was it successful?
        if self.mbox[1] != status::SUCCESS {
            return false;
        }

        for r in 0..self.result_count {
            let result_offset = self.results[r].offset;
            let res_tp = &mut self.results[r].tp;
            match res_tp {
                MailboxResult::SingleU32 { first } => {
                    if let Some(x) = first {
                        **x = self.mbox[result_offset as usize];
                    }
                }
                MailboxResult::Size { size } => {
                    if let Some(s) = size {
                        s.width = self.mbox[result_offset as usize];
                        s.height = self.mbox[(result_offset + 1) as usize];
                    }
                }
                MailboxResult::Point { point } => {
                    if let Some(p) = point {
                        p.x = self.mbox[result_offset as usize];
                        p.y = self.mbox[(result_offset + 1) as usize];
                    }
                }
                MailboxResult::ClockRate { clock_rate } => {
                    if let Some(c) = clock_rate {
                        c.id = self.mbox[result_offset as usize];
                        c.hz = self.mbox[(result_offset + 1) as usize];
                    }
                }
                MailboxResult::FrameBuffer { frame_buffer } => {
                    if let Some(b) = frame_buffer {
                        b.pointer = self.mbox[result_offset as usize];
                        b.size = self.mbox[(result_offset + 1) as usize];
                    }
                }
                MailboxResult::ARMMemory { arm_memory } => {
                    if let Some(mem) = arm_memory {
                        mem.base_address = self.mbox[result_offset as usize] as usize;
                        mem.size = self.mbox[(result_offset + 1) as usize] as usize;
                    }
                }
                MailboxResult::Nothing => {
                    panic!("TODO: We should error (better) here");
                }
            }
        }

        true
    }

    #[cfg(not(test))]
    fn mailbox_write(&mut self, data: u32, channel: u32) {
        // wait for space
        unsafe {
            while MailboxPropertyBufferBuilder::M_STATUS.read_volatile() & status::FULL != 0 {}
            MailboxPropertyBufferBuilder::M_WRITE.write_volatile(data | channel);
        }
    }

    #[cfg(not(test))]
    fn mailbox_read(&mut self, channel: u32) -> u32 {
        loop {
            unsafe {
                // wait for content
                while MailboxPropertyBufferBuilder::M_STATUS.read_volatile() & status::EMPTY != 0 {}
                let val = MailboxPropertyBufferBuilder::M_READ.read_volatile();

                // First byte is the channel (0xF = 1111).
                if (val & 0xF) == channel {
                    // The rest if the bytes (3) is the value (0xFFFF_FFF0 = 1...0000)
                    return val & 0xFFFF_FFF0;
                }
            }
        }
    }

    #[cfg(test)]
    fn mailbox_write(&mut self, _data: u32, _channel: u32) {
        // Fake successful mailbox submit
        self.mbox[1] = status::SUCCESS
    }

    #[cfg(test)]
    fn mailbox_read(&mut self, _channel: u32) -> u32 {
        // Of course we read everything from the mailbox.
        (&self.mbox as *const u32) as u32
    }

    #[cfg(test)]
    pub fn get_field_count(&self) -> usize {
        self.field_count - property_buffer::FIELD_COUNT_OFFSET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Results

    #[test]
    fn single_u32_result() {
        const DEPTH: u32 = 5;
        let mut depth: u32 = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_buffer_depth(&mut depth);

        let fc = res.field_count;
        res.mbox[fc - 1] = DEPTH;
        res.submit();

        assert_eq!(depth, DEPTH);
    }

    #[test]
    fn size_result() {
        const WIDTH: u32 = 10;
        const HEIGHT: u32 = 20;
        let mut size: Size = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_physical_size(&mut size);

        let fc = res.field_count;

        // Size is written on the last two fields
        res.mbox[fc - 2] = WIDTH;
        res.mbox[fc - 1] = HEIGHT;

        res.submit();

        assert_eq!(size.width, WIDTH);
        assert_eq!(size.height, HEIGHT);
    }

    #[test]
    fn point_result() {
        const X: u32 = 1000;
        const Y: u32 = 300;
        let mut point: Point = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_virtual_offset(&mut point);

        let fc = res.field_count;

        // Size is written on the last two fields
        res.mbox[fc - 2] = X;
        res.mbox[fc - 1] = Y;

        res.submit();

        assert_eq!(point.x, X);
        assert_eq!(point.y, Y);
    }

    #[test]
    fn clock_rate_result() {
        const ID: u32 = 5;
        const HZ: u32 = 1234;
        let mut clock: ClockRate = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_clock_rate(ID, &mut clock);

        let fc = res.field_count;
        res.mbox[fc - 1] = HZ;
        res.submit();

        assert_eq!(clock.id, ID);
        assert_eq!(clock.hz, HZ);
    }

    #[test]
    fn framebuffer_result() {
        const POINTER: u32 = 12;
        const SIZE: u32 = 65;
        let mut framebuffer: FrameBuffer = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.allocate_framebuffer(Some(&mut framebuffer));

        let fc = res.field_count;
        res.mbox[fc - 2] = POINTER;
        res.mbox[fc - 1] = SIZE;
        res.submit();

        assert_eq!(framebuffer.pointer, POINTER);
        assert_eq!(framebuffer.size, SIZE);
    }

    #[test]
    #[should_panic]
    fn nothing_result() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.add_result_reader(MailboxResult::Nothing, 0);
        res.submit();
    }

    // Methods
    #[test]
    fn set_clock_rate() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_clock_rate(1, 2, 3, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 6);

        assert_eq!(res.mbox[2], tags::SET_CLOCK_RATE); // dest address
        assert_eq!(res.mbox[3], 12); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 1);
        assert_eq!(res.mbox[6], 2);
        assert_eq!(res.mbox[7], 3);
    }

    #[test]
    fn get_clock_rate() {
        let mut clock_rate: ClockRate = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_clock_rate(clock::UART, &mut clock_rate);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::GET_CLOCK_RATE); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], clock::UART);
    }

    #[test]
    fn set_physical_size() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_physical_size(100, 200, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::SET_PHYSICAL_SIZE); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 100);
        assert_eq!(res.mbox[6], 200);
    }

    #[test]
    fn get_physical_size() {
        let mut size: Size = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_physical_size(&mut size);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::GET_PHYSICAL_SIZE); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn set_virtual_size() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_virtual_size(300, 400, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::SET_VIRTUAL_SIZE); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 300);
        assert_eq!(res.mbox[6], 400);
    }

    #[test]
    fn get_virtual_size() {
        let mut size: Size = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_virtual_size(&mut size);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::GET_VIRTUAL_SIZE); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn set_virtual_offset() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_virtual_offset(10, 20, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::SET_VIRTUAL_OFFSET); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 10);
        assert_eq!(res.mbox[6], 20);
    }

    #[test]
    fn get_virtual_offset() {
        let mut point: Point = Default::default();
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_virtual_offset(&mut point);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::GET_VIRTUAL_OFFSET); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn set_buffer_depth() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_buffer_depth(5, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 4);
        assert_eq!(res.mbox[2], tags::SET_BUFFER_DEPTH); // dest address
        assert_eq!(res.mbox[3], 4); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 5);
    }

    #[test]
    fn get_buffer_depth() {
        let mut depth: u32 = 0;
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_buffer_depth(&mut depth);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 4);
        assert_eq!(res.mbox[2], tags::GET_BUFFER_DEPTH); // dest address
        assert_eq!(res.mbox[3], 4); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn set_pixel_order() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.set_pixel_order(50, None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 4);
        assert_eq!(res.mbox[2], tags::SET_PIXEL_ORDER); // dest address
        assert_eq!(res.mbox[3], 4); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something

        // arguments
        assert_eq!(res.mbox[5], 50);
    }

    #[test]
    fn get_pixel_order() {
        let mut order: u32 = 0;
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_pixel_order(&mut order);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 4);
        assert_eq!(res.mbox[2], tags::GET_PIXEL_ORDER); // dest address
        assert_eq!(res.mbox[3], 4); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn allocate_framebuffer() {
        let mut res = MailboxPropertyBufferBuilder::new();
        res.allocate_framebuffer(None);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::ALLOCATE_FRAME_BUFFER); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn get_pitch() {
        let mut pitch: u32 = 0;
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_pitch(&mut pitch);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 4);
        assert_eq!(res.mbox[2], tags::GET_PITCH); // dest address
        assert_eq!(res.mbox[3], 4); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

    #[test]
    fn get_arm_memory() {
        let mut arm_mem: ARMMemory = Default::default();;
        let mut res = MailboxPropertyBufferBuilder::new();
        res.get_arm_memory(&mut arm_mem);

        // expected field count
        // tag + req + reserved + max(req_vals, resp_vals)
        assert_eq!(res.get_field_count(), 5);
        assert_eq!(res.mbox[2], tags::GET_ARM_MEMORY); // dest address
        assert_eq!(res.mbox[3], 8); // request length
        assert_eq!(res.mbox[4], 0); // reserved for something
    }

}
