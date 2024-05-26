use std::{
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Ok;
use cgmath::{Quaternion, Rotation};
use hidapi::HidApi;
use vqf_rs::VQF;

const ROKID_VENDOR_ID: u16 = 0x04D2;
const ROKID_MAX_PRODUCT_ID: u16 = 0x162F;

pub struct RokidMax {
    ahrs: Arc<Mutex<VQF>>,
}

impl RokidMax {
    pub fn new() -> anyhow::Result<Self> {
        let hid_api = HidApi::new()?;
        let device = hid_api.open(ROKID_VENDOR_ID, ROKID_MAX_PRODUCT_ID)?;
        let mut result = Self {
            ahrs: Arc::new(Mutex::new(new_vqf())),
        };
        result.reset()?;
        let ahrs = result.ahrs.clone();
        let _thread = thread::spawn(move || loop {
            let packet = read_packet(&device).unwrap();
            match packet {
                RokidMaxPacket::Combined(packet) => {
                    let mut vqf = ahrs.lock().unwrap();
                    update_vqf(&mut vqf, packet);
                }
                _ => { /* noop */ }
            }
            thread::sleep(std::time::Duration::from_millis(10));
        });
        Ok(result)
    }

    pub fn reset(&mut self) -> anyhow::Result<()> {
        self.ahrs.lock().unwrap().reset_state();
        Ok(())
    }

    pub fn quaternion(&self) -> Quaternion<f32> {
        let quat_vec = self.ahrs.lock().unwrap().quat_3d();
        Quaternion::new(quat_vec.0, quat_vec.1, quat_vec.2, quat_vec.3).invert()
    }
}

fn update_vqf(vqf: &mut VQF, packet: CombinedPacket) {
    vqf.update(
        packet.gyroscope(),
        packet.accelerometer(),
        Some(packet.magnetometer()),
    );
}

fn read_packet(device: &hidapi::HidDevice) -> anyhow::Result<RokidMaxPacket> {
    let mut buffer: [u8; 128] = [0; 128];
    let size = device.read(&mut buffer)?;
    let buffer = &buffer[0..size];
    let packet = buffer_to_packet(buffer)?;
    Ok(packet)
}

fn new_vqf() -> VQF {
    VQF::new(1.0 / 100.0, None, None, None)
}

fn buffer_to_packet(buffer: &[u8]) -> anyhow::Result<RokidMaxPacket> {
    let packet_type = buffer[0];
    let packet = match packet_type {
        2 => {
            let packet = bytemuck::from_bytes::<MiscPacket>(buffer);
            RokidMaxPacket::Misc(packet.to_owned())
        }
        4 => {
            let packet = bytemuck::from_bytes::<SensorPacket>(buffer);
            RokidMaxPacket::Sensor(packet.to_owned())
        }
        17 => {
            let packet = bytemuck::from_bytes::<CombinedPacket>(buffer);
            RokidMaxPacket::Combined(packet.to_owned())
        }
        _ => {
            anyhow::bail!("Unknown packet type: {}", packet_type);
        }
    };
    Ok(packet)
}

pub enum RokidMaxPacket {
    Misc(MiscPacket),
    Sensor(SensorPacket),
    // 実際に飛んでくるのはほぼ CombinedPacket のみなので、このパケットの場合だけ処理すれば十分そうだ
    Combined(CombinedPacket),
}

// https://github.com/badicsalex/ar-drivers-rs/ から引用

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MiscPacket {
    packet_type: u8,
    seq: u32,
    _unknown_0: [u8; 42],
    keys_pressed: u8,
    _unknown_1: [u8; 3],
    proxy_sensor: u8,
    _unknown_2: [u8; 12],
}

unsafe impl bytemuck::Zeroable for MiscPacket {}
unsafe impl bytemuck::Pod for MiscPacket {}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SensorPacket {
    packet_type: u8,
    sensor_type: u8,
    seq: u32,
    _unknown_0: [u8; 3],
    timestamp: u64,
    _unknown_1: [u8; 4],
    vector: [f32; 3],
    _unknown_2: [u8; 31],
}

unsafe impl bytemuck::Zeroable for SensorPacket {}
unsafe impl bytemuck::Pod for SensorPacket {}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CombinedPacket {
    packet_type: u8,
    timestamp: u64,
    accelerometer: [f32; 3],
    // ジャイロ
    gyroscope: [f32; 3],
    // 地磁気センサー
    magnetometer: [f32; 3],
    keys_pressed: u8,
    // 近接センサー。グラスを装着している時が 0 、装着していない時が 1
    proxy_sensor: u8,
    _unknown_0: u8,
    vsync_timestamp: u64,
    _unknown_1: [u8; 3],
    display_brightness: u8,
    volume: u8,
    _unknown_2: [u8; 3],
}

unsafe impl bytemuck::Zeroable for CombinedPacket {}
unsafe impl bytemuck::Pod for CombinedPacket {}

impl CombinedPacket {
    pub fn display_brightness(&self) -> u8 {
        self.display_brightness
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    // グラスを装着している時に true を返す
    pub fn proxy_sensor(&self) -> bool {
        self.proxy_sensor == 0
    }

    pub fn gyroscope(&self) -> [f32; 3] {
        self.gyroscope
    }

    pub fn accelerometer(&self) -> [f32; 3] {
        self.accelerometer
    }

    pub fn magnetometer(&self) -> [f32; 3] {
        self.magnetometer
    }
}
