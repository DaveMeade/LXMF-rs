impl DaemonConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }

    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml(&contents)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    pub fn reticulum_transport_enabled(&self) -> bool {
        self.reticulum_enable_transport
    }

    pub fn enabled_tcp_clients(&self) -> Vec<&InterfaceConfig> {
        self.interfaces
            .iter()
            .filter(|iface| iface.enabled() && iface.kind == "tcp_client")
            .collect()
    }

    pub fn tcp_client_endpoints(&self) -> Vec<(String, u16)> {
        self.enabled_tcp_clients()
            .iter()
            .filter_map(|iface| {
                let host = iface.host.as_ref()?;
                let port = iface.port?;
                Some((host.clone(), port))
            })
            .collect()
    }

    pub fn enabled_tcp_servers(&self) -> Vec<&InterfaceConfig> {
        self.interfaces
            .iter()
            .filter(|iface| iface.enabled() && iface.kind == "tcp_server")
            .collect()
    }

    pub fn tcp_server_endpoints(&self) -> Vec<(String, u16)> {
        self.enabled_tcp_servers()
            .iter()
            .filter_map(|iface| {
                let port = iface.port?;
                let host = iface
                    .host
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("0.0.0.0")
                    .to_string();
                Some((host, port))
            })
            .collect()
    }
}
