fn build_mesh_client_config(node_index: usize, transport_ports: &[u16]) -> String {
    let node_count = transport_ports.len();
    let next = (node_index + 1) % node_count;
    format!(
        "[reticulum]\nenable_transport = true\n\n[[interfaces]]\ntype = \"tcp_client\"\nenabled = true\nhost = \"127.0.0.1\"\nport = {}\n",
        transport_ports[next]
    )
}

#[cfg(test)]
mod tests {
    include!("scenario_mesh_tests.rs");
}
