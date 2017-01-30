extern crate crossbeam;


////////
//////// THIS IS BUSTED - I need to update it to use tsc,
//////// the time function is way too expensive
////////

extern crate multiqueue;
extern crate time;

use multiqueue::{Receiver, Sender, multiqueue};

use crossbeam::scope;

use std::sync::atomic::{AtomicUsize, Ordering, fence};
use std::sync::Barrier;

//prevent any inlining shenanigans
#[inline(never)]
fn precise_time_ns() -> u64 {
    time::precise_time_ns()
}

#[inline(never)]
fn waste_50_ns(val: &AtomicUsize) {
    val.store(0, Ordering::Release);
    fence(Ordering::SeqCst);
}

fn recv(bar: &Barrier, reader: Receiver<Option<u64>>) {
    let mut total_time = 0;
    let mut succ = 0;
    let tries = 10000;
    for _ in 0..tries {
        let start = precise_time_ns();
        let end = precise_time_ns();
        if end >= start {
            succ += 1;
            total_time += end - start;
        }
    }
    let to_subtract = total_time / succ;
    bar.wait();
    let mut v = Vec::with_capacity(100000);
    loop {
        if let Ok(popped) = reader.try_recv() {
            match popped {
                None => break,
                Some(pushed) => {
                    let current_time = precise_time_ns();
                    if current_time >= pushed {
                        let diff = current_time - pushed;
                        if diff > to_subtract {
                            v.push(diff - to_subtract);
                        }
                    }
                }
            }
        }
    }
    for val in v {
        println!("{}", val);
    }
}

fn send(bar: &Barrier, writer: Sender<Option<u64>>, num_push: usize, num_us: usize) {
    bar.wait();
    let val: AtomicUsize = AtomicUsize::new(0);
    for _ in 0..num_push {
        loop {
            let topush = Some(precise_time_ns());
            if let Ok(_) = writer.try_send(topush) {
                break;
            }
        }
        for _ in 0..(num_us * 20) {
            waste_50_ns(&val);
        }
    }
    while let Err(_) = writer.try_send(None) {}
}

fn main() {
    let (writer, reader) = multiqueue(20000);
    let bar = Barrier::new(2);
    let bref = &bar;
    scope(|scope| {
        scope.spawn(move || { send(bref, writer, 100000, 40); });
        recv(bref, reader);
    });
}
