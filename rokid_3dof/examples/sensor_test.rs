use std::{
    thread::{self},
    time::Duration,
};

use rokid_3dof::RokidMax;

fn main() -> anyhow::Result<()> {
    let rokid_max = RokidMax::new()?;
    loop {
        println!("{:?}", rokid_max.quaternion());

        thread::sleep(Duration::from_millis(1000));
    }
}
