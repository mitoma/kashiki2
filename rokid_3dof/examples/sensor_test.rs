use std::{
    thread::{self},
    time::Duration,
};

use rokid_3dof::{RokidMax, RokidMaxError};

fn main() -> Result<(), RokidMaxError> {
    let rokid_max = RokidMax::new()?;
    loop {
        println!("{:?}", rokid_max.quaternion());

        thread::sleep(Duration::from_millis(1000));
    }
}
