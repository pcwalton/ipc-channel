// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use libc;
use platform::{self, OsIpcChannel, OsIpcReceiverSet, OsIpcSender, OsIpcOneShotServer};
use platform::OsIpcSharedMemory;
use std::iter;
use std::thread;

#[test]
fn simple() {
    let (tx, rx) = platform::channel().unwrap();
    let data: &[u8] = b"1234567";
    tx.send(data, Vec::new(), Vec::new()).unwrap();
    let mut received_values = rx.recv().unwrap();
    received_values.data.truncate(7);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, Vec::new(), Vec::new()));
}

#[test]
fn sender_transfer() {
    let (super_tx, super_rx) = platform::channel().unwrap();
    let (sub_tx, sub_rx) = platform::channel().unwrap();
    let data: &[u8] = b"foo";
    super_tx.send(data, vec![OsIpcChannel::Sender(sub_tx)], vec![]).unwrap();
    let mut received_value = super_rx.recv().unwrap();
    assert_eq!(received_value.channels.len(), 1);
    let sub_tx = received_value.channels.pop().unwrap().to_sender();
    sub_tx.send(data, vec![], vec![]).unwrap();
    let mut received_values = sub_rx.recv().unwrap();
    received_values.data.truncate(3);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[test]
fn receiver_transfer() {
    let (super_tx, super_rx) = platform::channel().unwrap();
    let (sub_tx, sub_rx) = platform::channel().unwrap();
    let data: &[u8] = b"foo";
    super_tx.send(data, vec![OsIpcChannel::Receiver(sub_rx)], vec![]).unwrap();
    let mut received_values = super_rx.recv().unwrap();
    assert_eq!(received_values.channels.len(), 1);
    let sub_rx = received_values.channels.pop().unwrap().to_receiver();
    sub_tx.send(data, vec![], vec![]).unwrap();
    received_values = sub_rx.recv().unwrap();
    received_values.data.truncate(3);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[test]
fn multisender_transfer() {
    let (super_tx, super_rx) = platform::channel().unwrap();
    let (sub0_tx, sub0_rx) = platform::channel().unwrap();
    let (sub1_tx, sub1_rx) = platform::channel().unwrap();
    let data: &[u8] = b"asdfasdf";
    super_tx.send(data,
                  vec![OsIpcChannel::Sender(sub0_tx), OsIpcChannel::Sender(sub1_tx)],
                  vec![])
            .unwrap();
    let mut received_value = super_rx.recv().unwrap();
    assert_eq!(received_value.channels.len(), 2);

    let sub0_tx = received_value.channels.remove(0).to_sender();
    sub0_tx.send(data, vec![], vec![]).unwrap();
    let mut received_values = sub0_rx.recv().unwrap();
    received_values.data.truncate(8);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));

    let sub1_tx = received_value.channels.remove(0).to_sender();
    sub1_tx.send(data, vec![], vec![]).unwrap();
    received_values = sub1_rx.recv().unwrap();
    received_values.data.truncate(8);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[test]
fn medium_data() {
    let data: Vec<u8> = iter::repeat(0xba).take(65536).collect();
    let data: &[u8] = &data[..];
    let (tx, rx) = platform::channel().unwrap();
    tx.send(data, vec![], vec![]).unwrap();
    let mut received_values = rx.recv().unwrap();
    received_values.data.truncate(65536);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (&data[..], vec![], vec![]));
}

#[test]
fn medium_data_with_sender_transfer() {
    let data: Vec<u8> = iter::repeat(0xba).take(65536).collect();
    let data: &[u8] = &data[..];
    let (super_tx, super_rx) = platform::channel().unwrap();
    let (sub_tx, sub_rx) = platform::channel().unwrap();
    super_tx.send(data, vec![OsIpcChannel::Sender(sub_tx)], vec![]).unwrap();
    let mut received_values = super_rx.recv().unwrap();
    assert_eq!(received_values.channels.len(), 1);
    let sub_tx = received_values.channels.pop().unwrap().to_sender();
    sub_tx.send(data, vec![], vec![]).unwrap();
    let mut received_values = sub_rx.recv().unwrap();
    received_values.data.truncate(65536);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[test]
fn big_data() {
    let (tx, rx) = platform::channel().unwrap();
    let thread = thread::spawn(move || {
        let data: Vec<u8> = iter::repeat(0xba).take(1024 * 1024).collect();
        let data: &[u8] = &data[..];
        tx.send(data, vec![], vec![]).unwrap();
    });
    let mut received_values = rx.recv().unwrap();
    let data: Vec<u8> = iter::repeat(0xba).take(1024 * 1024).collect();
    let data: &[u8] = &data[..];
    received_values.data.truncate(1024 * 1024);
    assert_eq!(received_values.data.len(), data.len());
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (&data[..], vec![], vec![]));
    thread.join().unwrap();
}

#[test]
fn big_data_with_sender_transfer() {
    let (super_tx, super_rx) = platform::channel().unwrap();
    let (sub_tx, sub_rx) = platform::channel().unwrap();
    let thread = thread::spawn(move || {
        let data: Vec<u8> = iter::repeat(0xba).take(1024 * 1024).collect();
        let data: &[u8] = &data[..];
        super_tx.send(data, vec![OsIpcChannel::Sender(sub_tx)], vec![]).unwrap();
    });
    let mut received_values = super_rx.recv().unwrap();
    let data: Vec<u8> = iter::repeat(0xba).take(1024 * 1024).collect();
    let data: &[u8] = &data[..];
    received_values.data.truncate(1024 * 1024);
    assert_eq!(received_values.data.len(), data.len());
    assert_eq!(&received_values.data[..], &data[..]);
    assert_eq!(received_values.channels.len(), 1);
    assert_eq!(received_values.shared_memory.len(), 0);

    let data: Vec<u8> = iter::repeat(0xba).take(65536).collect();
    let data: &[u8] = &data[..];
    let sub_tx = received_values.channels[0].to_sender();
    sub_tx.send(data, vec![], vec![]).unwrap();
    let mut received_values = sub_rx.recv().unwrap();
    received_values.data.truncate(1024 * 1024);
    assert_eq!(received_values.data.len(), data.len());
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (&data[..], vec![], vec![]));
    thread.join().unwrap();
}

#[test]
fn receiver_set() {
    let (tx0, rx0) = platform::channel().unwrap();
    let (tx1, rx1) = platform::channel().unwrap();
    let mut rx_set = OsIpcReceiverSet::new().unwrap();
    let rx0_id = rx_set.add(rx0).unwrap();
    let rx1_id = rx_set.add(rx1).unwrap();

    let data: &[u8] = b"1234567";
    tx0.send(data, vec![], vec![]).unwrap();
    let (received_id, mut received_data, _, _) = rx_set.select()
                                                       .unwrap()
                                                       .into_iter()
                                                       .next()
                                                       .unwrap()
                                                       .unwrap();
    received_data.truncate(7);
    assert_eq!(received_id, rx0_id);
    assert_eq!(received_data, data);

    tx1.send(data, vec![], vec![]).unwrap();
    let (received_id, mut received_data, _, _) = rx_set.select()
                                                       .unwrap()
                                                       .into_iter()
                                                       .next()
                                                       .unwrap()
                                                       .unwrap();
    received_data.truncate(7);
    assert_eq!(received_id, rx1_id);
    assert_eq!(received_data, data);

    tx0.send(data, vec![], vec![]).unwrap();
    tx1.send(data, vec![], vec![]).unwrap();
    let (mut received0, mut received1) = (false, false);
    while !received0 || !received1 {
        for result in rx_set.select().unwrap().into_iter() {
            let (received_id, mut received_data, _, _) = result.unwrap();
            received_data.truncate(7);
            assert_eq!(received_data, data);
            assert!(received_id == rx0_id || received_id == rx1_id);
            if received_id == rx0_id {
                assert!(!received0);
                received0 = true;
            } else if received_id == rx1_id {
                assert!(!received1);
                received1 = true;
            }
        }
    }
}

#[test]
fn server() {
    let (server, name) = OsIpcOneShotServer::new().unwrap();
    let data: &[u8] = b"1234567";

    thread::spawn(move || {
        let tx = OsIpcSender::connect(name).unwrap();
        tx.send(data, vec![], vec![]).unwrap();
    });

    let (_, mut received_values) = server.accept().unwrap();
    received_values.data.truncate(7);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[cfg(not(windows))]
#[test]
fn cross_process() {
    let (server, name) = OsIpcOneShotServer::new().unwrap();
    let data: &[u8] = b"1234567";

    unsafe {
        if libc::fork() == 0 {
            let tx = OsIpcSender::connect(name).unwrap();
            tx.send(data, vec![], vec![]).unwrap();
            libc::exit(0);
        }
    }

    let (_, mut received_values) = server.accept().unwrap();
    received_values.data.truncate(7);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[cfg(not(windows))]
#[test]
fn cross_process_sender_transfer() {
    let (server, name) = OsIpcOneShotServer::new().unwrap();

    unsafe {
        if libc::fork() == 0 {
            let super_tx = OsIpcSender::connect(name).unwrap();
            let (sub_tx, sub_rx) = platform::channel().unwrap();
            let data: &[u8] = b"foo";
            super_tx.send(data, vec![OsIpcChannel::Sender(sub_tx)], vec![]).unwrap();
            sub_rx.recv().unwrap();
            let data: &[u8] = b"bar";
            super_tx.send(data, vec![], vec![]).unwrap();
            libc::exit(0);
        }
    }

    let (super_rx, mut received_values) = server.accept().unwrap();
    assert_eq!(received_values.channels.len(), 1);
    let sub_tx = received_values.channels.pop().unwrap().to_sender();
    let data: &[u8] = b"baz";
    sub_tx.send(data, vec![], vec![]).unwrap();

    let data: &[u8] = b"bar";
    let mut received_values = super_rx.recv().unwrap();
    received_values.data.truncate(3);
    assert_eq!((&received_values.data[..],
                received_values.channels,
                received_values.shared_memory),
               (data, vec![], vec![]));
}

#[test]
fn no_senders_notification() {
    let (sender, receiver) = platform::channel().unwrap();
    drop(sender);
    let result = receiver.recv();
    assert!(result.is_err());
    assert!(result.unwrap_err().channel_is_closed());
}

#[test]
fn shared_memory() {
    let (tx, rx) = platform::channel().unwrap();
    let data: &[u8] = b"1234567";
    let shmem_data = OsIpcSharedMemory::from_byte(0xba, 1024 * 1024);
    tx.send(data, vec![], vec![shmem_data]).unwrap();
    let mut received_data = rx.recv().unwrap();
    received_data.data.truncate(7);
    assert_eq!((&received_data.data[..], received_data.channels),
               (data, Vec::new()));
    assert_eq!(received_data.shared_memory[0].len(), 1024 * 1024);
    assert!(received_data.shared_memory[0].iter().all(|byte| *byte == 0xba));
}

#[test]
fn shared_memory_clone() {
    let shmem_data_0 = OsIpcSharedMemory::from_byte(0xba, 1024 * 1024);
    let shmem_data_1 = shmem_data_0.clone();
    assert_eq!(&shmem_data_0[..], &shmem_data_1[..]);
}

#[test]
fn try_recv() {
    let (tx, rx) = platform::channel().unwrap();
    assert!(rx.try_recv().is_err());
    let data: &[u8] = b"1234567";
    tx.send(data, Vec::new(), Vec::new()).unwrap();
    let mut received_data = rx.try_recv().unwrap();
    received_data.data.truncate(7);
    assert_eq!((&received_data.data[..],
                received_data.channels,
                received_data.shared_memory),
               (data, Vec::new(), Vec::new()));
    assert!(rx.try_recv().is_err());
}
