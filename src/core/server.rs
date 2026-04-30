use bitflags::bitflags;

bitflags! {
    pub struct ServerTickFlags: u8 {
        const PreTick = 1;
        const Tick = 1 << 2;
        const PostTick = 1 << 1;
    }
}
/// Global singleton
pub trait Server {
    fn setup(&mut self) {}

    fn pre_tick(&mut self) {}

    fn tick(&mut self) {}

    fn post_tick(&mut self) {}

    fn flags(&self) -> ServerTickFlags {
        return ServerTickFlags::all();
    }

    fn ia_alive(&self) -> bool {
        true
    }
}
