impl InterfaceManager {
    pub fn detach_interfaces(&mut self) -> usize {
        let detached = self.ifaces.len();
        for interface in &self.ifaces {
            interface.stop.cancel();
        }
        self.cleanup();
        detached
    }

    pub fn prioritize_interfaces(&mut self) {
        self.ifaces.sort_by(|left, right| {
            right.announce_bitrate_bps.cmp(&left.announce_bitrate_bps)
        });
    }
}
