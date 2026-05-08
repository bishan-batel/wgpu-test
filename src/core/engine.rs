use std::slice;

use crate::core::server::Server;

pub struct Engine {
    servers: Vec<Box<dyn Server>>,
    servers_lock: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            servers: vec![],
            servers_lock: false,
        }
    }

    pub fn add_server(&mut self, server: Box<dyn Server>) -> eyre::Result<()> {
        if self.servers_lock {
            eyre::bail!("Server have been locked");
        }

        self.servers.push(server);
        Ok(())
    }

    pub fn start(&mut self) -> eyre::Result<()> {
        self.servers_lock = true;

        for server in self.servers_mut() {
            server.setup();
        }

        Ok(())
    }

    pub fn servers(&self) -> slice::Iter<'_, Box<dyn Server>> {
        self.servers.iter()
    }

    pub fn servers_mut(&mut self) -> slice::IterMut<'_, Box<dyn Server>> {
        self.servers.iter_mut()
    }
}
