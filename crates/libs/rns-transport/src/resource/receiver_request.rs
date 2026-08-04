// Building the next fragment request for an inbound resource.
//
// Split out of `receiver.rs` to keep both files inside the repository's
// 500-line budget; `ResourceReceiver` itself is defined there. Both are
// `include!`d into the same `resource` module, so this is a file boundary
// rather than a privacy one.

/// How long to wait for a requested hashmap update before asking again.
///
/// The reference derives this from the link's measured bitrate
/// (`HMU_WAIT_FACTOR`, `RNS/Resource.py`); this uses round-trip time, which
/// this receiver already measures per fragment and which tracks the same
/// thing well enough for a backstop. The floor matters more than the
/// factor: on a fast link RTT can be a millisecond, and re-requesting the
/// map that often is the behaviour this bound exists to prevent.
fn hashmap_update_wait(rtt: Duration) -> Duration {
    const FACTOR: u32 = 4;
    const FLOOR: Duration = Duration::from_secs(2);
    rtt.saturating_mul(FACTOR).max(FLOOR)
}

impl ResourceReceiver {
    /// Every fragment asked for in the last round has arrived: widen the
    /// window by one, and measure the link while we have a clean interval
    /// to measure over.
    ///
    /// The reference does this at exactly the same point — after a part
    /// lands and `outstanding_parts` reaches zero (`RNS/Resource.py`).
    /// Growing by one per successful round rather than by a ratio is
    /// deliberate: it is what the reference does, and a resource transfer
    /// is short enough that a slow-start curve would spend most of the
    /// transfer still ramping.
    /// One more fragment landed. A full window's worth without a loss is
    /// this implementation's equivalent of the reference's drained round —
    /// see `fragments_since_window_change` for why the drain itself is not
    /// observable here.
    fn note_fragment_received(&mut self, now: Instant) {
        self.fragments_since_window_change += 1;
        if self.fragments_since_window_change >= self.window {
            self.note_round_complete(now);
        }
    }

    fn note_round_complete(&mut self, now: Instant) {
        self.fragments_since_window_change = 0;
        if self.window < self.window_max {
            self.window += 1;
            // Once the window has pulled far enough ahead of its floor, the
            // floor follows. A link that has proven itself over many rounds
            // should not fall all the way back to two on one timeout.
            if self.window - self.window_min > WINDOW_FLEXIBILITY - 1 {
                self.window_min += 1;
            }
        }

        let elapsed = now.duration_since(self.round_started_at).as_secs_f64();
        let transferred = self.received_bytes.saturating_sub(self.round_start_bytes);
        self.round_started_at = now;
        self.round_start_bytes = self.received_bytes;
        if elapsed <= 0.0 {
            return;
        }

        let rate = transferred as f64 / elapsed;
        if rate > RATE_FAST && self.fast_rate_rounds < FAST_RATE_THRESHOLD {
            self.fast_rate_rounds += 1;
            if self.fast_rate_rounds == FAST_RATE_THRESHOLD {
                self.window_max = WINDOW_MAX_FAST;
            }
        }
        // `fast_rate_rounds == 0` guard, from the reference: a link that has
        // ever measured fast is never demoted to the very-slow ceiling on a
        // single bad interval.
        if self.fast_rate_rounds == 0
            && rate < RATE_VERY_SLOW
            && self.very_slow_rate_rounds < VERY_SLOW_RATE_THRESHOLD
        {
            self.very_slow_rate_rounds += 1;
            if self.very_slow_rate_rounds == VERY_SLOW_RATE_THRESHOLD {
                self.window_max = WINDOW_MAX_VERY_SLOW;
            }
        }
    }

    /// Fragments were asked for and never arrived: narrow the window, and
    /// pull the ceiling down with it so the next few successful rounds do
    /// not immediately climb back to a size this link has just shown it
    /// cannot sustain. Mirrors the reference's retry path.
    fn note_fragments_lost(&mut self) {
        self.fragments_since_window_change = 0;
        if self.window > self.window_min {
            self.window -= 1;
            if self.window_max > self.window_min {
                self.window_max -= 1;
                if self.window_max - self.window > WINDOW_FLEXIBILITY - 1 {
                    self.window_max -= 1;
                }
            }
        }
    }

    fn build_request(&mut self, now: Instant, rtt: Duration) -> ResourceRequest {
        // TODO: the loss threshold (2×rtt) and EWMA alpha (7/8) are intuition-based
        // and have not been formally tuned or proven. On links with high jitter the
        // 2×rtt multiplier may be too tight (causing spurious re-requests); on links
        // with asymmetric delay it may be too loose. The EWMA alpha controls how
        // quickly the estimate tracks changes — a higher alpha (closer to 1) gives
        // more weight to history and reacts more slowly to sudden changes. Both
        // values should be validated against real-world Reticulum traffic traces.
        let loss_threshold = rtt.saturating_mul(2);

        // One shrink per round of losses, not one per fragment. A window of
        // four that times out wholesale is a single failed round — shrinking
        // four times would collapse straight to the floor and throw away
        // everything the link had proven. The reference shrinks once per
        // retry for the same reason.
        let mut lost_this_round = false;

        // Drain the front of in_flight_queue (front = oldest, since we append in time order).
        // Received entries are lazily pruned; entries older than 2×rtt are declared lost
        // and pushed to the front of request_queue for priority re-request.
        loop {
            match self.in_flight_queue.front() {
                None => break,
                Some(&(sent_at, idx)) => {
                    if self.parts[idx].is_some() {
                        self.in_flight_set.remove(&idx);
                        self.in_flight_queue.pop_front();
                    } else if now.duration_since(sent_at) > loss_threshold {
                        self.in_flight_set.remove(&idx);
                        self.in_flight_queue.pop_front();
                        self.request_queue.push_front(idx);
                        lost_this_round = true;
                    } else {
                        break;
                    }
                }
            }
        }

        if lost_this_round {
            self.note_fragments_lost();
        }

        // A hashmap update is outstanding, so there is nothing to ask for
        // yet: asking again would only re-request the same segment.
        //
        // The wait is bounded. A lost update must not park the transfer
        // forever — and it would, silently: `retry_count` only advances when
        // a request is actually sent, so a permanently-gated receiver is
        // never even declared failed. The reference clears the same flag
        // from its watchdog for the same reason (`RNS/Resource.py`).
        if self.waiting_for_hashmap_update {
            if now.duration_since(self.last_request) < hashmap_update_wait(rtt) {
                return ResourceRequest {
                    hashmap_exhausted: false,
                    last_map_hash: None,
                    resource_hash: self.resource_hash,
                    requested_hashes: Vec::new(),
                };
            }
            log::debug!(
                "[resource-diag] hashmap_update_wait_expired hash={} link={} height={}",
                self.resource_hash,
                self.link_id,
                self.hashmap_height
            );
            self.waiting_for_hashmap_update = false;
        }

        // Detect hashmap exhaustion **within the current window** — i.e. is
        // the next fragment this transfer needs still unmapped?
        //
        // Not "is any slot anywhere unmapped", which is true from the first
        // request until the very last segment arrives and turns every round
        // into a map re-request. The reference scans exactly
        // `parts[consecutive_completed_height + 1 ..][..window]`
        // (`RNS/Resource.py`, `request_next`).
        let scan_end = self.consecutive_completed_height.saturating_add(self.window).min(self.parts.len());
        let mut hashmap_exhausted = false;
        for idx in self.consecutive_completed_height..scan_end {
            if self.parts[idx].is_none() && self.hashmap[idx].is_none() {
                hashmap_exhausted = true;
                break;
            }
        }
        // The last hash we hold. The reference sender walks its own parts
        // from `receiver_min_consecutive_height` until it matches this, and
        // **cancels the whole transfer** if the resulting index is not a
        // multiple of the hashmap segment length — so this has to be the end
        // of a segment, which the last filled slot always is.
        let last_known = self.hashmap_height.checked_sub(1).and_then(|idx| self.hashmap[idx]);
        if hashmap_exhausted {
            self.waiting_for_hashmap_update = true;
        }

        // Fill available window slots. Lost fragments are at the front of request_queue
        // (pushed there above) so they get priority over new fragments.
        let window_space = self.window.saturating_sub(self.in_flight_set.len());
        let mut requested = Vec::new();
        while requested.len() < window_space {
            match self.request_queue.pop_front() {
                None => break,
                Some(idx) => {
                    if self.parts[idx].is_none() && !self.in_flight_set.contains_key(&idx) {
                        if let Some(hash) = self.hashmap[idx] {
                            requested.push(hash);
                            self.in_flight_set.insert(idx, now);
                            self.in_flight_queue.push_back((now, idx));
                        }
                    }
                    // Received or already in-flight — skip.
                }
            }
        }

        // A round is the interval between asking for fragments and having
        // them all arrive, so the clock starts when the request does — and
        // only when there is something in it to wait for.
        if !requested.is_empty() {
            self.round_started_at = now;
            self.round_start_bytes = self.received_bytes;
        }

        // Logged here rather than at the call sites: the manager builds
        // requests from three places and only the advertisement one was
        // logged, so every round after the first was invisible. Diagnosing
        // the stall this function now fixes meant reading raw packet
        // contexts to infer what the receiver had asked for.
        log::debug!(
            "[resource-diag] request_built hash={} link={} requested={} exhausted={} in_flight={} consecutive={}/{} mapped={} window={}/{}",
            self.resource_hash,
            self.link_id,
            requested.len(),
            hashmap_exhausted,
            self.in_flight_set.len(),
            self.consecutive_completed_height,
            self.parts.len(),
            self.hashmap_height,
            self.window,
            self.window_max
        );

        ResourceRequest {
            hashmap_exhausted,
            last_map_hash: if hashmap_exhausted { last_known } else { None },
            resource_hash: self.resource_hash,
            requested_hashes: requested,
        }
    }
}
