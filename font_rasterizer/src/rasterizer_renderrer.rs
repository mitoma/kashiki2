pub use crate::windfoil_renderrer::RasterizerRenderrer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineFillRule {
    EvenOdd,
    NonZero,
}
