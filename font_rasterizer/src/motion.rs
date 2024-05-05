use bitflags::bitflags;
use rand::Rng;

/// motion 仕様
/// bit
/// ---: 31 - 28 は motion type を指定
/// 31 : has_motion
/// 30 : ease_in
/// 29 : ease_out
/// 28 : loop
/// ---: 27 - 24 は motion detail を指定
/// 27 : to_current
/// 26 : turn back
/// 25 : use_distance(x)
/// 24 : use_distance(y)
/// ---: 23 - 16 は reserved
/// 23 : use_distance(xy)
/// 22 : reserved
/// 21 : reserved
/// 20 : camera detail
/// ---: 19 - 16 は easing function のタイプを指定
/// 19 : function type
/// 18 : function type
/// 17 : function type
/// 16 : function type
/// ---: 15 - 00 は motion target を指定
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

#[derive(Clone, Copy, Default, Debug)]
pub struct MotionFlags(u32);

impl MotionFlags {
    pub const ZERO_MOTION: MotionFlags = MotionFlags(0);

    pub fn new(
        motion_type: MotionType,
        motion_detail: MotionDetail,
        motion_target: MotionTarget,
        camera_detail: CameraDetail,
    ) -> MotionFlags {
        let value = (motion_type.mask() << 28)
            + ((motion_detail.bits() as u32) << 20) // 27 - 23 は motion detail が使う
            + ((camera_detail.bits() as u32) << 20) // camera detail は 20 のみ。(21, 22 は reserved)
            + (motion_type.easing_func_mask() << 16)
            + (motion_target.bits() as u32);
        MotionFlags(value)
    }

    pub fn builder() -> MotionFlagsBuilder {
        MotionFlagsBuilder::default()
    }

    pub fn random_motion() -> MotionFlags {
        let mut rng = rand::thread_rng();

        let func_type = match rng.gen_range(0..=11) {
            0 => EasingFuncType::Liner,
            1 => EasingFuncType::Sin,
            2 => EasingFuncType::Quad,
            3 => EasingFuncType::Cubic,
            4 => EasingFuncType::Quart,
            5 => EasingFuncType::Quint,
            6 => EasingFuncType::Expo,
            7 => EasingFuncType::Circ,
            8 => EasingFuncType::Back,
            9 => EasingFuncType::Elastic,
            10 => EasingFuncType::Bounce,
            _ => EasingFuncType::Liner,
        };

        let motion_type = match rng.gen_range(0..=3) {
            0 => MotionType::EaseIn(func_type, true),
            1 => MotionType::EaseOut(func_type, true),
            2 => MotionType::EaseInOut(func_type, true),
            3 => MotionType::EaseIn(func_type, true),
            _ => MotionType::None,
        };
        let motion_detail = match rng.gen_range(0..=4) {
            0 => MotionDetail::empty(),
            1 => MotionDetail::TO_CURRENT,
            2 => MotionDetail::USE_X_DISTANCE,
            3 => MotionDetail::USE_Y_DISTANCE,
            4 => MotionDetail::USE_XY_DISTANCE,
            _ => MotionDetail::empty(),
        };
        let mut motion_target = MotionTarget::empty();
        (0..4).for_each(|_| {
            motion_target |= match rng.gen_range(0..=15) {
                0 => MotionTarget::MOVE_X_PLUS,
                1 => MotionTarget::MOVE_X_MINUS,
                2 => MotionTarget::MOVE_Y_PLUS,
                3 => MotionTarget::MOVE_Y_MINUS,
                4 => MotionTarget::MOVE_Z_PLUS,
                5 => MotionTarget::MOVE_Z_MINUS,
                6 => MotionTarget::ROTATE_X_PLUS,
                7 => MotionTarget::ROTATE_X_MINUX,
                8 => MotionTarget::ROTATE_Y_PLUS,
                9 => MotionTarget::ROTATE_Y_MINUX,
                10 => MotionTarget::ROTATE_Z_PLUS,
                11 => MotionTarget::ROTATE_Z_MINUX,
                12 => MotionTarget::STRETCH_X_PLUS,
                13 => MotionTarget::STRETCH_X_MINUS,
                14 => MotionTarget::STRETCH_Y_PLUS,
                15 => MotionTarget::STRETCH_Y_MINUS,
                _ => MotionTarget::empty(),
            };
        });
        MotionFlags::new(
            motion_type,
            motion_detail,
            motion_target,
            CameraDetail::empty(),
        )
    }
}

impl From<MotionFlags> for u32 {
    fn from(value: MotionFlags) -> Self {
        value.0
    }
}
pub struct MotionFlagsBuilder {
    motion_type: MotionType,
    motion_detail: MotionDetail,
    motion_target: MotionTarget,
    camera_detail: CameraDetail,
}

impl Default for MotionFlagsBuilder {
    fn default() -> Self {
        Self {
            motion_type: MotionType::None,
            motion_detail: MotionDetail::empty(),
            motion_target: MotionTarget::empty(),
            camera_detail: CameraDetail::empty(),
        }
    }
}

impl MotionFlagsBuilder {
    pub fn motion_type(mut self, motion_type: MotionType) -> Self {
        self.motion_type = motion_type;
        self
    }

    pub fn motion_detail(mut self, motion_detail: MotionDetail) -> Self {
        self.motion_detail = motion_detail;
        self
    }

    pub fn motion_target(mut self, motion_target: MotionTarget) -> Self {
        self.motion_target = motion_target;
        self
    }

    pub fn camera_detail(mut self, camera_detail: CameraDetail) -> Self {
        self.camera_detail = camera_detail;
        self
    }

    pub fn build(self) -> MotionFlags {
        MotionFlags::new(
            self.motion_type,
            self.motion_detail,
            self.motion_target,
            self.camera_detail,
        )
    }
}

pub enum MotionType {
    None,
    // 第二引数はループするかどうか
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
        const TO_CURRENT      = 0b_1000_0000;
        const TURN_BACK       = 0b_0100_0000;
        const USE_X_DISTANCE  = 0b_0010_0000;
        const USE_Y_DISTANCE  = 0b_0001_0000;
        const USE_XY_DISTANCE = 0b_0000_1000;
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

bitflags! {
    pub struct CameraDetail: u8 {
        const IGNORE_CAMERA  = 0b_0000_0001;
    }
}

#[cfg(test)]
mod test {
    use crate::motion::{
        CameraDetail, EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType,
    };

    #[test]
    fn motion() {
        let flags = MotionFlags::new(
            MotionType::EaseOut(EasingFuncType::Bounce, false),
            MotionDetail::empty(),
            MotionTarget::MOVE_X_MINUS | MotionTarget::MOVE_Y_MINUS,
            CameraDetail::IGNORE_CAMERA,
        );
        println!("{:#034b}", flags.0);
    }
}
