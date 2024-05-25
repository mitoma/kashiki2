use std::{
    thread::{self},
    time::Duration,
};

use rokid_3dof::{RokidMax, RokidMaxPacket};

fn main() -> anyhow::Result<()> {
    let mut rokid_max = RokidMax::new()?;

    loop {
        rokid_max.update()?;
        println!("{:?}", rokid_max.quaternion());

        thread::sleep(Duration::from_millis(1000));
    }
    //Ok(())
}
