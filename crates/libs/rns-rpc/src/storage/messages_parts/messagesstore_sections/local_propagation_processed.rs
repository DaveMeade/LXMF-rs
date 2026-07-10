impl MessagesStore {
    pub fn mark_local_propagation_processed_at(
        &self,
        transient_id: &str,
        processed_at: i64,
    ) -> rusqlite::Result<bool> {
        self.with_write_conn(|conn| {
            let affected = conn.execute(
                "INSERT OR IGNORE INTO propagation_local_entries
                    (transient_id, processed_at)
                 VALUES (?1, ?2)",
                params![normalize_hex_key(transient_id), processed_at],
            )?;
            Ok(affected > 0)
        })
    }

    pub fn prune_local_propagation_processed_before(
        &self,
        cutoff_ts: i64,
    ) -> rusqlite::Result<Vec<String>> {
        self.with_write_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT transient_id
                 FROM propagation_local_entries
                 WHERE processed_at < ?1
                 ORDER BY processed_at ASC, transient_id ASC",
            )?;
            let expired = stmt
                .query_map(params![cutoff_ts], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(stmt);

            for transient_id in &expired {
                conn.execute(
                    "DELETE FROM propagation_local_entries WHERE transient_id = ?1",
                    params![transient_id],
                )?;
            }

            Ok(expired)
        })
    }

    pub fn prune_expired_local_propagation_processed(
        &self,
        now_ts: i64,
    ) -> rusqlite::Result<Vec<String>> {
        self.prune_local_propagation_processed_before(
            now_ts.saturating_sub(LXMF_LOCAL_TRANSIENT_CACHE_EXPIRY_SECS),
        )
    }
}
