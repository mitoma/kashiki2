use anyhow::Ok;
use cgmath::{InnerSpace, Quaternion};
use hidapi::HidApi;

const ROKID_VENDOR_ID: u16 = 0x04D2;
const ROKID_MAX_PRODUCT_ID: u16 = 0x162F;

pub struct RokidMax {
    device: hidapi::HidDevice,
    initial: SensorData,
    current: SensorData,
    fifo: Vec<SensorData>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SensorData {
    pub gyroscope: [f32; 3],
    pub accelerometer: [f32; 3],
    pub magnetometer: [f32; 3],
}

impl RokidMax {
    pub fn new() -> anyhow::Result<Self> {
        let hid_api = HidApi::new()?;
        let device = hid_api.open(ROKID_VENDOR_ID, ROKID_MAX_PRODUCT_ID)?;
        let mut result = Self {
            device,
            initial: SensorData::default(),
            current: SensorData::default(),
            fifo: Vec::new(),
        };
        result.reset()?;
        Ok(result)
    }

    pub fn reset(&mut self) -> anyhow::Result<()> {
        match self.read_packet()? {
            RokidMaxPacket::Combined(packet) => {
                self.fifo.clear();
                let sensor_data = SensorData {
                    gyroscope: packet.gyroscope(),
                    accelerometer: packet.accelerometer(),
                    magnetometer: packet.magnetometer(),
                };
                self.initial = sensor_data.clone();
                self.current = sensor_data.clone();
                self.fifo.push(sensor_data);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        match self.read_packet()? {
            RokidMaxPacket::Combined(packet) => {
                while self.fifo.len() >= 20 {
                    self.fifo.remove(0);
                }
                let sensor_data = SensorData {
                    gyroscope: packet.gyroscope(),
                    accelerometer: packet.accelerometer(),
                    magnetometer: packet.magnetometer(),
                };
                self.current = sensor_data.clone();
                self.fifo.push(sensor_data);
            }
            _ => {}
        }
        Ok(())
    }

    fn read(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer: [u8; 128] = [0; 128];
        let size = self.device.read(&mut buffer)?;
        Ok(buffer[0..size].to_vec())
    }

    pub fn gyroscope_quaternion(&self) -> Quaternion<f32> {
        Quaternion::from_arc(
            self.initial.gyroscope.into(),
            self.current.gyroscope.into(),
            None,
        )
        .normalize()
    }

    pub fn gyroscope_quaternion_avg(&self) -> Quaternion<f32> {
        let sum = self.fifo.iter().fold([0.0, 0.0, 0.0], |init, sensor| {
            [
                init[0] + sensor.gyroscope[0],
                init[1] + sensor.gyroscope[1],
                init[2] + sensor.gyroscope[2],
            ]
        });
        let len = self.fifo.len() as f32;
        let sum = [sum[0] / len, sum[1] / len, sum[2] / len];
        Quaternion::from_arc(self.initial.gyroscope.into(), sum.into(), None).normalize()
    }

    pub fn accelerometer_quaternion(&self) -> Quaternion<f32> {
        Quaternion::from_arc(
            self.initial.accelerometer.into(),
            self.current.accelerometer.into(),
            None,
        )
        .normalize()
    }

    pub fn accelerometer_quaternion_avg(&self) -> Quaternion<f32> {
        let sum = self.fifo.iter().fold([0.0, 0.0, 0.0], |init, sensor| {
            [
                init[0] + sensor.accelerometer[0],
                init[1] + sensor.accelerometer[1],
                init[2] + sensor.accelerometer[2],
            ]
        });
        let len = self.fifo.len() as f32;
        let sum = [sum[0] / len, sum[1] / len, sum[2] / len];
        Quaternion::from_arc(self.initial.accelerometer.into(), sum.into(), None).normalize()
    }

    pub fn magnetometer_quaternion(&self) -> Quaternion<f32> {
        Quaternion::from_arc(
            self.initial.magnetometer.into(),
            self.current.magnetometer.into(),
            None,
        )
        .normalize()
    }

    pub fn magnetometer_quaternion_avg(&self) -> Quaternion<f32> {
        let sum = self.fifo.iter().fold([0.0, 0.0, 0.0], |init, sensor| {
            [
                init[0] + sensor.magnetometer[0],
                init[1] + sensor.magnetometer[1],
                init[2] + sensor.magnetometer[2],
            ]
        });
        let len = self.fifo.len() as f32;
        let sum = [sum[0] / len, sum[1] / len, sum[2] / len];
        Quaternion::from_arc(self.initial.magnetometer.into(), sum.into(), None).normalize()
    }

    pub fn read_packet(&self) -> anyhow::Result<RokidMaxPacket> {
        let buffer = self.read()?;
        let packet = buffer_to_packet(&buffer)?;
        Ok(packet)
    }
}

fn buffer_to_packet(buffer: &[u8]) -> anyhow::Result<RokidMaxPacket> {
    let packet_type = buffer[0];
    let packet = match packet_type {
        2 => {
            let packet = bytemuck::from_bytes::<MiscPacket>(&buffer);
            RokidMaxPacket::Misc(packet.to_owned())
        }
        4 => {
            let packet = bytemuck::from_bytes::<SensorPacket>(&buffer);
            RokidMaxPacket::Sensor(packet.to_owned())
        }
        17 => {
            let packet = bytemuck::from_bytes::<CombinedPacket>(&buffer);
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
