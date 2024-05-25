use std::{
    thread::{self},
    time::Duration,
};

use rokid_3dof::{RokidMax, RokidMaxPacket};

fn main() -> anyhow::Result<()> {
    let mut rokid_max = RokidMax::new()?;

    loop {
        //rokid_max.update()?;
        match rokid_max.read_packet()? {
            RokidMaxPacket::Combined(packet) => {
                println!("{:?}", packet.magnetometer());
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(1000));
    }
    //Ok(())
}
