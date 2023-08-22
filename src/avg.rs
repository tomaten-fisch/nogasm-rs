pub struct Queue<T, const N: usize> {
    arr: [T; N],
    pub num: usize,
    index: usize,
}

impl<T: Copy, const N: usize> Queue<T, { N }> {
    pub fn new(default: T) -> Queue<T, N> {
        Queue {
            arr: [default; N],
            num: 0,
            index: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        self.arr[self.index] = value;
        if self.num < N {
            self.num += 1;
        }
        self.index = (self.index + 1) % N;
    }

    pub fn peek(&mut self) -> T {
        self.arr[self.index]
    }
}

pub struct RunningAverage<const N: usize> {
    queue: Queue<u32, N>,
    sum: u32,
}

impl<const N: usize> RunningAverage<{ N }> {
    pub fn new() -> RunningAverage<N> {
        RunningAverage {
            queue: Queue::new(0),
            sum: 0,
        }
    }

    pub fn get(&self) -> u32 {
        match self.queue.num {
            0 => 0,
            n => self.sum / n as u32,
        }
    }

    pub fn add(&mut self, val: u32) {
        self.sum -= self.queue.peek() as u32;
        self.queue.push(val);
        self.sum += val as u32;
    }
}
