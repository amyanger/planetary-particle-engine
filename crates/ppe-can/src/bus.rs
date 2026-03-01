use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::trace;

use crate::{CanFilter, CanFrame};

/// A virtual CAN bus that broadcasts frames to all subscribers.
#[derive(Clone)]
pub struct VirtualCanBus {
    ingest_tx: Sender<CanFrame>,
    subscribers: Arc<Mutex<Vec<Sender<CanFrame>>>>,
}

impl VirtualCanBus {
    /// Create a new VirtualCanBus with the given ingest channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (ingest_tx, ingest_rx) = bounded::<CanFrame>(capacity);
        let subscribers: Arc<Mutex<Vec<Sender<CanFrame>>>> = Arc::new(Mutex::new(Vec::new()));

        let subs = subscribers.clone();
        std::thread::Builder::new()
            .name("can-bus-router".into())
            .spawn(move || {
                while let Ok(frame) = ingest_rx.recv() {
                    trace!(id = %frame.id, dlc = frame.dlc(), "routing CAN frame");
                    let subs = subs.lock().unwrap();
                    // Retain only senders that haven't disconnected
                    for sub in subs.iter() {
                        let _ = sub.try_send(frame.clone());
                    }
                }
            })
            .expect("failed to spawn CAN bus router thread");

        Self {
            ingest_tx,
            subscribers,
        }
    }

    /// Create a new BusNode connected to this bus.
    pub fn connect(&self, filter: CanFilter, rx_capacity: usize) -> BusNode {
        let (tx, rx) = bounded(rx_capacity);
        {
            let mut subs = self.subscribers.lock().unwrap();
            subs.push(tx);
        }
        BusNode {
            send_tx: self.ingest_tx.clone(),
            recv_rx: rx,
            filter,
        }
    }
}

/// A node connected to the virtual CAN bus.
/// Sends frames to the bus and receives filtered frames.
pub struct BusNode {
    send_tx: Sender<CanFrame>,
    recv_rx: Receiver<CanFrame>,
    filter: CanFilter,
}

impl BusNode {
    /// Send a frame onto the CAN bus.
    pub fn send(&self, frame: CanFrame) -> Result<(), ppe_core::PpeError> {
        self.send_tx
            .send(frame)
            .map_err(|e| ppe_core::PpeError::CanBus(format!("send failed: {e}")))
    }

    /// Try to receive a frame (non-blocking). Returns None if no matching frame available.
    pub fn try_recv(&self) -> Option<CanFrame> {
        loop {
            match self.recv_rx.try_recv() {
                Ok(frame) => {
                    if self.filter.matches(frame.id) {
                        return Some(frame);
                    }
                    // Frame didn't match filter, try next
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            }
        }
    }

    /// Receive with a timeout. Returns None on timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<CanFrame> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.recv_rx.recv_timeout(remaining) {
                Ok(frame) => {
                    if self.filter.matches(frame.id) {
                        return Some(frame);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Drain all pending frames that match the filter.
    pub fn drain(&self) -> Vec<CanFrame> {
        let mut frames = Vec::new();
        while let Some(frame) = self.try_recv() {
            frames.push(frame);
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CanId;

    #[test]
    fn bus_send_receive() {
        let bus = VirtualCanBus::new(64);
        let node_a = bus.connect(CanFilter::AcceptAll, 64);
        let node_b = bus.connect(CanFilter::AcceptAll, 64);

        let frame = CanFrame::new(CanId::new(0x100).unwrap(), &[1, 2, 3], 0);
        node_a.send(frame.clone()).unwrap();

        // Give the router thread a moment
        std::thread::sleep(Duration::from_millis(10));

        let received = node_b.recv_timeout(Duration::from_millis(100));
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received.id, CanId::new(0x100).unwrap());
        assert_eq!(received.data.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn bus_filtered_receive() {
        let bus = VirtualCanBus::new(64);
        let sender = bus.connect(CanFilter::AcceptAll, 64);
        let filtered = bus.connect(CanFilter::Exact(CanId::new(0x200).unwrap()), 64);

        sender
            .send(CanFrame::new(CanId::new(0x100).unwrap(), &[1], 0))
            .unwrap();
        sender
            .send(CanFrame::new(CanId::new(0x200).unwrap(), &[2], 0))
            .unwrap();

        std::thread::sleep(Duration::from_millis(10));

        let received = filtered.recv_timeout(Duration::from_millis(100));
        assert!(received.is_some());
        assert_eq!(received.unwrap().data.as_slice(), &[2]);
    }
}
