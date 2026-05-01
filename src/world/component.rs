use std::fmt::Debug;

pub trait Component: Debug {
    fn name(&self) {}
}
