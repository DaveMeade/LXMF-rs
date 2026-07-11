use crate::hash::{AddressHash, Hash};
use crate::packet::Packet;
use rmpv::Value;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedPacket {
    pub packet: Packet,
    pub interface_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReticulumPacketDiskCache {
    root: PathBuf,
}

impl ReticulumPacketDiskCache {
    pub fn new(reticulum_storage_path: impl AsRef<Path>) -> Self {
        Self { root: reticulum_storage_path.as_ref().join("cache") }
    }

    pub async fn write(
        &self,
        packet: &Packet,
        interface_reference: Option<&str>,
        announce: bool,
    ) -> io::Result<Hash> {
        let hash = packet.hash();
        let path = self.path(hash, announce);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let raw = packet
            .to_bytes()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "encode cached packet"))?;
        let interface =
            interface_reference.map_or(Value::Nil, |value| Value::from(value.to_owned()));
        let value = Value::Array(vec![Value::Binary(raw), interface]);
        let mut payload = Vec::new();
        rmpv::encode::write_value(&mut payload, &value)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "encode cached packet"))?;
        tokio::fs::write(path, payload).await?;
        Ok(hash)
    }

    pub async fn read(&self, hash: Hash, announce: bool) -> io::Result<Option<CachedPacket>> {
        let payload = match tokio::fs::read(self.path(hash, announce)).await {
            Ok(payload) => payload,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let value = rmpv::decode::read_value(&mut io::Cursor::new(payload))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let fields = value.as_array().filter(|fields| fields.len() >= 2).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid packet cache row")
        })?;
        let raw = match &fields[0] {
            Value::Binary(raw) => raw.as_slice(),
            Value::String(raw) => raw.as_str().map(str::as_bytes).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid packet bytes")
            })?,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid packet bytes")),
        };
        let packet = Packet::from_bytes(raw)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "decode cached packet"))?;
        let interface_reference = if fields[1].is_nil() {
            None
        } else {
            Some(
                fields[1]
                    .as_str()
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid interface reference")
                    })?
                    .to_owned(),
            )
        };
        Ok(Some(CachedPacket { packet, interface_reference }))
    }

    pub async fn clean_announces(
        &self,
        active_path_hashes: &[Hash],
        tunnel_path_hashes: &[Hash],
    ) -> io::Result<usize> {
        let directory = self.root.join("announces");
        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
        };
        let keep = active_path_hashes
            .iter()
            .chain(tunnel_path_hashes)
            .map(|hash| hex::encode(hash.as_slice()))
            .collect::<BTreeSet<_>>();
        let mut removed = 0;
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            if file_type.is_file() && (hex::decode(&name).is_err() || !keep.contains(&name)) {
                tokio::fs::remove_file(entry.path()).await?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub async fn save_packet_hashlist(
        &self,
        storage_path: impl AsRef<Path>,
        hashes: &[Hash],
    ) -> io::Result<()> {
        let value = Value::Array(
            hashes.iter().map(|hash| Value::Binary(hash.as_slice().to_vec())).collect(),
        );
        let mut payload = Vec::new();
        rmpv::encode::write_value(&mut payload, &value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        tokio::fs::write(storage_path.as_ref().join("packet_hashlist"), payload).await
    }

    pub fn interface_hash(reference: &str) -> Option<AddressHash> {
        AddressHash::new_from_hex_string(reference).ok()
    }

    fn path(&self, hash: Hash, announce: bool) -> PathBuf {
        let directory = if announce { self.root.join("announces") } else { self.root.clone() };
        directory.join(hex::encode(hash.as_slice()))
    }
}
