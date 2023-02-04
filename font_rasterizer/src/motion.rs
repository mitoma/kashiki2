use bitflags::bitflags;

/// motion 仕様
/// bit
/// 31 : has_motion
/// 30 : ease_in
/// 29 : ease_out
/// 28 : loop
/// ----
/// 27 : set_minus
/// 26 : to_current
/// 25 : use_distance
/// 24 :
/// ----
/// 23 : 23 - 20 は easing function のタイプを指定
/// 22 :
/// 21 :
/// 20 :
/// ----
/// 19 : 19 - 12 は duration を 0 ms - 2550 ms までの範囲で指定
/// 18 :
/// 17 :
/// 16 :
/// ----
/// 15 :
/// 14 :
/// 13 :
/// 12 :
/// ----
/// 11 : 11 - 08 は gain を 0倍 - 4倍 まで(0.25刻み)
/// 10 :
/// 09 :
/// 08 :
/// ----
/// 07 : STRETCH_Y
/// 06 : STRETCH_X
/// 05 : ROTATE_Z
/// 04 : ROTATE_Y
/// ----
/// 03 : ROTATE_X
/// 02 : MOVE_Z
/// 01 : MOVE_Y
/// 00 : MOVE_X
///
/// easing function
/// 0000: liner
/// 0001: sin
/// 0010: quad
/// 0011: cubic
/// 0100: quart
/// 0101: quint
/// 0110: expo
/// 0111: circ
/// 1000: back
/// 1001: elastic
/// 1010: bounce
/// 1011: (unused)
/// 1100: (unused)
/// 1101: (unused)
/// 1110: (unused)
/// 1111: (unused)

#[derive(Clone, Copy)]
pub struct MotionFlags(u32);

impl MotionFlags {
    pub const ZERO_MOTION: MotionFlags = MotionFlags(0);

    pub fn new(
        motion_type: MotionType,
        motion_detail: MotionDetail,
        duration: u8,
        gain: u8,
        motion_target: MotionTarget,
    ) -> MotionFlags {
        let gain = if gain >= 16 { 15 } else { gain };

        let value = (motion_type.mask() << 28)
            + ((motion_detail.bits as u32) << 24)
            + (motion_type.easing_func_mask() << 20)
            + ((duration as u32) << 12)
            + ((gain as u32) << 8)
            + (motion_target.bits as u32);
        MotionFlags(value)
    }
}

impl From<MotionFlags> for u32 {
    fn from(value: MotionFlags) -> Self {
        value.0
    }
}

pub enum MotionType {
    None,
    EaseIn(EasingFuncType, bool),
    EaseOut(EasingFuncType, bool),
    EaseInOut(EasingFuncType, bool),
}

impl MotionType {
    fn mask(&self) -> u32 {
        match self {
            MotionType::None => 0b_0000_0000,
            MotionType::EaseIn(_, l) => 0b_0000_1100 + Self::loopbit(*l),
            MotionType::EaseOut(_, l) => 0b_0000_1010 + Self::loopbit(*l),
            MotionType::EaseInOut(_, l) => 0b_0000_1110 + Self::loopbit(*l),
        }
    }

    fn loopbit(l: bool) -> u32 {
        if l {
            1
        } else {
            0
        }
    }

    fn easing_func_mask(&self) -> u32 {
        match self {
            MotionType::EaseIn(func, _) => func.mask(),
            MotionType::EaseOut(func, _) => func.mask(),
            MotionType::EaseInOut(func, _) => func.mask(),
            _ => 0,
        }
    }
}

pub enum EasingFuncType {
    Liner,
    Sin,
    Quad,
    Cubic,
    Quart,
    Quint,
    Expo,
    Circ,
    Back,
    Elastic,
    Bounce,
}
impl EasingFuncType {
    fn mask(&self) -> u32 {
        match self {
            EasingFuncType::Liner => 0,
            EasingFuncType::Sin => 1,
            EasingFuncType::Quad => 2,
            EasingFuncType::Cubic => 3,
            EasingFuncType::Quart => 4,
            EasingFuncType::Quint => 5,
            EasingFuncType::Expo => 6,
            EasingFuncType::Circ => 7,
            EasingFuncType::Back => 8,
            EasingFuncType::Elastic => 9,
            EasingFuncType::Bounce => 10,
        }
    }
}

bitflags! {
    pub struct MotionDetail :u8{
        const SET_MINUS    = 0b_0000_1000;
        const TO_CURRENT   = 0b_0000_0100;
        const USE_DISTANCE = 0b_0000_0010;
    }
}

bitflags! {
    pub struct MotionTarget: u8 {
        const MOVE_X =      0b_0000_0001;
        const MOVE_Y =      0b_0000_0010;
        const MOVE_Z =      0b_0000_0100;
        const ROTATE_X =    0b_0000_1000;
        const ROTATE_Y =    0b_0001_0000;
        const ROTATE_Z =    0b_0010_0000;
        const STRETCH_X =   0b_0100_0000;
        const STRETCH_Y =   0b_1000_0000;
    }
}

#[cfg(test)]
mod test {
    use crate::motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType};

    #[test]
    fn motion() {
        let flags = MotionFlags::new(
            MotionType::EaseOut(EasingFuncType::Bounce, false),
            MotionDetail::empty(),
            100,
            1,
            MotionTarget::MOVE_X | MotionTarget::MOVE_Y,
        );
        println!("{:#034b}", flags.0);
    }
}
