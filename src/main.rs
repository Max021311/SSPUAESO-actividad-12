
use std::sync::{Arc, Mutex, MutexGuard, TryLockResult, TryLockError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[cfg(not(feature = "no-timeouts"))]
const TIMEOUT: u64 = 750;

pub trait TimeoutLock<T> {
    /// Try to acquire the lock for the mutex until a timeout.
    /// ```rust
    /// use std::sync::{Mutex, TryLockError};
    /// use std::time::Duration;
    ///
    /// let lock = Mutex::new(String::from("Hello world"));
    /// match lock.try_lock_for(Duration::from_millis(10)) {
    ///     Ok(guard) => { println!("The value behind the mutex is: {}", *guard) }
    ///     Err(TryLockError::Poisoned(guard)) => { eprintln!("The value behind the mutex is
    ///     poisoned") },
    ///     Err(TryLockError::WouldBlock) => { eprintln!("The lock of the mutex could not be acquired") }
    /// };
    /// ```
    fn try_lock_for(&self, timeout: Duration, interval: Duration) -> TryLockResult<MutexGuard<'_, T>>;
}

impl<T> TimeoutLock<T> for Mutex<T> {
    fn try_lock_for(&self, timeout: Duration, interval: Duration) -> TryLockResult<MutexGuard<'_, T>> {
        let start = Instant::now();
        while Instant::now() - start < timeout {
            match self.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(TryLockError::Poisoned(guard)) => return Err(TryLockError::Poisoned(guard)),
                Err(TryLockError::WouldBlock) => {}
            }
            thread::sleep(interval);
        }
        Err(TryLockError::WouldBlock)
    }
}

struct Philosopher<T> {
    name: String,
    left_fork: Arc<Mutex<T>>,
    right_fork: Arc<Mutex<T>>,
    is_left_handed: bool,
    counter: u32,
}

impl<T> Philosopher<T> {
    fn new(name: &str, left_fork: Arc<Mutex<T>>, right_fork: Arc<Mutex<T>>, is_left_handed: bool) -> Self {
        Self {
            name: name.to_string(),
            left_fork,
            right_fork,
            is_left_handed,
            counter: 0
        }
    }

    /// Try to dine waiting for the forks for 100 milliseconds trying every 10 milliseconds get the
    /// forks ([See][TimeoutLock::try_lock_for]). If the philosopher is left handed begin with the left fork instead of the right fork
    /// 
    /// If the philosopher could not get at least one fork then the other fork is released.
    ///
    fn dine(&mut self) {
        println!("{} está pensando.", self.name);
        #[cfg(not(feature = "no-timeouts"))]
        thread::sleep(Duration::from_millis(TIMEOUT));

        let locks = if self.is_left_handed {
            let left = self.left_fork.lock();
            let right = self.right_fork.try_lock_for(
                Duration::from_millis(100), 
                Duration::from_millis(10)
            );
            (left, right)
        } else {
            let left = self.right_fork.lock();
            let right = self.left_fork.try_lock_for(
                Duration::from_millis(100), 
                Duration::from_millis(10)
            );
            (left, right)
        };


        if let (Ok(_), Ok(_)) = locks {
            println!("{} está comiendo.", self.name);
            #[cfg(not(feature = "no-timeouts"))]
            thread::sleep(Duration::from_millis(TIMEOUT));

            self.counter += 1;
            println!("{} comidas: {}.", self.name, self.counter);
            #[cfg(not(feature = "no-timeouts"))]
            thread::sleep(Duration::from_millis(TIMEOUT));

            println!("{} terminó de comer y está pensando nuevamente.", self.name);
        } else {
            println!("{} no puede comer ya que no pudo tomar ambos tenedores.", self.name);
        }
        #[cfg(not(feature = "no-timeouts"))]
        thread::sleep(Duration::from_millis(TIMEOUT));
    }
}

fn main() {
    // Create the forks as empty tuples
    let forks = [
        Arc::new(Mutex::new(())),
        Arc::new(Mutex::new(())),
        Arc::new(Mutex::new(())),
        Arc::new(Mutex::new(())),
        Arc::new(Mutex::new(())),
    ];

    let philosophers = [
        Philosopher::new("Filósofo 1", forks[0].clone(), forks[1].clone(), false),
        Philosopher::new("Filósofo 2", forks[1].clone(), forks[2].clone(), false),
        Philosopher::new("Filósofo 3", forks[2].clone(), forks[3].clone(), false),
        Philosopher::new("Filósofo 4", forks[3].clone(), forks[4].clone(), false),
        // Este filósofo es zurdo
        Philosopher::new("Filósofo 5", forks[4].clone(), forks[0].clone(), true),
    ];

    let now = Instant::now();
    // Spawn a thread for each philosopher
    let handles: Vec<(String, JoinHandle<()>)> = philosophers
        .into_iter()
        .map(|mut philosopher| {
            let name = philosopher.name.clone();
            let handle = thread::spawn(move || {
                loop {
                    if philosopher.counter == 6 {
                        break
                    }
                    philosopher.dine();
                }
            });
            return (name, handle)
        })
        .collect();

    // Wait for each thread to end
    for (name, handle) in handles {
        if let Err(_) = handle.join() {
            eprintln!("{name} ha tenido un error al comer");
        }
    }
    println!("Los filósofos han terminado de comer.");
    println!("Tiempo transcurrido: {:.2?}", now.elapsed())
}
