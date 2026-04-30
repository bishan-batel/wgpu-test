use std::slice;

use crate::core::server::Server;

pub struct Engine {
    servers: Vec<&'static mut dyn Server>,
    servers_lock: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            servers: vec![],
            servers_lock: false,
        }
    }

    pub fn add_server<T>(&mut self, server: &'static mut dyn Server) -> eyre::Result<()>
    where
        T: Server + Sized + 'static,
    {
        if self.servers_lock {
            eyre::bail!("Server have been locked");
        }

        self.servers.push(server);
        Ok(())
    }

    pub fn start(&mut self) -> eyre::Result<()> {
        Ok(())
    }

    pub fn servers(&self) -> slice::Iter<'_, &'static mut dyn Server> {
        self.servers.iter()
    }

    pub fn servers_mut(&mut self) -> slice::IterMut<'_, &'static mut dyn Server> {
        self.servers.iter_mut()
    }
}
