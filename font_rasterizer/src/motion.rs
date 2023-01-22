struct VertexMotion {
    ty: MotionGroup,
}

enum MotionGroup {
    Oneshot(OneShotMotionType),
    Loop(LoopMotionType),
}

// ワンショットで動く
enum OneShotMotionType {
    

}

enum LoopMotionType {}
