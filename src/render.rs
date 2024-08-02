pub mod vitepress;

use crate::processor::Processor;

pub trait Renderer {
    type Output;

    fn render(&mut self, processor: Processor) -> Self::Output;
}
