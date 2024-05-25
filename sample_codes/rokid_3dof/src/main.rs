use std::{
    thread::{self},
    time::Duration,
};

use rokid_3dof::{RokidMax, RokidMaxPacket};

fn main() -> anyhow::Result<()> {
    let rokid_max = RokidMax::new()?;

    loop {
        let packet = rokid_max.read_packet()?;
        match packet {
            RokidMaxPacket::Misc(packet) => {
                println!("{:?}", packet);
            }
            RokidMaxPacket::Sensor(packet) => {
                println!("{:?}", packet);
            }
            RokidMaxPacket::Combined(packet) => {
                println!(
                    "display:{:?}, volume:{:?}, proxy:{:?}, accel:{:?}",
                    packet.display_brightness(),
                    packet.volume(),
                    packet.proxy_sensor(),
                    packet.accelerometer(),
                );
            }
        }
        println!("quatanion:{:?}", rokid_max.gyroscope_quaternion());
        thread::sleep(Duration::from_millis(1000));
    }
    //Ok(())
}
