use std::f32::consts::PI;

use ahrs::{Ahrs, Madgwick};
use anyhow::Ok;
use cgmath::{Quaternion, Rad, Rotation3};
use hidapi::HidApi;

const ROKID_VENDOR_ID: u16 = 0x04D2;
const ROKID_MAX_PRODUCT_ID: u16 = 0x162F;

pub struct RokidMax {
    device: hidapi::HidDevice,
    ahrs: Madgwick<f32>,
}

impl RokidMax {
    pub fn new() -> anyhow::Result<Self> {
        let hid_api = HidApi::new()?;
        let device = hid_api.open(ROKID_VENDOR_ID, ROKID_MAX_PRODUCT_ID)?;
        let mut result = Self {
            device,
            ahrs: new_madgwick(),
        };
        result.reset()?;
        Ok(result)
    }

    pub fn reset(&mut self) -> anyhow::Result<()> {
        self.ahrs = new_madgwick();
        self.update()?;
        Ok(())
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        match self.read_packet()? {
            RokidMaxPacket::Combined(packet) => {
                let g = packet.gyroscope().into();
                let a = packet.accelerometer().into();
                let m = packet.magnetometer().into();
                self.ahrs.update(&g, &a, &m).unwrap();
            }
            _ => { /* noop */ }
        }
        Ok(())
    }

    fn read(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer: [u8; 128] = [0; 128];
        let size = self.device.read(&mut buffer)?;
        Ok(buffer[0..size].to_vec())
    }

    pub fn quaternion(&self) -> Quaternion<f32> {
        let quat_vec = self.ahrs.quat.as_vector();
        // とりあえずつじつまが合うように補正は入れたが、正しいかは不明
        Quaternion::new(-quat_vec[1], -quat_vec[2], -quat_vec[3], quat_vec[0])
            * Quaternion::from_angle_y(Rad(PI))
    }

    pub fn read_packet(&self) -> anyhow::Result<RokidMaxPacket> {
        let buffer = self.read()?;
        let packet = buffer_to_packet(&buffer)?;
        Ok(packet)
    }
}

fn new_madgwick() -> Madgwick<f32> {
    Madgwick::new(1.0 / 60.0, 0.01)
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
