use bitflags::bitflags;

/// motion 仕様
/// bit
/// 31 : has_motion
/// 30 : ease_in
/// 29 : ease_out
/// 28 : loop
/// ----
/// 27 : to_current
/// 26 : use_distance
/// 25 :
/// 24 :
/// ----
/// 23 :
/// 22 :
/// 21 :
/// 20 :
/// ----
/// 19 : 19 - 16 は easing function のタイプを指定
/// 18 :
/// 17 :
/// 16 :
/// ----
/// 15 : STRETCH_Y_MINUS
/// 14 : STRETCH_Y_PLUS
/// 13 : STRETCH_X_MINUS
/// 12 : STRETCH_X_PLUS
/// ----
/// 11 : ROTATE_Z_MINUS
/// 10 : ROTATE_Z_PLUS
/// 09 : ROTATE_Y_MINUS
/// 08 : ROTATE_Y_PLUS
/// ----
/// 07 : ROTATE_X_MINUS
/// 06 : ROTATE_X_PLUS
/// 05 : MOVE_Z_MINUS
/// 04 : MOVE_Z_PLUS
/// ----
/// 03 : MOVE_Y_MINUS
/// 02 : MOVE_Y_PLUS
/// 01 : MOVE_X_MINUS
/// 00 : MOVE_X_PLUS
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
        motion_target: MotionTarget,
    ) -> MotionFlags {
        let value = (motion_type.mask() << 28)
            + ((motion_detail.bits() as u32) << 24)
            + (motion_type.easing_func_mask() << 16)
            + (motion_target.bits() as u32);
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
        u32::from(l)
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
        const TO_CURRENT      = 0b_0000_1000;
        const USE_X_DISTANCE  = 0b_0000_0100;
        const USE_Y_DISTANCE  = 0b_0000_0010;
        const USE_XY_DISTANCE = 0b_0000_0001;
    }
}

bitflags! {
    pub struct MotionTarget: u16 {
        const MOVE_X_PLUS     = 0b_0000_0000_0000_0001;
        const MOVE_X_MINUS    = 0b_0000_0000_0000_0010;
        const MOVE_Y_PLUS     = 0b_0000_0000_0000_0100;
        const MOVE_Y_MINUS    = 0b_0000_0000_0000_1000;
        const MOVE_Z_PLUS     = 0b_0000_0000_0001_0000;
        const MOVE_Z_MINUS    = 0b_0000_0000_0010_0000;
        const ROTATE_X_PLUS   = 0b_0000_0000_0100_0000;
        const ROTATE_X_MINUX  = 0b_0000_0000_1000_0000;
        const ROTATE_Y_PLUS   = 0b_0000_0001_0000_0000;
        const ROTATE_Y_MINUX  = 0b_0000_0010_0000_0000;
        const ROTATE_Z_PLUS   = 0b_0000_0100_0000_0000;
        const ROTATE_Z_MINUX  = 0b_0000_1000_0000_0000;
        const STRETCH_X_PLUS  = 0b_0001_0000_0000_0000;
        const STRETCH_X_MINUS = 0b_0010_0000_0000_0000;
        const STRETCH_Y_PLUS  = 0b_0100_0000_0000_0000;
        const STRETCH_Y_MINUS = 0b_1000_0000_0000_0000;
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
            MotionTarget::MOVE_X_MINUS | MotionTarget::MOVE_Y_MINUS,
        );
        println!("{:#034b}", flags.0);
    }
}
