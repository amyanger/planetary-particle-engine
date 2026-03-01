use ppe_can::{CanFilter, CanFrame, CanId, VirtualCanBus};
use std::time::Duration;

#[test]
fn two_nodes_send_and_receive_through_filter() {
    let bus = VirtualCanBus::new(64);

    // Node A accepts all frames
    let node_a = bus.connect(CanFilter::AcceptAll, 64);
    // Node B only accepts BMS frames (0x100-0x10F)
    let node_b = bus.connect(
        CanFilter::Range {
            low: CanId::new(0x100).unwrap(),
            high: CanId::new(0x10F).unwrap(),
        },
        64,
    );

    // Send a BMS frame from A
    let bms_frame = CanFrame::new(CanId::new(0x100).unwrap(), &[0x64, 0x00], 1000);
    node_a.send(bms_frame).unwrap();

    // Send a motor frame from A (should NOT be received by B)
    let motor_frame = CanFrame::new(CanId::new(0x200).unwrap(), &[0xFF], 2000);
    node_a.send(motor_frame).unwrap();

    // Give the router thread time to process
    std::thread::sleep(Duration::from_millis(50));

    // Node B should receive the BMS frame
    let received = node_b.recv_timeout(Duration::from_millis(100));
    assert!(received.is_some(), "Node B should receive BMS frame");
    let received = received.unwrap();
    assert_eq!(received.id, CanId::new(0x100).unwrap());
    assert_eq!(received.data.as_slice(), &[0x64, 0x00]);

    // Node B should NOT receive the motor frame
    let received = node_b.recv_timeout(Duration::from_millis(100));
    assert!(received.is_none(), "Node B should NOT receive motor frame");
}

#[test]
fn multiple_subscribers_receive_same_frame() {
    let bus = VirtualCanBus::new(64);

    let node_a = bus.connect(CanFilter::AcceptAll, 64);
    let node_b = bus.connect(CanFilter::AcceptAll, 64);
    let node_c = bus.connect(CanFilter::AcceptAll, 64);

    let frame = CanFrame::new(CanId::new(0x001).unwrap(), &[0xDE, 0xAD], 0);
    node_a.send(frame).unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Both B and C should receive the frame
    assert!(node_b.recv_timeout(Duration::from_millis(100)).is_some());
    assert!(node_c.recv_timeout(Duration::from_millis(100)).is_some());
}
