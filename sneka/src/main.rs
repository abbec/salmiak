#![cfg_attr(target_arch = "aarch64", no_main)]
#![cfg_attr(target_arch = "aarch64", no_std)]

#[cfg(target_arch = "aarch64")]
#[macro_use]
extern crate salmiak;

#[cfg(target_arch = "aarch64")]
mod entry {
    use salmiak::gpu;
    use salmiak::memory::{
        alloc::{BumpAllocator, *},
        MB,
    };
    use salmiak::serial;

    entry!(boot);

    fn boot() -> ! {
        sprintln!("----- S.N.E.K.A -----");

        let gpu_allocator: BumpAllocator = create_child_allocator(None, 2 * MB);
        let gpu = gpu::init(640, 480, &gpu_allocator).unwrap();

        let mut ypos = 150;
        let mut xpos = 150;
        let move_dt = 10;

        loop {
            gpu.clear_screen(&gpu::Color::BLACK);
            gpu.draw_rectangle(356, 300, 100, 20, &gpu::Color::BLUE);

            if let Some(c) = serial::readchar() {
                match c as char {
                    'a' => xpos -= move_dt,
                    'd' => xpos += move_dt,
                    'w' => ypos -= move_dt,
                    's' => ypos += move_dt,
                    'r' => salmiak::power::reset(),
                    _ => (),
                };
            }

            //draw super snek
            for i in 0..15 {
                let x = xpos + i * 7;
                let y = ypos + i * 7;

                gpu.draw_circle_shaded(x, y, 10, &gpu::Color::RED, &gpu::Color::GREEN);
            }

            gpu.swap();
            // do game stuff
        }
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn main() -> Result<(), String> {
    println!("😡 You should not run this on the host platform");
    Ok(())
}
