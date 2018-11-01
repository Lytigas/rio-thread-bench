#![feature(test)]

extern crate crossbeam;
use crossbeam::atomic::ArcCell;
use std::sync::{Arc, Weak};

#[derive(Default, Debug, Copy, Clone)]
struct Message {
    x: f64,
    y: f64,
    dummy: [u32; 20],
}

impl Message {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            dummy: [50; 20],
        }
    }
}

#[derive(Debug)]
pub struct Latest<M>
where
    M: Default + Clone,
{
    q: Vec<Arc<M>>,
    latest: Arc<ArcCell<M>>,
    index: usize,
}

impl<M> Latest<M>
where
    M: Default + Clone,
{
    pub fn new(cap: usize) -> Self {
        debug_assert!(cap >= 2);
        let mut q: Vec<Arc<M>> = Vec::with_capacity(cap);
        for _ in 0..cap {
            q.push(Arc::new(Default::default()));
        }

        let latest = Arc::new(ArcCell::new(q.get(0).unwrap().clone()));

        Self {
            q,
            latest,
            index: 1,
        }
    }

    #[inline]
    pub fn reader(&self) -> LatestReader<M> {
        LatestReader(Arc::downgrade(&self.latest))
    }

    #[inline]
    fn next_idx(&self) -> usize {
        (self.index + 1) % self.q.len()
    }

    #[inline]
    pub fn set(&mut self, msg: M) {
        let mut new: &mut Arc<M>;
        loop {
            let idx = self.next_idx();
            new = self
                .q
                .get_mut(idx)
                .expect(format!("Max index reached {}", self.index).as_ref());
            match Arc::get_mut(new) {
                Some(x) => {
                    *x = msg;
                    break;
                }
                _ => {
                    self.index = self.next_idx();
                }
            }
        }
        self.latest.set(new.clone());
    }
}

#[derive(Debug, Clone)]
pub struct LatestReader<M: Default + Clone>(Weak<ArcCell<M>>);

impl<M> LatestReader<M>
where
    M: Default + Clone,
{
    #[inline]
    pub fn get(&self) -> Option<M> {
        match self.0.upgrade() {
            Some(arc) => Some((*arc.get()).clone()),
            _ => None,
        }
    }
}

extern crate test;
use self::test::Bencher;
use std::thread;
use std::time::Duration;

#[bench]
fn latest_reads(b: &mut Bencher) {
    let mut latest = Latest::new(2);
    let reader = latest.reader();
    let handle = thread::spawn(move || {
        for i in 0..20000 {
            latest.set(Message::new(i as f64, -i as f64));
            thread::sleep(Duration::from_nanos(5));
        }
    });
    b.iter(|| test::black_box(reader.get().expect("q closed")));
    handle.join().unwrap();
}

#[bench]
fn latest_writes(b: &mut Bencher) {
    let mut latest = Latest::new(2);
    test::black_box(&latest);
    b.iter(|| latest.set(Message::new(1 as f64, -1 as f64)));
}

use std::sync::Mutex;

#[bench]
fn mutex_reads(b: &mut Bencher) {
    let mutex = Arc::new(Mutex::new(Message::new(0., 0.)));
    let mutex1 = mutex.clone();
    let mutex2 = mutex.clone();
    let mutex3 = mutex.clone();

    let handle1 = thread::spawn(move || {
        for i in 0..200000 {
            {
                *mutex.lock().unwrap() = Message::new(i as f64, -i as f64);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex1.lock().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex2.lock().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    b.iter(|| test::black_box(mutex3.lock().unwrap().y));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

#[bench]
fn mutex_writes(b: &mut Bencher) {
    let mutex = Arc::new(Mutex::new(Message::new(0., 0.)));
    let mutex1 = mutex.clone();
    let mutex2 = mutex.clone();
    let mutex3 = mutex.clone();

    let handle1 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex1.lock().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex2.lock().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex3.lock().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    b.iter(|| test::black_box(*mutex.lock().unwrap() = Message::new(1 as f64, -1 as f64)));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

use std::sync::RwLock;
#[bench]
fn rwlock_reads(b: &mut Bencher) {
    let mutex = Arc::new(RwLock::new(Message::new(0., 0.)));
    let mutex1 = mutex.clone();
    let mutex2 = mutex.clone();
    let mutex3 = mutex.clone();

    let handle1 = thread::spawn(move || {
        for i in 0..200000 {
            {
                *mutex.write().unwrap() = Message::new(i as f64, -i as f64);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex1.read().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex2.read().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    b.iter(|| test::black_box(mutex3.read().unwrap().y));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

#[bench]
fn rwlock_writes(b: &mut Bencher) {
    let mutex = Arc::new(RwLock::new(Message::new(0., 0.)));
    let mutex1 = mutex.clone();
    let mutex2 = mutex.clone();
    let mutex3 = mutex.clone();

    let handle1 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex1.read().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex2.read().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..200000 {
            {
                test::black_box(mutex3.read().unwrap().y);
                thread::sleep(Duration::from_nanos(5));
            }
        }
    });
    b.iter(|| test::black_box(*mutex.write().unwrap() = Message::new(1 as f64, -1 as f64)));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

extern crate bus;
use bus::{Bus, BusReader};
#[bench]
fn bus_reads(b: &mut Bencher) {
    let mut bus = Bus::new(200);
    let mut reader = bus.add_rx();
    let mut reader1 = bus.add_rx();
    let mut reader2 = bus.add_rx();
    let handle1 = thread::spawn(move || {
        for i in 0..20000 {
            bus.try_broadcast(Message::new(i as f64, -i as f64)).ok();
            thread::sleep(Duration::from_nanos(5));
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..20000 {
            test::black_box(reader1.try_recv().ok());
            thread::sleep(Duration::from_nanos(5));
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..20000 {
            test::black_box(reader2.try_recv().ok());
            thread::sleep(Duration::from_nanos(5));
        }
    });
    b.iter(|| test::black_box(reader.try_recv()));
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

#[bench]
fn bus_writes(b: &mut Bencher) {
    let mut bus = Bus::new(200);
    let mut reader1 = bus.add_rx();
    let mut reader2 = bus.add_rx();
    let mut reader3 = bus.add_rx();
    let handle1 = thread::spawn(move || {
        for _ in 0..20000 {
            test::black_box(reader1.try_recv().ok());
            thread::sleep(Duration::from_nanos(5));
        }
    });
    let handle2 = thread::spawn(move || {
        for _ in 0..20000 {
            test::black_box(reader2.try_recv().ok());
            thread::sleep(Duration::from_nanos(5));
        }
    });
    let handle3 = thread::spawn(move || {
        for _ in 0..20000 {
            test::black_box(reader3.try_recv().ok());
            thread::sleep(Duration::from_nanos(5));
        }
    });
    b.iter(|| bus.try_broadcast(Message::new(1 as f64, -1 as f64)).ok());
    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

fn main() {}
