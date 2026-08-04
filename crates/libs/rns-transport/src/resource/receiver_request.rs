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

/// Why a fragment request is being built.
///
/// Only the trigger differs between these; what gets asked for is the same
/// window either way. The reference tops the window up on an advertisement,
/// on a hashmap update and on a watchdog retry, but on the receive path only
/// once the round has drained (`elif self.outstanding_parts == 0:`,
/// `RNS/Resource.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestTrigger {
    /// An advertisement, a hashmap update, or a retry: fill the window now.
    Immediate,
    /// A fragment just arrived. Asking again before the round has drained
    /// would carry the single slot that fragment just vacated — which is one
    /// request packet and one round trip per fragment, and is what made the
    /// window ladder above meaningless in bandwidth terms however high it
    /// climbed.
    PartReceived,
}

impl ResourceReceiver {
    /// One more fragment landed. The round is over when everything asked for
    /// has arrived — the reference's `outstanding_parts == 0`.
    ///
    /// An earlier version of this counted a window's worth of fragments
    /// instead, because with the window topped up on every arriving part the
    /// pipeline never drained and this condition never fired. That was a
    /// workaround for the refill strategy, not for the reference's design;
    /// now that `build_request` asks for a whole window per round, the drain
    /// is observable and is what the ladder should key on.
    fn note_fragment_received(&mut self, now: Instant) {
        if self.in_flight_set.is_empty() {
            self.note_round_complete(now);
        }
    }

    /// Every fragment asked for in the last round has arrived: widen the
    /// window by one, and measure the link while we have a clean interval to
    /// measure over.
    ///
    /// The reference does this at exactly the same point — after a part lands
    /// and `outstanding_parts` reaches zero (`RNS/Resource.py`). Growing by
    /// one per successful round rather than by a ratio is deliberate: it is
    /// what the reference does, and a resource transfer is short enough that
    /// a slow-start curve would spend most of the transfer still ramping.
    fn note_round_complete(&mut self, now: Instant) {
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
        if self.window > self.window_min {
            self.window -= 1;
            if self.window_max > self.window_min {
                self.window_max -= 1;
                // `saturating_sub`, not `-`: a window carried over from a
                // previous resource on this link can legitimately start above
                // `window_max` (see the restore in `manager.rs`), and the
                // reference reaches the same comparison with a negative value
                // where Rust would underflow and panic.
                if self.window_max.saturating_sub(self.window) > WINDOW_FLEXIBILITY - 1 {
                    self.window_max -= 1;
                }
            }
        }
    }

    /// How long to wait, with no part arriving at all, before treating the
    /// outstanding window as lost.
    ///
    /// This is an *idle* timer, not a per-fragment one, and it is measured
    /// from the last part that arrived rather than from when each fragment
    /// was asked for. That is the reference's shape (`RNS/Resource.py`'s
    /// watchdog: `last_activity + part_timeout_factor * expected_tof_remaining
    /// + RETRY_GRACE_TIME`).
    ///
    /// Two properties of that shape are load-bearing:
    ///
    /// * it **scales with the outstanding window**, because a sender serves
    ///   fragments in sequence — the tail of a window of 60 cannot arrive
    ///   within the same budget as the tail of a window of 4. A threshold
    ///   that is constant in window size makes a large window declare its own
    ///   tail lost on a perfect link, and every such "loss" ratchets the
    ///   ceiling down permanently.
    /// * it has an **absolute floor**. On a fast link the measured interval
    ///   is a fraction of a millisecond, and any purely rate-derived budget
    ///   collapses toward zero. The reference floors it at `RETRY_GRACE_TIME`
    ///   for the same reason `hashmap_update_wait` above has a floor.
    fn part_timeout(&self, rtt: Duration, arrival_interval: Duration) -> Duration {
        /// `PART_TIMEOUT_FACTOR_AFTER_RTT` in the reference.
        const FACTOR: u32 = 2;
        /// `RETRY_GRACE_TIME` in the reference.
        const GRACE: Duration = Duration::from_millis(250);
        let outstanding = self.in_flight_set.len().max(1) as u32;
        arrival_interval
            .saturating_mul(outstanding)
            .max(rtt)
            .saturating_mul(FACTOR)
            .saturating_add(GRACE)
    }

    fn build_request(
        &mut self,
        now: Instant,
        rtt: Duration,
        arrival_interval: Duration,
        trigger: RequestTrigger,
    ) -> ResourceRequest {
        // Prune fragments that have already landed. The queue is in send
        // order, so received entries collect at the front.
        while let Some(&(_, idx)) = self.in_flight_queue.front() {
            if self.parts[idx].is_some() {
                self.in_flight_set.remove(&idx);
                self.in_flight_queue.pop_front();
            } else {
                break;
            }
        }

        // One shrink per failed round, not one per fragment. A window that
        // times out wholesale is a single failed round — shrinking once per
        // fragment would collapse straight to the floor and throw away
        // everything the link had proven.
        let mut lost_this_round = false;

        // Nothing has arrived for the whole idle budget: give up on the
        // entire outstanding set at once and re-queue it, as the reference's
        // watchdog does. Declaring fragments lost individually, on a timer
        // that did not scale with the window, is what generated the
        // loss events that walked the window ladder into its floor.
        if !self.in_flight_set.is_empty()
            && now.duration_since(self.last_progress) > self.part_timeout(rtt, arrival_interval)
        {
            // Re-queue in ascending index order so the re-request stays as
            // close to the sender's serving anchor as possible: a reference
            // sender only serves parts at or above
            // `receiver_min_consecutive_height` and silently drops anything
            // below it, so the oldest gap is the one that must go first.
            let mut lost: Vec<usize> = self.in_flight_queue.iter().map(|&(_, idx)| idx).collect();
            lost.sort_unstable();
            for idx in lost.into_iter().rev() {
                self.request_queue.push_front(idx);
            }
            self.in_flight_queue.clear();
            self.in_flight_set.clear();
            lost_this_round = true;
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

        // Fill the window. Lost fragments are at the front of request_queue
        // (pushed there above) so they get priority over new fragments.
        //
        // Whether a request should be built at all is the *caller's*
        // decision, and it is where the batching lives: the reference tops
        // the window up on an advertisement, on a hashmap update and on a
        // watchdog retry, but on the receive path only when the round has
        // drained (`elif self.outstanding_parts == 0:`, `RNS/Resource.py`).
        // Rebuilding on every arriving part instead is what made
        // `window - in_flight` almost always 1 — measured over a real
        // transfer, 15,689 of 16,919 requests carried a single hash for
        // 39,809 fragments, one request packet and one round trip each.
        let window_space = if trigger == RequestTrigger::PartReceived
            && !self.in_flight_set.is_empty()
        {
            0
        } else {
            self.window.saturating_sub(self.in_flight_set.len())
        };
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
